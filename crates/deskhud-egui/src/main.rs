//! DeskHud 入口（egui / eframe）。

// release 无控制台；debug 保留以便看 tracing 日志
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod fonts;
mod pet_dock;
mod pet_draw;
mod pet_input;
mod pet_menu;
mod settings;
mod win_chrome;

use std::sync::{Arc, OnceLock};

use deskhud_ui::persist;
use eframe::egui;

/// 程序图标（主窗 / 设置窗共用）。
pub(crate) fn icon() -> Arc<egui::IconData> {
    static ICON: OnceLock<Arc<egui::IconData>> = OnceLock::new();
    ICON.get_or_init(|| {
        let img = image::load_from_memory(include_bytes!("../assets/icon.png"))
            .expect("icon.png")
            .into_rgba8();
        let (width, height) = img.dimensions();
        Arc::new(egui::IconData {
            rgba: img.into_raw(),
            width,
            height,
        })
    })
    .clone()
}

fn main() -> eframe::Result {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let prefs = match persist::load() {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "prefs load failed; using defaults");
            deskhud_ui::UiPreferences::default()
        }
    };
    if let Some(path) = persist::prefs_path() {
        tracing::info!(?path, "prefs path");
    }

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("")
        .with_inner_size([prefs.shell.pet_width, prefs.shell.pet_height])
        .with_decorations(false)
        .with_transparent(true)
        .with_has_shadow(false)
        .with_resizable(false)
        .with_taskbar(false)
        .with_icon(icon());
    if prefs.shell.pet_topmost {
        viewport = viewport.with_always_on_top();
    }
    if let Some([x, y]) = prefs.shell.pet_pos() {
        viewport = viewport.with_position([x, y]);
    }

    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Glow,
        multisampling: 0,
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "DeskHud",
        options,
        Box::new(move |cc| Ok(Box::new(app::PetApp::new(cc, prefs)))),
    )
}
