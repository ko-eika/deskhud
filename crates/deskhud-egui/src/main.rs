//! DeskHud application entry point.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod fonts;
#[cfg(windows)]
mod gpu_overlay_probe;
#[cfg(windows)]
mod gpu_probe;
mod image_decode;
mod native_host;
mod overlay_control;
#[cfg(windows)]
mod overlay_probe;
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

    #[cfg(windows)]
    if std::env::var_os("DESKHUD_OVERLAY_PROBE").is_some() {
        return overlay_probe::run();
    }
    #[cfg(windows)]
    if std::env::var_os("DESKHUD_GPU_PROBE").is_some() {
        return gpu_probe::run();
    }
    #[cfg(windows)]
    if std::env::var_os("DESKHUD_GPU_OVERLAY_PROBE").is_some() {
        return gpu_overlay_probe::run();
    }

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
