use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use waggledance_core::Engine;

use crate::paseo::Paseo;

pub const EVERY: Duration = Duration::from_secs(300);

#[derive(Debug, Clone)]
pub struct Bounds {
    pub home: Option<PathBuf>,
    pub tmp: PathBuf,
}

impl Bounds {
    pub fn host() -> Self {
        Self {
            home: dirs::home_dir().map(|h| realpath(&h)),
            tmp: realpath(Path::new("/tmp")),
        }
    }
}

fn realpath(path: &Path) -> PathBuf {
    std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn eligible(root: &Path, bounds: &Bounds) -> bool {
    root.join(".git").is_dir()
        && root != Path::new("/")
        && Some(root) != bounds.home.as_deref()
        && !root.starts_with(&bounds.tmp)
}

pub async fn discover(paseo: &dyn Paseo, engine: &Arc<Engine>, bounds: &Bounds) -> Vec<PathBuf> {
    let workspaces = match paseo.workspaces().await {
        Ok(w) => w,
        Err(e) => {
            tracing::warn!("Paseo discovery could not list workspaces: {e}");
            return Vec::new();
        }
    };
    let mut known: Vec<PathBuf> = match engine.list_projects() {
        Ok(projects) => projects.iter().map(|p| realpath(&p.root_path)).collect(),
        Err(e) => {
            tracing::warn!("Paseo discovery could not list projects: {e}");
            return Vec::new();
        }
    };
    let store = paseo.stored_workspaces();
    let mut registered = Vec::new();
    for workspace in workspaces {
        let retired = store.iter().any(|s| {
            s.workspace_id == workspace.id && (s.archived_at.is_some() || s.is_paseo_owned_worktree)
        });
        if workspace.isolation != "local" || retired {
            continue;
        }
        let Ok(root) = std::fs::canonicalize(&workspace.cwd) else {
            continue;
        };
        if !eligible(&root, bounds)
            || known
                .iter()
                .any(|k| k.starts_with(&root) || root.starts_with(k))
        {
            continue;
        }
        known.push(root.clone());
        let (engine, path) = (engine.clone(), root.clone());
        match tokio::task::spawn_blocking(move || engine.register(&path, None)).await {
            Ok(Ok(project)) => {
                tracing::info!(
                    project = %project.id,
                    workspace = %workspace.id,
                    "Paseo discovery registered {}",
                    root.display()
                );
                registered.push(root);
            }
            Ok(Err(e)) => {
                tracing::warn!("Paseo discovery could not register {}: {e}", root.display())
            }
            Err(e) => tracing::warn!("Paseo discovery register task failed: {e}"),
        }
    }
    registered
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::paseo::fake::FakePaseo;
    use crate::paseo::read_store;
    use waggledance_core::{Config, SqliteStore};

    struct Rig {
        fake: FakePaseo,
        engine: Arc<Engine>,
        bounds: Bounds,
        dir: tempfile::TempDir,
    }

    fn rig() -> Rig {
        let dir = tempfile::tempdir().unwrap();
        Rig {
            fake: FakePaseo::default(),
            engine: Arc::new(Engine::new(
                SqliteStore::open_in_memory().unwrap(),
                Config::default(),
            )),
            bounds: Bounds {
                home: None,
                tmp: PathBuf::from("/nonexistent-tmp"),
            },
            dir,
        }
    }

    impl Rig {
        fn repo(&self, rel: &str) -> PathBuf {
            let root = realpath(self.dir.path()).join(rel);
            std::fs::create_dir_all(root.join(".git")).unwrap();
            std::fs::write(root.join("README.md"), "# hi\n").unwrap();
            root
        }

        fn workspace(&self, id: &str, isolation: &str, root: &Path) {
            self.fake.add_workspace(id, isolation, root);
        }

        async fn discover(&self) -> Vec<PathBuf> {
            discover(&self.fake, &self.engine, &self.bounds).await
        }

        fn roots(&self) -> Vec<PathBuf> {
            let mut roots: Vec<PathBuf> = self
                .engine
                .list_projects()
                .unwrap()
                .into_iter()
                .map(|p| p.root_path)
                .collect();
            roots.sort();
            roots
        }
    }

    #[tokio::test]
    async fn discovery_registers_a_new_local_repo_root_once_without_opting_it_in() {
        let r = rig();
        let root = r.repo("repo");
        r.workspace("wks_1", "local", &root);
        r.workspace("wks_2", "local", &root.join("."));
        assert_eq!(r.discover().await, vec![root.clone()]);
        assert!(r.discover().await.is_empty());
        assert_eq!(r.roots(), vec![root]);
        let project = r.engine.list_projects().unwrap().remove(0);
        assert!(!project.orchestration_enabled);
    }

    #[tokio::test]
    async fn discovery_skips_worktrees_archived_owned_home_tmp_missing_and_non_git_roots() {
        let mut r = rig();
        r.fake.state.lock().unwrap().store = read_store(
            &Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("testdata/fixtures/paseo-0.11.2/workspaces-store.json"),
        );
        let worktree = realpath(r.dir.path()).join("wt");
        std::fs::create_dir_all(&worktree).unwrap();
        std::fs::write(worktree.join(".git"), "gitdir: /elsewhere\n").unwrap();
        r.workspace("wks_wt", "local", &worktree);
        let plain = realpath(r.dir.path()).join("plain");
        std::fs::create_dir_all(&plain).unwrap();
        r.workspace("wks_plain", "local", &plain);
        r.workspace("wks_missing", "local", &r.dir.path().join("gone"));
        r.workspace("wks_isolated", "worktree", &r.repo("isolated"));
        r.workspace("wks_fixture_archived", "local", &r.repo("archived"));
        r.workspace("wks_fixture_owned", "local", &r.repo("owned"));
        let home = r.repo("home");
        r.workspace("wks_home", "local", &home);
        let tmp = r.repo("tmp/inner");
        r.workspace("wks_tmp", "local", &tmp);
        r.bounds = Bounds {
            home: Some(home),
            tmp: tmp.parent().unwrap().to_path_buf(),
        };
        assert!(r.discover().await.is_empty());
        assert!(r.roots().is_empty());
    }

    #[tokio::test]
    async fn discovery_skips_roots_nested_in_or_containing_a_registered_project() {
        let r = rig();
        let outer = r.repo("outer");
        let inner = r.repo("outer/inner");
        let parent = r.repo("parent");
        let child = r.repo("parent/child");
        r.engine.register(&outer, None).unwrap();
        r.engine.register(&child, None).unwrap();
        r.workspace("wks_inner", "local", &inner);
        r.workspace("wks_parent", "local", &parent);
        assert!(r.discover().await.is_empty());
        assert_eq!(r.roots(), vec![outer, child]);
    }

    #[test]
    fn the_host_bounds_name_home_and_tmp() {
        let b = Bounds::host();
        assert_eq!(b.tmp, realpath(Path::new("/tmp")));
        assert!(!eligible(Path::new("/"), &b));
    }
}
