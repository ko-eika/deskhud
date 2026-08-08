//! 框架无关的一帧宠物外观。

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
    /// 可选对话气泡文案（壳画在宠上方；短句为宜）。
    pub bubble_text: Option<String>,
}

impl Default for PetPaint {
    fn default() -> Self {
        Self {
            body_rgb: [0.25, 0.55, 0.95],
            eye_rgb: [0.98, 0.98, 0.98],
            bounce: 1.0,
            pupil_offset: [0.0, 0.0],
            draw_eyes: true,
            bubble_text: None,
        }
    }
}
