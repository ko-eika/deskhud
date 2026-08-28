//! 程序入口。
//!
//! 具体的 winit 事件循环和窗口生命周期由 [`runtime`] 模块负责；这里仅负责启动应用。

#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod area;
mod components;
mod fonts;
mod graphics;
mod image_decode;
mod input;
mod menu;
mod runtime;
mod views;

/// 启动桌面应用事件循环。
fn main() {
    init_logging();
    runtime::run();
}

/// Windows GUI 子系统没有控制台；将运行日志写入用户配置目录，避免保存/加载问题
/// 发生时完全没有可观测信息。
fn init_logging() {
    let log_path = deskhud_ui::prefs_path().map(|path| path.with_file_name("deskhud.log"));
    let Some(log_path) = log_path else {
        return;
    };
    let Some(parent) = log_path.parent() else {
        return;
    };
    if std::fs::create_dir_all(parent).is_err() {
        return;
    }
    let Ok(file) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_path)
    else {
        return;
    };
    let _ = tracing_subscriber::fmt()
        .with_ansi(false)
        .with_max_level(tracing::Level::INFO)
        .with_writer(file)
        .try_init();
    tracing::info!(path = ?log_path, "logging initialized");
}
