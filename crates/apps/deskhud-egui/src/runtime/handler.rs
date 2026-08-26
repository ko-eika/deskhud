//! 应用事件循环模块。
//!
//! 该模块只负责接收 winit 事件，并将窗口相关工作交给窗口管理器。

use std::{collections::HashMap, sync::Arc};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use super::{
    render::{RenderCommand, RenderResult, Renderer, WindowCommand},
    viewport::UserEvent,
    window_manager::WindowManager,
};

pub(crate) fn run() {
    // winit 主线程只接收系统事件，并把事件转发给专用渲染线程。
    // OpenGL Context 不在这里绘制，避免多个线程同时操作图形资源。
    let event_loop = EventLoop::<UserEvent>::with_user_event()
        .build()
        .expect("创建事件循环失败");
    let proxy = event_loop.create_proxy();
    event_loop.set_control_flow(ControlFlow::Wait);
    event_loop
        .run_app(&mut App {
            proxy,
            renderer: None,
            windows: HashMap::new(),
            closing: false,
        })
        .expect("运行事件循环失败");
}

struct App {
    /// 用于向 winit 事件循环发送跨线程通知。
    proxy: winit::event_loop::EventLoopProxy<UserEvent>,
    /// 专用渲染线程；应用退出时通过 `shutdown` 等待其释放资源。
    renderer: Option<Renderer>,
    /// 主线程持有的窗口句柄，用于执行原生窗口尺寸和位置命令。
    windows: HashMap<WindowId, Arc<winit::window::Window>>,
    /// 渲染线程已收到退出请求，等待其完成资源释放后再退出事件循环。
    closing: bool,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // resumed 可能被系统调用多次，只在第一次恢复时创建窗口和渲染线程。
        if self.renderer.is_none() {
            #[cfg(target_os = "macos")]
            set_macos_dock_icon(false);
            let mut windows = WindowManager::new(self.proxy.clone());
            windows.create_pet(event_loop);
            let handles = windows.window_handles();
            self.windows = handles.into_iter().collect();
            self.renderer = Some(Renderer::start(windows, self.proxy.clone()));
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: UserEvent) {
        // 用户事件来自 egui 回调或渲染线程，必须在 winit 线程执行原生窗口操作。
        match event {
            UserEvent::Repaint => {
                if let Some(renderer) = &self.renderer {
                    renderer.send(RenderCommand::RenderNow);
                }
            }
            UserEvent::RenderResult(RenderResult::ShouldClose) => {
                #[cfg(target_os = "macos")]
                {
                    // macOS 的 AppKit/NSOpenGL 对象必须在主线程析构；当前
                    // 渲染线程仍持有这些对象时，进入 winit::exiting 并 join
                    // 会导致退出卡死。应用退出时直接结束进程，交由 macOS
                    // 回收窗口和图形资源，避免跨线程析构。
                    std::process::exit(0);
                }
                #[cfg(not(target_os = "macos"))]
                {
                    self.closing = true;
                }
            }
            UserEvent::RenderResult(RenderResult::Stopped) => {
                if self.closing {
                    event_loop.exit();
                }
            }
            UserEvent::WindowCommand { window_id, command } => {
                if let Some(window) = self.windows.get(&window_id) {
                    match command {
                        WindowCommand::Drag => {
                            let _ = window.drag_window();
                        }
                        WindowCommand::Resize { width, height } => {
                            let _ = window
                                .request_inner_size(winit::dpi::PhysicalSize::new(width, height));
                        }
                        WindowCommand::Move { position } => {
                            window.set_outer_position(position);
                        }
                        #[cfg(target_os = "macos")]
                        WindowCommand::SetDockIcon { visible } => {
                            set_macos_dock_icon(visible);
                        }
                    }
                }
            }
        }
    }

    fn window_event(
        &mut self,
        _event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        // 绘制统一由渲染模块调度，因此主线程收到原生重绘通知时不直接绘制。
        if matches!(event, WindowEvent::RedrawRequested) {
            return;
        }

        if !self.closing {
            if let Some(renderer) = &self.renderer {
                renderer.send(RenderCommand::WindowEvent { window_id, event });
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        // 没有待处理事件时休眠，重绘请求会通过 UserEvent 唤醒事件循环。
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // 先停止并等待渲染线程，确保 OpenGL 资源仍在正确线程中销毁。
        if let Some(renderer) = self.renderer.take() {
            renderer.shutdown();
        }
    }
}

#[cfg(target_os = "macos")]
fn set_macos_dock_icon(visible: bool) {
    use core::{
        ffi::{c_char, c_void},
        mem,
    };

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_msgSend();
        fn objc_getClass(name: *const c_char) -> Id;
        fn sel_registerName(name: *const c_char) -> Sel;
    }

    unsafe {
        let send_id: unsafe extern "C" fn(Id, Sel) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let send_policy: unsafe extern "C" fn(Id, Sel, isize) -> bool =
            mem::transmute(objc_msgSend as *const ());
        let send_id_arg: unsafe extern "C" fn(Id, Sel, Id) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let send_data: unsafe extern "C" fn(Id, Sel, *const u8, usize) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let application = send_id(
            objc_getClass(c"NSApplication".as_ptr()),
            sel_registerName(c"sharedApplication".as_ptr()),
        );
        if !application.is_null() {
            // NSApplicationActivationPolicyRegular = 0;
            // NSApplicationActivationPolicyAccessory = 1.
            let policy = if visible { 0 } else { 1 };
            let _ = send_policy(
                application,
                sel_registerName(c"setActivationPolicy:".as_ptr()),
                policy,
            );

            // 直接设置 NSApplication.applicationIconImage，保证 cargo run
            // 或未及时刷新 Dock 缓存的 .app 也使用项目图标。
            let data_class = objc_getClass(c"NSData".as_ptr());
            let image_class = objc_getClass(c"NSImage".as_ptr());
            let data = send_data(
                data_class,
                sel_registerName(c"dataWithBytes:length:".as_ptr()),
                APP_ICON_PNG.as_ptr(),
                APP_ICON_PNG.len(),
            );
            let image = if data.is_null() || image_class.is_null() {
                core::ptr::null_mut()
            } else {
                let image = send_id(image_class, sel_registerName(c"alloc".as_ptr()));
                send_id_arg(image, sel_registerName(c"initWithData:".as_ptr()), data)
            };
            if !image.is_null() {
                let _ = send_id_arg(
                    application,
                    sel_registerName(c"setApplicationIconImage:".as_ptr()),
                    image,
                );
            }
        }
    }
}

#[cfg(target_os = "macos")]
const APP_ICON_PNG: &[u8] = include_bytes!("../../../../../assets/icon.png");
