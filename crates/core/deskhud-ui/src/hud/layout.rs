//! HUD 条目屏幕布局：位置 + 统一缩放（固定内容比例）。

use serde::{Deserialize, Serialize};

/// 单条 HUD 在某一显示器上的布局。
///
/// 条目有固定逻辑基准尺寸（由内容类型决定）；`scale` 只做等比缩放。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudSlotLayout {
    /// 显示器稳定标识；主屏为 `primary`。
    #[serde(default = "default_display")]
    pub display: String,
    /// 相对该屏宽的左缘（0..1）。
    #[serde(default)]
    pub x: f32,
    /// 相对该屏高的上缘（0..1）。
    #[serde(default)]
    pub y: f32,
    /// 相对基准尺寸的缩放（1 = 默认）。
    #[serde(default = "default_scale")]
    pub scale: f32,
}

fn default_display() -> String {
    "primary".into()
}

fn default_scale() -> f32 {
    1.0
}

impl Default for HudSlotLayout {
    fn default() -> Self {
        Self {
            display: default_display(),
            x: 0.02,
            y: 0.04,
            scale: 1.0,
        }
    }
}

impl HudSlotLayout {
    /// 夹紧位置与缩放（位置允许贴边 0..1；具体是否越界由宿主按条目尺寸再夹）。
    pub fn clamp01(mut self) -> Self {
        self.scale = self.scale.clamp(0.5, 3.0);
        self.x = self.x.clamp(0.0, 1.0);
        self.y = self.y.clamp(0.0, 1.0);
        self
    }

    /// 默认槽：按索引错落。
    pub fn default_for_index(index: usize) -> Self {
        let i = index as f32;
        Self {
            display: default_display(),
            x: 0.02,
            y: 0.04 + i * 0.05,
            scale: 1.0,
        }
        .clamp01()
    }
}
