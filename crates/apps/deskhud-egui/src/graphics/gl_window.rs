//! 原生窗口和 OpenGL 上下文生命周期。
#![allow(clippy::too_many_arguments)]

use glutin::{
    config::ConfigTemplateBuilder,
    context::{ContextApi, ContextAttributesBuilder, NotCurrentContext, PossiblyCurrentContext},
    display::{Display, GetGlDisplay as _, GlDisplay as _},
    prelude::*,
    surface::{Surface, SurfaceAttributesBuilder, WindowSurface},
};
use std::{num::NonZeroU32, sync::Arc};
#[cfg(target_os = "windows")]
use winit::platform::windows::WindowAttributesExtWindows;
#[cfg(target_os = "linux")]
use winit::platform::x11::WindowAttributesExtX11;
use winit::window::Icon;
use winit::{event_loop::ActiveEventLoop, raw_window_handle::HasWindowHandle as _, window::Window};

/// 保存窗口与 OpenGL 资源。Painter 使用 OpenGL，因此这些资源必须比 Painter 活得更久。
pub(crate) struct GlWindow {
    window: Arc<Window>,
    not_current: Option<NotCurrentContext>,
    current: Option<PossiblyCurrentContext>,
    pub(super) display: Display,
    surface: Surface<WindowSurface>,
}

// Context 在 winit 线程创建但保持未激活状态，转移到专用渲染线程后才设为 current。
// 这是跨线程传递 GlWindow 的前提；实际绘制仍只能发生在渲染线程。
unsafe impl Send for GlWindow {}

impl GlWindow {
    /// 使用指定标题和尺寸创建一个 OpenGL 窗口。
    pub(crate) unsafe fn new_with_title(
        event_loop: &ActiveEventLoop,
        title: &str,
        size: [f64; 2],
        min_size: Option<[f64; 2]>,
        decorations: bool,
        transparent: bool,
        resizable: bool,
        skip_taskbar: bool,
        visible: bool,
        undecorated_shadow: bool,
        x11_popup: bool,
        share_context: Option<&PossiblyCurrentContext>,
    ) -> Self {
        let window_attributes = Window::default_attributes()
            .with_title(title)
            .with_inner_size(winit::dpi::LogicalSize::new(size[0], size[1]))
            .with_window_icon(load_app_icon())
            .with_transparent(transparent)
            .with_decorations(decorations)
            .with_visible(visible)
            .with_resizable(resizable);
        let window_attributes = if let Some(min_size) = min_size {
            window_attributes
                .with_min_inner_size(winit::dpi::LogicalSize::new(min_size[0], min_size[1]))
        } else {
            window_attributes
        };
        #[cfg(target_os = "windows")]
        let window_attributes = window_attributes
            .with_skip_taskbar(skip_taskbar)
            .with_undecorated_shadow(undecorated_shadow);
        #[cfg(not(target_os = "windows"))]
        let _ = (skip_taskbar, undecorated_shadow);
        #[cfg(target_os = "linux")]
        let window_attributes = window_attributes.with_override_redirect(x11_popup);
        #[cfg(not(target_os = "linux"))]
        let _ = x11_popup;

        let config_template = ConfigTemplateBuilder::new()
            .prefer_hardware_accelerated(Some(true))
            .with_depth_size(0)
            .with_stencil_size(0)
            .with_transparency(transparent);
        let (window, gl_config) = glutin_winit::DisplayBuilder::new()
            .with_preference(glutin_winit::ApiPreference::FallbackEgl)
            .with_window_attributes(Some(window_attributes.clone()))
            .build(event_loop, config_template, |mut configs| {
                configs.next().expect("没有找到可用的 OpenGL 配置")
            })
            .expect("创建 OpenGL Display 失败");

        let display = gl_config.display();
        let raw_window_handle = window
            .as_ref()
            .map(|window| window.window_handle().expect("获取窗口句柄失败").as_raw());
        let mut context_builder = ContextAttributesBuilder::new();
        let mut fallback_context_builder =
            ContextAttributesBuilder::new().with_context_api(ContextApi::Gles(None));
        if let Some(share_context) = share_context {
            context_builder = context_builder.with_sharing(share_context);
            fallback_context_builder = fallback_context_builder.with_sharing(share_context);
        }
        let context_attributes = context_builder.build(raw_window_handle);
        let fallback_context_attributes = fallback_context_builder.build(raw_window_handle);
        let not_current_context = unsafe {
            display
                .create_context(&gl_config, &context_attributes)
                .or_else(|_| display.create_context(&gl_config, &fallback_context_attributes))
        }
        .expect("创建 OpenGL Context 失败");
        let window = window.unwrap_or_else(|| {
            glutin_winit::finalize_window(event_loop, window_attributes, &gl_config)
                .expect("创建窗口失败")
        });

        #[cfg(target_os = "macos")]
        disable_macos_window_tiling(&window);

        // Hidden secondary windows keep their native geometry, but do not need
        // a full-size framebuffer until their first visible frame.
        let size = if visible {
            window.inner_size()
        } else {
            winit::dpi::PhysicalSize::new(1, 1)
        };
        let width = NonZeroU32::new(size.width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(size.height).unwrap_or(NonZeroU32::MIN);
        let surface_attributes = SurfaceAttributesBuilder::<WindowSurface>::new().build(
            window.window_handle().expect("获取窗口句柄失败").as_raw(),
            width,
            height,
        );
        let surface = unsafe { display.create_window_surface(&gl_config, &surface_attributes) }
            .expect("创建 OpenGL Surface 失败");
        Self {
            window: Arc::new(window),
            not_current: Some(not_current_context),
            current: None,
            display,
            surface,
        }
    }

    pub(crate) fn window(&self) -> &Window {
        &self.window
    }

    /// 返回可发送给 winit 线程的共享窗口句柄。
    pub(crate) fn window_handle(&self) -> Arc<Window> {
        self.window.clone()
    }

    /// 更新 OpenGL Surface 的物理像素尺寸。
    ///
    /// 宽高为 0 时使用 1 像素占位；最小化窗口可能产生这种尺寸，真正绘制会由
    /// `Viewport::render` 跳过，恢复有效尺寸后再继续。
    pub(crate) fn resize(&self, width: u32, height: u32) -> bool {
        let width = NonZeroU32::new(width).unwrap_or(NonZeroU32::MIN);
        let height = NonZeroU32::new(height).unwrap_or(NonZeroU32::MIN);
        if let Some(context) = &self.current {
            self.surface.resize(context, width, height);
            true
        } else {
            false
        }
    }

    /// 交换前后缓冲区，将当前帧提交到原生窗口。
    pub(crate) fn swap_buffers(&self) {
        self.surface
            .swap_buffers(self.current.as_ref().expect("OpenGL Context 未激活"))
            .expect("交换 OpenGL buffer 失败");
    }

    /// 多个视口共享同一线程时，绘制前必须切回当前窗口自己的 OpenGL Context。
    pub(crate) fn make_current(&mut self) {
        if self
            .current
            .as_ref()
            .is_some_and(glutin::context::PossiblyCurrentContext::is_current)
        {
            return;
        }
        if let Some(context) = self.current.take() {
            self.not_current = Some(
                context
                    .make_not_current()
                    .expect("释放旧的 OpenGL Context 失败"),
            );
        }
        let context = self
            .not_current
            .take()
            .expect("OpenGL Context 已被释放")
            .make_current(&self.surface)
            .expect("切换 OpenGL Context 失败");
        self.current = Some(context);
    }

    /// 销毁窗口前释放当前线程上的 OpenGL Context。
    pub(crate) fn release_context(&mut self) {
        if let Some(context) = self.current.take() {
            context
                .make_not_current_in_place()
                .expect("释放 OpenGL Context 失败");
        }
    }
}

/// 加载跨平台窗口图标。PNG 在编译时嵌入，避免运行时依赖工作目录中的资源文件。
fn load_app_icon() -> Option<Icon> {
    let image = image::load_from_memory(include_bytes!("../../../../../assets/icon.png"))
        .ok()?
        .into_rgba8();
    let (width, height) = image.dimensions();
    Icon::from_rgba(image.into_raw(), width, height).ok()
}

#[cfg(target_os = "macos")]
fn disable_macos_window_tiling(window: &Window) {
    // winit 0.30 没有暴露 NSWindow.collectionBehavior；这里通过 AppKit
    // 的公开属性禁止窗口参与全屏平铺。普通的 Sequoia 边缘拖拽仍由
    // Window::drag_window 负责，以保持 macOS 原生拖拽的平滑性。
    use core::{
        ffi::{c_char, c_void},
        mem,
    };
    use winit::raw_window_handle::{HasWindowHandle, RawWindowHandle};

    type Id = *mut c_void;
    type Sel = *mut c_void;

    #[link(name = "AppKit", kind = "framework")]
    #[link(name = "objc")]
    unsafe extern "C" {
        fn objc_msgSend();
        fn sel_registerName(name: *const c_char) -> Sel;
    }

    unsafe {
        let RawWindowHandle::AppKit(handle) =
            window.window_handle().expect("获取窗口句柄失败").as_raw()
        else {
            return;
        };
        let send_id: unsafe extern "C" fn(Id, Sel) -> Id =
            mem::transmute(objc_msgSend as *const ());
        let send_ulong: unsafe extern "C" fn(Id, Sel) -> usize =
            mem::transmute(objc_msgSend as *const ());
        let send_set: unsafe extern "C" fn(Id, Sel, usize) =
            mem::transmute(objc_msgSend as *const ());
        let view = handle.ns_view.as_ptr();
        let window = send_id(view, sel_registerName(c"window".as_ptr()));
        if window.is_null() {
            return;
        }
        let behavior = send_ulong(window, sel_registerName(c"collectionBehavior".as_ptr()));
        // NSWindowCollectionBehaviorFullScreenDisallowsTiling = 1 << 12.
        send_set(
            window,
            sel_registerName(c"setCollectionBehavior:".as_ptr()),
            behavior | (1usize << 12),
        );
    }
}
