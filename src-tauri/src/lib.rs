//! Library crate exposing the spoke-widget runtime. The thin `main.rs`
//! binary calls into [`run`].

pub mod agent_actions;
pub mod config;
pub mod opener;
pub mod status_poller;
pub mod tray;

use std::sync::Arc;

use anyhow::Context;
use tokio::sync::RwLock;

use crate::config::WidgetConfig;
use crate::status_poller::StatusPoller;

/// Tauri-managed runtime state, shared across menu handlers + commands.
pub struct AppState {
    pub config: Arc<RwLock<WidgetConfig>>,
    pub poller: Arc<StatusPoller>,
}

#[tauri::command]
async fn get_config(state: tauri::State<'_, AppState>) -> Result<WidgetConfig, String> {
    Ok(state.config.read().await.clone())
}

#[tauri::command]
async fn save_config(
    new_cfg: WidgetConfig,
    state: tauri::State<'_, AppState>,
) -> Result<(), String> {
    new_cfg.save().map_err(|e| e.to_string())?;
    *state.config.write().await = new_cfg;
    Ok(())
}

#[tauri::command]
async fn open_dashboard(app: tauri::AppHandle) -> Result<(), String> {
    use tauri::Manager;
    use tauri_plugin_shell::ShellExt;
    let cfg = app
        .state::<AppState>()
        .config
        .read()
        .await
        .clone();
    #[allow(deprecated)]
    app.shell()
        .open(cfg.dashboard_url(), None)
        .map_err(|e| e.to_string())
}

/// Entry point. Sets up logging, loads config, starts the poller and
/// installs the tray.
pub fn run() {
    if env_logger::try_init().is_err() {
        // logger already set up by Tauri / tests – fine.
    }

    let config = Arc::new(RwLock::new(WidgetConfig::load().unwrap_or_default()));
    let poller = Arc::new(StatusPoller::new(config.clone()));

    let state = AppState {
        config: config.clone(),
        poller: poller.clone(),
    };

    let autostart_enabled = {
        let cfg = config.blocking_read();
        cfg.autostart
    };

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .manage(state)
        .invoke_handler(tauri::generate_handler![
            get_config,
            save_config,
            open_dashboard
        ])
        .setup(move |app| {
            let handle = app.handle().clone();

            // Apply autostart preference.
            {
                use tauri_plugin_autostart::ManagerExt;
                let manager = handle.autolaunch();
                let already_enabled = manager.is_enabled().unwrap_or(false);
                if autostart_enabled && !already_enabled {
                    let _ = manager.enable();
                } else if !autostart_enabled && already_enabled {
                    let _ = manager.disable();
                }
            }

            // Hide the settings window on launch — tray only.
            if let Some(window) = handle.get_webview_window("settings") {
                let _ = window.hide();
            }

            // Spawn poller.
            poller.clone().spawn();

            // Install tray.
            tray::setup(&handle, config.clone(), poller.clone())
                .context("setting up tray")?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("error while building tauri application")
        .run(|_app_handle, event| {
            if let tauri::RunEvent::ExitRequested { api, .. } = event {
                // Keep app alive when the (hidden) settings window is closed.
                api.prevent_exit();
            }
        });
}

// Re-export for downstream tests.
pub use tauri::Manager as _Manager;
