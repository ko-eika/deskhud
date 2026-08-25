//! 单个原生视口的运行时封装。
//!
//! 该模块负责维护窗口、egui 输入状态、egui 上下文和 OpenGL 绘制器之间的关系。

#![cfg_attr(target_os = "macos", allow(dead_code))]

use std::sync::Arc;

use egui::{Color32, Context, RawInput};
use egui_glow::Painter;
use egui_winit::State;
use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::graphics::GlWindow;
use crate::views::ViewOutput;

use super::{
    render::{RenderResult, WindowCommand},
    viewport_config::ViewportConfig,
};

pub(crate) struct ViewportOutput {
    /// UI 请求关闭当前窗口或整个应用。
    pub should_close: bool,
    /// UI 请求的逻辑尺寸。
    pub resize_to: Option<[f32; 2]>,
    /// 菜单点击返回的业务标识。
    pub selected_menu_item: Option<String>,
    /// 当前悬浮的子菜单索引。
    pub open_submenu: Option<usize>,
    /// 当前悬浮的菜单项索引。
    pub hovered_item: Option<usize>,
    /// 子菜单触发区域的逻辑坐标和高度。
    pub submenu_anchor: Option<[f32; 3]>,
}

/// 由 egui 或其它线程发送给 winit 事件循环的唤醒事件。
#[derive(Debug)]
pub(crate) enum UserEvent {
    Repaint,
    RenderResult(RenderResult),
    WindowCommand {
        window_id: WindowId,
        command: WindowCommand,
    },
}

/// 原生窗口的层级。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum WindowLayer {
    AlwaysOnTop,
    Normal,
    AlwaysOnBottom,
}

/// 一个独立原生窗口的完整运行时状态。
pub(crate) struct Viewport {
    /// 原生窗口及其 OpenGL Context/Surface。
    gl_window: GlWindow,
    /// egui 绘制上下文。
    context: Context,
    /// winit 到 egui 的输入状态适配器。
    state: State,
    /// 向 winit 线程发送重绘和窗口命令的代理。
    proxy: EventLoopProxy<UserEvent>,
    /// 当前视口的 egui OpenGL 绘制器。
    painter: Option<Painter>,
    transparent: bool,
    drag_anywhere: bool,
    always_on_top: bool,
    window_layer: WindowLayer,
    visible: bool,
    cursor_position: Option<PhysicalPosition<f64>>,
    destroyed: bool,
}

impl Viewport {
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        config: ViewportConfig,
        proxy: &EventLoopProxy<UserEvent>,
    ) -> Self {
        let gl_window = unsafe {
            GlWindow::new_with_title(
                event_loop,
                config.title,
                config.size,
                config.decorations,
                config.transparent,
                config.resizable,
                config.skip_taskbar,
                config.visible,
                config.undecorated_shadow,
                config.x11_popup,
                None,
            )
        };
        let context = Context::default();
        crate::fonts::configure_context(&context);
        let repaint_proxy = proxy.clone();
        context.set_request_repaint_callback(move |_info| {
            let _ = repaint_proxy.send_event(UserEvent::Repaint);
        });
        let state = State::new(
            context.clone(),
            config.egui_id,
            gl_window.window(),
            None,
            None,
            None,
        );
        if config.always_on_top {
            gl_window
                .window()
                .set_window_level(winit::window::WindowLevel::AlwaysOnTop);
        }
        Self {
            gl_window,
            context,
            state,
            proxy: proxy.clone(),
            painter: None,
            transparent: config.transparent,
            drag_anywhere: config.drag_anywhere,
            always_on_top: config.always_on_top,
            window_layer: if config.always_on_top {
                WindowLayer::AlwaysOnTop
            } else {
                WindowLayer::Normal
            },
            visible: config.visible,
            cursor_position: None,
            destroyed: false,
        }
    }

    pub(crate) fn window_id(&self) -> WindowId {
        self.gl_window.window().id()
    }

    pub(crate) fn window_handle(&self) -> Arc<winit::window::Window> {
        self.gl_window.window_handle()
    }

    pub(crate) fn request_inner_size(&self, size: PhysicalSize<u32>) {
        let _ = self.proxy.send_event(UserEvent::WindowCommand {
            window_id: self.window_id(),
            command: WindowCommand::Resize {
                width: size.width,
                height: size.height,
            },
        });
    }

    pub(crate) fn request_outer_position(&self, position: PhysicalPosition<i32>) {
        let _ = self.proxy.send_event(UserEvent::WindowCommand {
            window_id: self.window_id(),
            command: WindowCommand::Move { position },
        });
    }

    /// 返回当前视口的原生窗口。
    pub(crate) fn window(&self) -> &winit::window::Window {
        self.gl_window.window()
    }

    pub(crate) fn set_visible(&mut self, visible: bool) {
        self.visible = visible;
        self.gl_window.window().set_visible(visible);
        if visible {
            let _ = self.gl_window.window().set_cursor_hittest(true);
        }
    }

    pub(crate) fn set_visible_without_focus(&mut self, visible: bool) {
        self.visible = visible;
        self.gl_window.window().set_visible(visible);
        if visible {
            let _ = self.gl_window.window().set_cursor_hittest(true);
        }
    }

    /// 设置窗口是否接收鼠标命中。
    ///
    /// 透明窗口的视觉透明和鼠标穿透是两个独立的系统属性；HUD 在普通
    /// 显示状态下需要关闭命中测试，避免挡住其下方的应用窗口。
    pub(crate) fn set_cursor_hittest(&mut self, enabled: bool) {
        let _ = self.gl_window.window().set_cursor_hittest(enabled);
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.visible
    }

    /// 返回当前窗口是否处于置顶状态。
    pub(crate) fn window_layer(&self) -> WindowLayer {
        self.window_layer
    }

    pub(crate) fn set_window_layer(&mut self, layer: WindowLayer) {
        self.window_layer = layer;
        self.always_on_top = layer == WindowLayer::AlwaysOnTop;
        let level = match layer {
            WindowLayer::AlwaysOnTop => winit::window::WindowLevel::AlwaysOnTop,
            WindowLayer::Normal => winit::window::WindowLevel::Normal,
            WindowLayer::AlwaysOnBottom => winit::window::WindowLevel::AlwaysOnBottom,
        };
        self.gl_window.window().set_window_level(level);
    }

    /// 返回最近一次鼠标位置对应的屏幕坐标。
    pub(crate) fn cursor_screen_position(&self) -> Option<PhysicalPosition<i32>> {
        let cursor = self.cursor_position?;
        let window = self.gl_window.window().outer_position().ok()?;
        Some(PhysicalPosition::new(
            window.x + cursor.x.round() as i32,
            window.y + cursor.y.round() as i32,
        ))
    }

    pub(crate) fn toggle_always_on_top(&mut self) {
        let layer = if self.always_on_top {
            WindowLayer::Normal
        } else {
            WindowLayer::AlwaysOnTop
        };
        self.set_window_layer(layer);
    }

    pub(crate) fn render<F>(&mut self, draw_ui: F) -> ViewportOutput
    where
        F: FnOnce(&Context, RawInput) -> ViewOutput,
    {
        let size = self.gl_window.window().inner_size();
        if size.width == 0 || size.height == 0 {
            return ViewportOutput {
                should_close: false,
                resize_to: None,
                selected_menu_item: None,
                open_submenu: None,
                hovered_item: None,
                submenu_anchor: None,
            };
        }
        // 另一个视口可能刚刚完成绘制，因此先恢复当前视口的 OpenGL 上下文。
        self.gl_window.make_current();
        if self.painter.is_none() {
            let painter = self.gl_window.create_painter();
            self.state.set_max_texture_side(painter.max_texture_side());
            self.painter = Some(painter);
        }
        let raw_input = self.state.take_egui_input(self.gl_window.window());
        let ui_output = draw_ui(&self.context, raw_input);
        let move_by = ui_output.move_by;
        let result = ViewportOutput {
            should_close: ui_output.should_close,
            resize_to: ui_output.resize_to,
            selected_menu_item: ui_output.selected_menu_item,
            open_submenu: ui_output.open_submenu,
            hovered_item: ui_output.hovered_item,
            submenu_anchor: ui_output.submenu_anchor,
        };
        let egui::FullOutput {
            platform_output,
            mut textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = ui_output.full_output;
        self.state
            .handle_platform_output(self.gl_window.window(), platform_output);
        if let Some([width, height]) = result.resize_to {
            let requested_size = PhysicalSize::new(
                (width * pixels_per_point).round().max(1.0) as u32,
                (height * pixels_per_point).round().max(1.0) as u32,
            );
            let current_size = self.gl_window.window().inner_size();
            if current_size != requested_size {
                self.request_inner_size(requested_size);
            }
        }
        if let Some([delta_x, delta_y]) = move_by {
            if let Ok(position) = self.gl_window.window().outer_position() {
                let delta = PhysicalPosition::new(
                    (delta_x * pixels_per_point).round() as i32,
                    (delta_y * pixels_per_point).round() as i32,
                );
                self.request_outer_position(PhysicalPosition::new(
                    position.x + delta.x,
                    position.y + delta.y,
                ));
            }
        }
        let primitives = self.context.tessellate(shapes, pixels_per_point);
        let clear_color = if self.transparent {
            Color32::TRANSPARENT.to_normalized_gamma_f32()
        } else {
            Color32::from_rgb(30, 32, 38).to_normalized_gamma_f32()
        };
        let painter = self.painter.as_mut().expect("Painter 未初始化");
        painter.clear([size.width, size.height], clear_color);
        painter.paint_and_update_textures(
            [size.width, size.height],
            pixels_per_point,
            &primitives,
            &mut textures_delta,
        );
        self.gl_window.swap_buffers();
        result
    }

    pub(crate) fn handle_event(&mut self, event: &WindowEvent) {
        if let WindowEvent::CursorMoved { position, .. } = event {
            self.cursor_position = Some(*position);
        }
        if matches!(event, WindowEvent::Focused(false)) {
            // 窗口失去焦点时收起右键菜单和其它 egui 弹出层。
            egui::Popup::close_all(&self.context);
            self.context.request_repaint();
        }
        if self.drag_anywhere {
            if let WindowEvent::MouseInput {
                state: ElementState::Pressed,
                button: MouseButton::Left,
                ..
            } = event
            {
                // 交给 winit 事件线程执行平台原生拖动。Linux/Wayland 要求这类
                // 窗口管理器交互由创建窗口的事件循环线程发起。
                let _ = self.proxy.send_event(UserEvent::WindowCommand {
                    window_id: self.window_id(),
                    command: WindowCommand::Drag,
                });
            }
        }
        match event {
            WindowEvent::Resized(size) => {
                self.gl_window.resize(size.width, size.height);
            }
            _ => {}
        }
        let _ = self.state.on_window_event(self.gl_window.window(), event);
        // 实际绘制由渲染线程统一调度；输入事件只请求下一帧，避免在事件处理阶段绘制。
        if !matches!(event, WindowEvent::RedrawRequested) {
            self.context.request_repaint();
        }
    }

    pub(crate) fn destroy(&mut self) {
        if self.destroyed {
            return;
        }
        self.destroyed = true;
        // 必须先在 Painter 所属上下文中销毁 Painter，再释放 OpenGL 上下文。
        self.gl_window.make_current();
        if let Some(mut painter) = self.painter.take() {
            painter.destroy();
        }
        self.gl_window.release_context();
    }
}
