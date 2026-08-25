//! OpenGL 窗口和绘制资源模块。
//!
//! 该模块保持为独立的底层基础设施，不包含应用视口业务。

mod gl_window;
mod painter;

pub(crate) use gl_window::GlWindow;
