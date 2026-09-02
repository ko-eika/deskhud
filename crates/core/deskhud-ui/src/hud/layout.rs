//! HUD 条目屏幕布局：位置 + 独立宽高缩放。

use serde::{Deserialize, Serialize};

/// HUD 条目相对基准尺寸的最小缩放因子。
pub const HUD_SIZE_FACTOR_MIN: f32 = 0.5;
/// HUD 条目相对基准尺寸的最大缩放因子。
pub const HUD_SIZE_FACTOR_MAX: f32 = 3.0;

/// 单条 HUD 在某一显示器上的布局。
///
/// 条目有固定逻辑基准尺寸（由内容类型决定）；配置通过 `width` / `height`
/// 独立调节。
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
    /// 水平相对基准尺寸的比例（1 = 默认宽度）。
    #[serde(default = "default_size_factor")]
    pub width: f32,
    /// 垂直相对基准尺寸的比例（1 = 默认高度）。
    #[serde(default = "default_size_factor")]
    pub height: f32,
}

fn default_size_factor() -> f32 {
    1.0
}

fn default_display() -> String {
    "primary".into()
}

impl Default for HudSlotLayout {
    fn default() -> Self {
        Self {
            display: default_display(),
            x: 0.02,
            y: 0.04,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl HudSlotLayout {
    /// 夹紧位置与缩放（位置允许贴边 0..1；具体是否越界由宿主按条目尺寸再夹）。
    pub fn clamp01(mut self) -> Self {
        self.width = self.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        self.height = self.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
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
            width: 1.0,
            height: 1.0,
        }
        .clamp01()
    }
}
