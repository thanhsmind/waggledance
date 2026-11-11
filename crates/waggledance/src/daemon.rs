use crate::runtime::{self, DaemonInfo};
use anyhow::Result;
use std::path::PathBuf;
use std::sync::Arc;
use waggledance_core::engine::now_rfc3339;

pub(crate) fn open_notify_store(
    cfg: &waggledance_core::config::TerminalConfig,
    override_dir: Option<&std::path::Path>,
) -> waggledance_core::notify_store::NotifyStore {
    if !cfg.notify_enabled {
        return waggledance_core::notify_store::NotifyStore::open_in_memory()
            .expect("in-memory sqlite always opens");
    }
    let notify_store_path = waggledance_core::config::notify_store_path_override(override_dir);
    waggledance_core::notify_store::NotifyStore::open(&notify_store_path).unwrap_or_else(|e| {
        tracing::warn!("notify outbox open failed ({e}); using an in-memory outbox");
        waggledance_core::notify_store::NotifyStore::open_in_memory()
            .expect("in-memory sqlite always opens")
    })
}

pub(crate) fn telegram_credentials(
    cfg: &waggledance_core::config::TerminalConfig,
    config_data_dir: Option<&std::path::Path>,
) -> Option<(String, String)> {
    let cred_path = waggledance_core::config::notify_credential_path_override(config_data_dir);
    let token = waggledance_core::config::load_notify_credential(&cred_path)?;
    let chat_id = cfg.notify_chat_id.clone()?;
    if token.trim().is_empty() || chat_id.trim().is_empty() {
        return None;
    }
    Some((token, chat_id))
}

pub async fn serve() -> Result<()> {
    let engine = Arc::new(runtime::build_engine()?);
    let herdr_socket_path = crate::herdr::socket::default_socket_path()
        .unwrap_or_else(|_| PathBuf::from("/nonexistent/herdr.sock"));
    let herdr: Arc<dyn crate::herdr::Herdr> =
        Arc::new(crate::herdr::socket::SocketHerdr::new(herdr_socket_path));
    let notify_store = Arc::new(open_notify_store(&engine.config.terminal, None));
    let background = crate::TerminalBackground::new();
    let telegram = telegram_credentials(&engine.config.terminal, None);
    background.reconcile(
        &engine.config.terminal,
        herdr,
        notify_store,
        telegram,
        Some(engine.clone()),
    );

    runtime::write_lock(&DaemonInfo {
        pid: std::process::id(),
        started_at: now_rfc3339(),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
    })?;
    tracing::info!(
        "waggledance supervisor running (pid {})",
        std::process::id()
    );
    let _ = tokio::signal::ctrl_c().await;
    runtime::remove_lock();
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notify_store_opens_lazily_only_once_the_notify_switch_is_on() {
        let dir = tempfile::tempdir().unwrap();
        let path = waggledance_core::config::notify_store_path_override(Some(dir.path()));

        let off = waggledance_core::config::TerminalConfig::default();
        assert!(!off.notify_enabled);
        let _store = open_notify_store(&off, Some(dir.path()));
        assert!(!path.exists());

        let on = waggledance_core::config::TerminalConfig {
            notify_enabled: true,
            ..Default::default()
        };
        let _store = open_notify_store(&on, Some(dir.path()));
        assert!(path.exists());
    }
}
