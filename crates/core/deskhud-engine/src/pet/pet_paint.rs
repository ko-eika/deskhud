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
    /// 眼白 RGB。
    pub eye_rgb: [f32; 3],
    /// 呼吸缩放（1.0 = 正常）。
    pub bounce: f32,
    /// 瞳孔相对默认眼心的偏移（逻辑像素，宠窗坐标系）。
    pub pupil_offset: [f32; 2],
    /// 是否绘制眼睛（含随偏移移动的瞳孔）。
    pub draw_eyes: bool,
    /// Eye openness in the inclusive range `0.0..=1.0`.
    ///
    /// `1.0` is fully open and `0.0` is closed. Renderers clamp invalid values so pet behavior
    /// can animate a blink without knowing the active platform backend.
    pub eye_open: f32,
    /// 可选对话气泡文案（壳画在宠上方；短句为宜）。
    pub bubble_text: Option<String>,
    /// 对话气泡外观；默认由宿主跟随 [`crate::PetTheme`]。
    pub bubble_style: PetBubbleStyle,
}

impl Default for PetPaint {
    fn default() -> Self {
        Self {
            body_rgb: [0.25, 0.55, 0.95],
            eye_rgb: [0.98, 0.98, 0.98],
            bounce: 1.0,
            pupil_offset: [0.0, 0.0],
            draw_eyes: true,
            eye_open: 1.0,
            bubble_text: None,
            bubble_style: PetBubbleStyle::FollowTheme,
        }
    }
}
