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
    runtime::run();
}
