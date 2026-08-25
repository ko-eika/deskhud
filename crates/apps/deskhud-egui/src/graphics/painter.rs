//! egui OpenGL Painter 创建逻辑。

use std::{ffi::CString, sync::Arc};

use egui_glow::{Painter, glow};
use glutin::display::GlDisplay as _;

use super::GlWindow;

impl GlWindow {
    /// 根据当前 OpenGL Display 创建 egui 绘制器。
    pub(crate) fn create_painter(&self) -> Painter {
        let display = &self.display;
        let gl = Arc::new(unsafe {
            glow::Context::from_loader_function(|function_name| {
                let function_name = CString::new(function_name).expect("OpenGL 函数名中包含空字节");
                display.get_proc_address(&function_name)
            })
        });
        Painter::new(gl, "", None, false).expect("初始化 egui OpenGL Painter 失败")
    }
}
