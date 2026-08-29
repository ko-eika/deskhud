//! 应用事件循环模块。
//!
//! 该模块只负责接收 winit 事件，并将窗口相关工作交给窗口管理器。

use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, Instant},
};

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

use super::{
    app_icon,
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
            pet_window_id: None,
            bubble_window_id: None,
            drag_follow: None,
            #[cfg(target_os = "macos")]
            _global_key_monitor: None,
            #[cfg(target_os = "windows")]
            _global_key_monitor: None,
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
    /// 原生拖动时需零等待同步位置的两个窗口。
    pet_window_id: Option<WindowId>,
    bubble_window_id: Option<WindowId>,
    /// 原生系统拖动期间以全局指针预测宠物位置，绕过低频 `Moved` 事件。
    drag_follow: Option<DragFollow>,
    /// 必须持有 monitor，才能让 CoreGraphics event tap 在主线程 RunLoop 中持续生效。
    #[cfg(target_os = "macos")]
    _global_key_monitor: Option<crate::input::GlobalKeyMonitor>,
    /// Windows low-level hooks must be retained until the event loop exits.
    #[cfg(target_os = "windows")]
    _global_key_monitor: Option<crate::input::GlobalKeyMonitor>,
    /// 渲染线程已收到退出请求，等待其完成资源释放后再退出事件循环。
    closing: bool,
}

impl ApplicationHandler<UserEvent> for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        // resumed 可能被系统调用多次，只在第一次恢复时创建窗口和渲染线程。
        if self.renderer.is_none() {
            app_icon::set_visibility(false);
            let mut windows = WindowManager::new(self.proxy.clone());
            windows.create_pet(event_loop);
            let (pet_window_id, bubble_window_id) = windows
                .pet_and_bubble_window_ids()
                .expect("pet and bubble viewports must be created together");
            let handles = windows.window_handles();
            self.windows = handles.into_iter().collect();
            self.pet_window_id = Some(pet_window_id);
            self.bubble_window_id = Some(bubble_window_id);
            #[cfg(target_os = "macos")]
            {
                let (keyboard, mouse) = windows.global_input_monitoring();
                self.set_global_input_monitoring(keyboard, mouse);
            }
            #[cfg(target_os = "windows")]
            {
                let (keyboard, mouse) = windows.global_input_monitoring();
                self.set_global_input_monitoring(keyboard, mouse);
            }
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
            UserEvent::PetEvent(event) => {
                if let Some(renderer) = &self.renderer {
                    renderer.send(RenderCommand::PetEvent(event));
                }
            }
            UserEvent::SetGlobalInputMonitoring { keyboard, mouse } => {
                #[cfg(target_os = "macos")]
                self.set_global_input_monitoring(keyboard, mouse);
                #[cfg(target_os = "windows")]
                self.set_global_input_monitoring(keyboard, mouse);
                #[cfg(not(target_os = "macos"))]
                #[cfg(not(target_os = "windows"))]
                let _ = (keyboard, mouse);
            }
            UserEvent::WindowCommand { window_id, command } => {
                if let Some(window) = self.windows.get(&window_id).cloned() {
                    match command {
                        WindowCommand::Drag => {
                            if self.pet_window_id == Some(window_id) {
                                self.start_drag_follow(&window);
                            }
                            #[cfg(target_os = "macos")]
                            {
                                // 不把透明宠物交给 AppKit 的原生拖拽，避免
                                // macOS 在屏幕边缘触发系统窗口平铺/吸附。
                                let _ = window;
                            }
                            #[cfg(all(not(target_os = "macos"), not(target_os = "windows")))]
                            let _ = window.drag_window();
                            #[cfg(target_os = "windows")]
                            {
                                // Windows 使用上面的全局指针跟随，不能再叠加
                                // drag_window()，否则系统拖动起点和绝对坐标跟随
                                // 会同时修改窗口位置，表现为鼠标移动很多但宠物
                                // 只移动一点。
                                let _ = window;
                            }
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
                            app_icon::set_visibility(visible);
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

        if let WindowEvent::Moved(position) = &event {
            self.follow_bubble_on_pet_move(window_id, *position);
        }

        if !self.closing {
            if let Some(renderer) = &self.renderer {
                renderer.send(RenderCommand::WindowEvent { window_id, event });
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.update_drag_follow() {
            // 拖动期间以 120Hz 采样全局指针，确保独立工具窗紧贴宠物。
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(8),
            ));
        } else {
            // 没有待处理事件时休眠，重绘请求会通过 UserEvent 唤醒事件循环。
            event_loop.set_control_flow(ControlFlow::Wait);
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        // 先停止并等待渲染线程，确保 OpenGL 资源仍在正确线程中销毁。
        if let Some(renderer) = self.renderer.take() {
            renderer.shutdown();
        }
    }
}

impl App {
    #[cfg(target_os = "macos")]
    fn set_global_input_monitoring(&mut self, keyboard: bool, mouse: bool) {
        self._global_key_monitor = (keyboard || mouse)
            .then(|| crate::input::install_global_key_monitor(self.proxy.clone(), keyboard, mouse))
            .flatten();
        if (keyboard || mouse) && self._global_key_monitor.is_none() {
            tracing::warn!(
                "global input monitor unavailable; grant Accessibility permission to DeskHud"
            );
        }
    }

    #[cfg(target_os = "windows")]
    fn set_global_input_monitoring(&mut self, keyboard: bool, mouse: bool) {
        self._global_key_monitor = (keyboard || mouse)
            .then(|| crate::input::install_global_key_monitor(self.proxy.clone(), keyboard, mouse))
            .flatten();
        if (keyboard || mouse) && self._global_key_monitor.is_none() {
            tracing::warn!("global input monitor unavailable on Windows");
        }
    }

    /// 原生拖动事件抵达主线程时立即移动工具窗，避免经渲染线程往返一帧。
    fn follow_bubble_on_pet_move(
        &self,
        window_id: WindowId,
        pet_position: winit::dpi::PhysicalPosition<i32>,
    ) {
        if self.pet_window_id != Some(window_id) {
            return;
        }
        self.position_bubble_for_pet(window_id, pet_position);
    }

    fn start_drag_follow(&mut self, pet: &winit::window::Window) {
        let (Ok(pet_origin), Some(pointer_origin)) = (
            pet.outer_position(),
            crate::input::global_pointer_position(),
        ) else {
            return;
        };
        self.drag_follow = Some(DragFollow {
            pet_origin,
            pointer_origin,
        });
    }

    /// 返回拖动跟随是否仍在进行，以便事件循环安排下一次低延迟采样。
    fn update_drag_follow(&mut self) -> bool {
        let Some(follow) = self.drag_follow else {
            return false;
        };
        if !crate::input::global_mouse_buttons().primary_down {
            self.drag_follow = None;
            return false;
        }
        let Some(pointer) = crate::input::global_pointer_position() else {
            // Keep the active drag alive across a transient global-pointer
            // sampling failure. Clearing it here makes the pet stop following
            // while the button is still down and can desynchronize drag state.
            return true;
        };
        let Some(pet_window_id) = self.pet_window_id else {
            self.drag_follow = None;
            return false;
        };
        let position = winit::dpi::PhysicalPosition::new(
            follow.pet_origin.x + (pointer[0] - follow.pointer_origin[0]).round() as i32,
            follow.pet_origin.y + (pointer[1] - follow.pointer_origin[1]).round() as i32,
        );
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        if let Some(pet) = self.windows.get(&pet_window_id) {
            pet.set_outer_position(position);
        }
        #[cfg(any(target_os = "macos", target_os = "windows"))]
        let position = self
            .windows
            .get(&pet_window_id)
            .and_then(|pet| pet.outer_position().ok())
            .unwrap_or(position);
        self.position_bubble_for_pet(pet_window_id, position);
        true
    }

    fn position_bubble_for_pet(
        &self,
        pet_window_id: WindowId,
        pet_position: winit::dpi::PhysicalPosition<i32>,
    ) {
        let (Some(pet), Some(bubble_id)) =
            (self.windows.get(&pet_window_id), self.bubble_window_id)
        else {
            return;
        };
        let Some(bubble) = self.windows.get(&bubble_id) else {
            return;
        };

        const BUBBLE_WIDTH: i32 = 180;
        const BUBBLE_HEIGHT: i32 = 52;
        const GAP: i32 = 12;
        let pet_size = pet.outer_size();
        let center_x = pet_position.x + pet_size.width as i32 / 2;
        let (area_position, area_size) = pet
            .current_monitor()
            .map(|monitor| (monitor.position(), monitor.size()))
            .unwrap_or((winit::dpi::PhysicalPosition::new(0, 0), pet_size));
        let min_x = area_position.x + BUBBLE_WIDTH / 2;
        let max_x = area_position.x + area_size.width as i32 - BUBBLE_WIDTH / 2;
        let x = center_x.clamp(min_x, max_x.max(min_x));
        let above = pet_position.y - BUBBLE_HEIGHT - GAP >= area_position.y;
        let y = if above {
            pet_position.y - BUBBLE_HEIGHT / 2 - GAP
        } else {
            (pet_position.y + pet_size.height as i32 + BUBBLE_HEIGHT / 2 + GAP)
                .min(area_position.y + area_size.height as i32 - BUBBLE_HEIGHT / 2)
        };
        bubble.set_outer_position(winit::dpi::PhysicalPosition::new(
            x - BUBBLE_WIDTH / 2,
            y - BUBBLE_HEIGHT / 2,
        ));
    }
}

#[derive(Clone, Copy)]
struct DragFollow {
    pet_origin: winit::dpi::PhysicalPosition<i32>,
    pointer_origin: [f64; 2],
}
