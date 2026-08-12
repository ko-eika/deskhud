//! DeskHud application entry point.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fonts;
#[cfg(windows)]
mod gpu_overlay_probe;
mod image_decode;
mod native_host;
mod overlay_control;
mod pet_menu;
mod pet_scene;
mod platform;
mod settings;
mod theme;

use deskhud_ui::persist;
use overlay_control::OverlayControlBus;

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let prefs = persist::load().unwrap_or_else(|error| {
        tracing::warn!(%error, "prefs load failed; using defaults");
        deskhud_ui::UiPreferences::default()
    });
    if let Some(path) = persist::prefs_path() {
        tracing::info!(?path, "prefs path");
    }

    let controls = OverlayControlBus::default();
    #[cfg(windows)]
    {
        gpu_overlay_probe::spawn(controls.clone(), prefs.shell.topmost, prefs.pet.pos())?;
        native_host::run(prefs, controls)
    }

    #[cfg(not(windows))]
    native_host::run(prefs, controls)
}
