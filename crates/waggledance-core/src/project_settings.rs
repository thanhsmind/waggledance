use serde::Deserialize;
use std::path::{Path, PathBuf};

pub const FILE_NAME: &str = ".waggledance.toml";

const TOP_KEYS: [&str; 2] = ["orchestration", "paseo"];
const PASEO_KEYS: [&str; 4] = ["workspace", "provider", "mode", "thinking"];

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ProjectSettings {
    pub orchestration: Option<bool>,
    pub paseo: ProjectPaseo,
    #[serde(skip)]
    pub unreadable: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Deserialize)]
#[serde(default)]
pub struct ProjectPaseo {
    pub workspace: Option<String>,
    pub provider: Option<String>,
    pub mode: Option<String>,
    pub thinking: Option<String>,
}

pub fn settings_path(root: &Path) -> PathBuf {
    root.join(FILE_NAME)
}

pub fn load(root: &Path) -> (ProjectSettings, Vec<String>) {
    let path = settings_path(root);
    match std::fs::read_to_string(&path) {
        Ok(text) => parse(&text, &path),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => (ProjectSettings::default(), vec![]),
        Err(e) => unreadable(format!("{} could not be read: {e}", path.display())),
    }
}

pub fn parse(text: &str, path: &Path) -> (ProjectSettings, Vec<String>) {
    let table: toml::Table = match toml::from_str(text) {
        Ok(t) => t,
        Err(e) => return invalid(path, e),
    };
    let mut warnings = unknown_keys(&table, &TOP_KEYS, "", path);
    if let Some(toml::Value::Table(paseo)) = table.get("paseo") {
        warnings.extend(unknown_keys(paseo, &PASEO_KEYS, "paseo.", path));
    }
    match ProjectSettings::deserialize(toml::Value::Table(table)) {
        Ok(settings) => (settings, warnings),
        Err(e) => invalid(path, e),
    }
}

fn invalid(path: &Path, e: impl std::fmt::Display) -> (ProjectSettings, Vec<String>) {
    unreadable(format!(
        "{} failed to parse: {}",
        path.display(),
        e.to_string().trim()
    ))
}

fn unreadable(warning: String) -> (ProjectSettings, Vec<String>) {
    (
        ProjectSettings {
            unreadable: Some(warning.clone()),
            ..ProjectSettings::default()
        },
        vec![warning],
    )
}

fn unknown_keys(table: &toml::Table, known: &[&str], prefix: &str, path: &Path) -> Vec<String> {
    table
        .keys()
        .filter(|k| !known.contains(&k.as_str()))
        .map(|k| format!("{}: unknown key `{prefix}{k}` ignored", path.display()))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!(
            "waggledance-project-settings-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).unwrap();
        d
    }

    #[test]
    fn project_settings_missing_file_is_all_none_without_warnings() {
        let d = dir("missing");
        assert_eq!(load(&d), (ProjectSettings::default(), vec![]));
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn project_settings_reads_orchestration_true_and_false() {
        let p = Path::new("/r/.waggledance.toml");
        assert_eq!(parse("orchestration = true", p).0.orchestration, Some(true));
        assert_eq!(
            parse("orchestration = false", p).0.orchestration,
            Some(false)
        );
    }

    #[test]
    fn project_settings_reads_the_paseo_table() {
        let d = dir("paseo");
        std::fs::write(
            settings_path(&d),
            "orchestration = true\n[paseo]\nworkspace = \"wks_1\"\nprovider = \"claude\"\nmode = \"bypassPermissions\"\nthinking = \"high\"\n",
        )
        .unwrap();
        let (s, warnings) = load(&d);
        assert!(warnings.is_empty(), "{warnings:?}");
        assert_eq!(
            s.paseo,
            ProjectPaseo {
                workspace: Some("wks_1".into()),
                provider: Some("claude".into()),
                mode: Some("bypassPermissions".into()),
                thinking: Some("high".into()),
            }
        );
        std::fs::remove_dir_all(&d).ok();
    }

    #[test]
    fn project_settings_invalid_toml_is_off_with_a_warning() {
        let p = Path::new("/r/.waggledance.toml");
        let (s, warnings) = parse("orchestration = tru", p);
        assert_eq!(s.orchestration, None);
        assert_eq!(s.unreadable.as_ref(), warnings.first());
        assert_eq!(warnings.len(), 1);
        assert!(
            warnings[0].contains("/r/.waggledance.toml failed to parse"),
            "{warnings:?}"
        );

        let (s, warnings) = parse("orchestration = \"yes\"", p);
        assert_eq!(s.orchestration, None);
        assert!(warnings[0].contains("failed to parse"), "{warnings:?}");
    }

    #[test]
    fn project_settings_unknown_keys_warn_but_keep_the_rest() {
        let p = Path::new("/r/.waggledance.toml");
        let (s, warnings) = parse(
            "orchestration = true\ncolour = 1\n[paseo]\nmodel = \"x\"\n",
            p,
        );
        assert_eq!(s.orchestration, Some(true));
        assert_eq!(warnings.len(), 2, "{warnings:?}");
        assert!(
            warnings.iter().any(|w| w.contains("`colour`")),
            "{warnings:?}"
        );
        assert!(
            warnings.iter().any(|w| w.contains("`paseo.model`")),
            "{warnings:?}"
        );
    }
}
