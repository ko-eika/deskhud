//! 各个应用视口的原生窗口配置。

use egui::ViewportId;

/// 单个视口对应的原生窗口配置。
pub(crate) struct ViewportConfig {
    /// 原生窗口标题。
    pub title: &'static str,
    /// 初始逻辑尺寸 `[宽度, 高度]`。
    pub size: [f64; 2],
    /// Minimum logical client size, if the viewport is resizable.
    pub min_size: Option<[f64; 2]>,
    /// egui 使用的视口标识。
    pub egui_id: ViewportId,
    /// 是否显示系统边框和标题栏。
    pub decorations: bool,
    /// 是否启用透明 framebuffer。
    pub transparent: bool,
    /// 是否允许用户调整窗口大小。
    pub resizable: bool,
    /// 是否允许通过拖动任意区域移动窗口。
    pub drag_anywhere: bool,
    /// 是否从任务栏隐藏窗口。
    pub skip_taskbar: bool,
    /// 创建时是否立即显示窗口。
    pub visible: bool,
    /// 是否创建时置顶。
    pub always_on_top: bool,
    /// 是否在创建视口时立即配置字体；隐藏的大视口可延迟到首次绘制。
    pub configure_fonts: bool,
    /// Windows 无边框窗口是否保留系统阴影。
    pub undecorated_shadow: bool,
    pub x11_popup: bool,
}

impl ViewportConfig {
    /// 返回 Pet 主视口的配置。
    pub(crate) fn pet() -> Self {
        Self {
            title: "Pet",
            size: [160.0, 160.0],
            min_size: None,
            egui_id: ViewportId::ROOT,
            decorations: false,
            transparent: true,
            resizable: false,
            drag_anywhere: true,
            skip_taskbar: true,
            visible: false,
            always_on_top: true,
            configure_fonts: false,
            undecorated_shadow: false,
            x11_popup: false,
        }
    }

    /// 返回 HUD 视口的配置。
    pub(crate) fn hud() -> Self {
        Self {
            title: "HUD",
            size: [360.0, 180.0],
            min_size: None,
            egui_id: ViewportId::from_hash_of("hud"),
            decorations: false,
            transparent: true,
            resizable: false,
            drag_anywhere: false,
            skip_taskbar: true,
            visible: false,
            always_on_top: true,
            configure_fonts: false,
            undecorated_shadow: false,
            x11_popup: false,
        }
    }

    /// 返回 Settings 视口的配置。
    pub(crate) fn settings() -> Self {
        Self {
            title: "Settings",
            size: [1600.0, 900.0],
            min_size: Some([800.0, 450.0]),
            egui_id: ViewportId::from_hash_of("setting"),
            decorations: true,
            transparent: false,
            resizable: true,
            drag_anywhere: false,
            skip_taskbar: false,
            visible: false,
            always_on_top: false,
            configure_fonts: false,
            undecorated_shadow: false,
            x11_popup: false,
        }
    }
}
