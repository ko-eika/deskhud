//! DeskHud application entry point.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

// 设置页与宠物菜单只在 Windows / macOS 走平台原生 + egui 路径接入；
// Linux 目前为宠物运行态专用，相关 UI 模块按未启用处理。
#[cfg_attr(target_os = "linux", allow(dead_code))]
mod fonts;
#[cfg(windows)]
mod gpu_overlay_probe;
#[cfg_attr(target_os = "linux", allow(dead_code))]
mod image_decode;
mod native_host;
#[cfg(windows)]
mod native_menu;
mod overlay_control;
mod overlay_surface;
#[cfg_attr(target_os = "linux", allow(dead_code))]
mod pet_menu;
mod pet_scene;
mod platform;
#[cfg_attr(target_os = "linux", allow(dead_code))]
mod settings;
mod single_instance;
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

    let Some(_instance_guard) = single_instance::acquire_or_activate() else {
        return Ok(());
    };

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
