//! System-tray icon + menu wiring.
//!
//! The tray is the *primary* UI of the widget. The "settings" window only
//! pops up when the user explicitly asks for it.

use std::sync::Arc;

use anyhow::{Context, Result};
use tauri::{
    image::Image,
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Manager, Wry,
};
use tokio::sync::RwLock;

use crate::config::WidgetConfig;
use crate::status_poller::{SpokeState, StatusPoller};

/// IDs of all tray-menu items. Centralised so we can match on them in the
/// event handler without sprinkling string literals around.
pub mod ids {
    pub const OPEN_DASHBOARD: &str = "open_dashboard";
    pub const STATUS_LABEL: &str = "status_label";
    pub const RESTART_STACK: &str = "restart_stack";
    pub const PAUSE_HEARTBEAT: &str = "pause_heartbeat";
    pub const SETTINGS: &str = "settings";
    pub const ABOUT: &str = "about";
    pub const QUIT: &str = "quit";
}

/// Resolve an embedded icon for the given state.
///
/// Falls back to the "unknown" icon if the requested PNG isn't bundled
/// (e.g. during early development). This keeps the tray *always* visible.
fn load_state_icon(app: &AppHandle, state: SpokeState) -> Result<Image<'static>> {
    let filename = state.icon_filename();
    let resolver = app.path();
    let candidate = resolver
        .resolve(
            format!("icons/{filename}"),
            tauri::path::BaseDirectory::Resource,
        )
        .ok();
    if let Some(path) = candidate {
        if path.exists() {
            return Image::from_path(path)
                .map(|img| img.to_owned())
                .context("loading state icon from resources");
        }
    }
    // Dev fallback: read from project source tree.
    let src_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("icons")
        .join(filename);
    if src_path.exists() {
        return Image::from_path(src_path)
            .map(|img| img.to_owned())
            .context("loading state icon from manifest dir");
    }
    // Last-resort fallback: 1x1 transparent pixel so the tray stays visible.
    const PIXEL: &[u8] = &[0u8, 0u8, 0u8, 0u8];
    Ok(Image::new(PIXEL, 1, 1).to_owned())
}

/// Build the tray menu. Returns the menu plus the dynamic "status" item
/// so the poller task can update its label later.
fn build_menu(app: &AppHandle) -> Result<(Menu<Wry>, MenuItem<Wry>)> {
    let open_dash = MenuItem::with_id(
        app,
        ids::OPEN_DASHBOARD,
        "Open Dashboard",
        true,
        None::<&str>,
    )?;
    let status_label = MenuItem::with_id(
        app,
        ids::STATUS_LABEL,
        "Status: starting…",
        false, // disabled => looks like a header
        None::<&str>,
    )?;
    let restart = MenuItem::with_id(
        app,
        ids::RESTART_STACK,
        "Restart Spoke-Stack",
        true,
        None::<&str>,
    )?;
    let pause = MenuItem::with_id(
        app,
        ids::PAUSE_HEARTBEAT,
        "Pause Heartbeat",
        true,
        None::<&str>,
    )?;
    let settings = MenuItem::with_id(app, ids::SETTINGS, "Settings…", true, None::<&str>)?;
    let about = MenuItem::with_id(app, ids::ABOUT, "About…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, ids::QUIT, "Quit", true, None::<&str>)?;
    let sep1 = PredefinedMenuItem::separator(app)?;
    let sep2 = PredefinedMenuItem::separator(app)?;

    let menu = Menu::with_items(
        app,
        &[
            &open_dash,
            &status_label,
            &sep1,
            &restart,
            &pause,
            &settings,
            &sep2,
            &about,
            &quit,
        ],
    )?;
    Ok((menu, status_label))
}

/// Install the tray icon, wire menu events, and spawn a small task that
/// reflects [`SpokeState`] transitions into the tray icon + label.
pub fn setup(
    app: &AppHandle,
    config: Arc<RwLock<WidgetConfig>>,
    poller: Arc<StatusPoller>,
) -> Result<()> {
    let (menu, status_item) = build_menu(app)?;
    let initial_icon = load_state_icon(app, SpokeState::Unknown)?;

    let tray = TrayIconBuilder::with_id("spoke-widget-tray")
        .icon(initial_icon)
        .icon_as_template(false)
        .tooltip("Spoke Widget - starting…")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |app, event| {
            handle_menu_event(app, event.id.as_ref());
        })
        .on_tray_icon_event(|tray, event| {
            // Single left-click anywhere on the icon → open dashboard.
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                let app = tray.app_handle().clone();
                handle_menu_event(&app, ids::OPEN_DASHBOARD);
            }
        })
        .build(app)?;

    // Stash the tray in app state so other modules can update it.
    app.manage(TrayState {
        status_item: status_item.clone(),
    });

    // Spawn the state-reflector task. It runs on the tokio runtime created
    // by the Tauri builder.
    let app_handle = app.clone();
    let mut rx = poller.subscribe();
    tauri::async_runtime::spawn(async move {
        while rx.changed().await.is_ok() {
            let state = *rx.borrow();
            if let Err(err) = apply_state(&app_handle, &tray, &status_item, state) {
                log::warn!("could not apply tray state: {err}");
            }
        }
    });

    Ok(())
}

/// Snapshot of dynamic tray-menu items, stored in Tauri's `State`.
pub struct TrayState {
    #[allow(dead_code)]
    pub status_item: MenuItem<Wry>,
}

fn apply_state(
    app: &AppHandle,
    tray: &tauri::tray::TrayIcon,
    status_item: &MenuItem<Wry>,
    state: SpokeState,
) -> Result<()> {
    let icon = load_state_icon(app, state)?;
    tray.set_icon(Some(icon))?;
    let label = format!("Status: {}", state.label());
    status_item.set_text(&label)?;
    tray.set_tooltip(Some(format!("Spoke Widget — {}", state.label())))?;
    Ok(())
}

fn handle_menu_event(app: &AppHandle, id: &str) {
    log::debug!("tray menu event: {id}");
    match id {
        ids::OPEN_DASHBOARD => {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let cfg = match app2.try_state::<crate::AppState>() {
                    Some(s) => s.config.read().await.clone(),
                    None => WidgetConfig::default(),
                };
                use tauri_plugin_shell::ShellExt;
                #[allow(deprecated)]
                if let Err(err) = app2.shell().open(cfg.dashboard_url(), None) {
                    log::error!("failed to open dashboard: {err}");
                }
            });
        }
        ids::RESTART_STACK => {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = app2.try_state::<crate::AppState>() else {
                    return;
                };
                let cfg = state.config.read().await.clone();
                match crate::agent_actions::restart_all(&cfg).await {
                    Ok(msg) => log::info!("{msg}"),
                    Err(err) => log::error!("restart_all failed: {err}"),
                }
            });
        }
        ids::PAUSE_HEARTBEAT => {
            let app2 = app.clone();
            tauri::async_runtime::spawn(async move {
                let Some(state) = app2.try_state::<crate::AppState>() else {
                    return;
                };
                let cfg = state.config.read().await.clone();
                match crate::agent_actions::toggle_heartbeat(&cfg).await {
                    Ok(msg) => log::info!("{msg}"),
                    Err(err) => log::warn!("heartbeat toggle failed: {err}"),
                }
            });
        }
        ids::SETTINGS => {
            if let Some(window) = app.get_webview_window("settings") {
                let _ = window.show();
                let _ = window.set_focus();
            } else {
                log::warn!("settings window not found");
            }
        }
        ids::ABOUT => {
            use tauri_plugin_shell::ShellExt;
            #[allow(deprecated)]
            if let Err(err) = app.shell().open(
                "https://github.com/janpow77/spoke-widget",
                None,
            ) {
                log::error!("failed to open About URL: {err}");
            }
        }
        ids::QUIT => {
            app.exit(0);
        }
        ids::STATUS_LABEL => {
            // disabled item, intentional no-op
        }
        other => log::debug!("unhandled menu id: {other}"),
    }
}
