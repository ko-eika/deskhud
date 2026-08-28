//! 应用运行时。
//!
//! 负责 winit 事件循环、窗口管理、视口运行时和渲染线程调度。

mod app_icon;
mod handler;
mod render;
pub(crate) mod viewport;
pub(crate) mod viewport_config;
mod window_manager;

pub fn run() {
    handler::run();
}
