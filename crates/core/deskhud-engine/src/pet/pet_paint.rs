//! 框架无关的一帧宠物外观。

/// 宠物请求的对话气泡外观。
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum PetBubbleStyle {
    /// 交由宿主按 [`crate::PetTheme`] 选择高对比度的默认配色。
    #[default]
    FollowTheme,
    /// 使用宠物包提供的颜色与圆角；每个颜色分量均为 `0.0..=1.0`。
    Custom {
        /// 气泡背景 RGBA。
        background_rgba: [f32; 4],
        /// 文本 RGBA。
        text_rgba: [f32; 4],
        /// 气泡圆角半径（逻辑像素）。
        corner_radius: f32,
    },
}

/// UI 壳负责落到具体渲染器。
#[derive(Debug, Clone, PartialEq)]
pub struct PetPaint {
    /// 身体 RGB 0..=1。
    pub body_rgb: [f32; 3],
    /// 可选对话气泡文案（壳画在宠上方；短句为宜）。
    pub bubble_text: Option<String>,
    /// 对话气泡外观；默认由宿主跟随 [`crate::PetTheme`]。
    pub bubble_style: PetBubbleStyle,
}

impl Default for PetPaint {
    fn default() -> Self {
        Self {
            body_rgb: [0.25, 0.55, 0.95],
            bubble_text: None,
            bubble_style: PetBubbleStyle::FollowTheme,
        }
    }
}
