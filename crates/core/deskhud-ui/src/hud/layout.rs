//! HUD 条目屏幕布局：位置 + 独立宽高缩放。

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// HUD 条目相对基准尺寸的最小缩放因子。
pub const HUD_SIZE_FACTOR_MIN: f32 = 0.5;
/// HUD 条目相对基准尺寸的最大缩放因子。
pub const HUD_SIZE_FACTOR_MAX: f32 = 3.0;

/// 单条 HUD 在某一显示器上的布局。
///
/// 条目有固定逻辑基准尺寸（由内容类型决定）；配置通过 `size` 中的
/// `width` / `height` 比例独立调节。
#[derive(Debug, Clone, PartialEq)]
pub struct HudSlotLayout {
    /// 显示器稳定标识；主屏为 `primary`。
    pub display: String,
    /// 相对 HUD 窗口左缘的物理像素位置。
    pub x: f32,
    /// 相对 HUD 窗口上缘的物理像素位置。
    pub y: f32,
    /// 水平相对基准尺寸的比例（1 = 默认宽度）。
    pub width: f32,
    /// 垂直相对基准尺寸的比例（1 = 默认高度）。
    pub height: f32,
}

#[derive(Serialize, Deserialize)]
struct HudSlotLayoutSerde {
    #[serde(default = "default_display")]
    display: String,
    #[serde(default)]
    position: [f32; 2],
    #[serde(default = "default_size")]
    size: [f32; 2],
}

fn default_size() -> [f32; 2] {
    [1.0, 1.0]
}

impl Serialize for HudSlotLayout {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        HudSlotLayoutSerde {
            display: self.display.clone(),
            position: [self.x, self.y],
            size: [self.width, self.height],
        }
        .serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for HudSlotLayout {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = HudSlotLayoutSerde::deserialize(deserializer)?;
        Ok(Self {
            display: value.display,
            x: value.position[0],
            y: value.position[1],
            width: value.size[0],
            height: value.size[1],
        })
    }
}

fn default_display() -> String {
    "primary".into()
}

impl Default for HudSlotLayout {
    fn default() -> Self {
        Self {
            display: default_display(),
            x: 8.0,
            y: 8.0,
            width: 1.0,
            height: 1.0,
        }
    }
}

impl HudSlotLayout {
    /// Clamps only the pixel position, preserving the caller's size units.
    pub fn clamp_position(mut self) -> Self {
        self.x = self.x.max(0.0);
        self.y = self.y.max(0.0);
        self
    }

    /// 夹紧缩放因子；位置由宿主按活动区和条目尺寸处理。
    pub fn clamp01(mut self) -> Self {
        self.width = self.width.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        self.height = self.height.clamp(HUD_SIZE_FACTOR_MIN, HUD_SIZE_FACTOR_MAX);
        self.x = self.x.max(0.0);
        self.y = self.y.max(0.0);
        self
    }

    /// 默认槽：按索引错落。
    pub fn default_for_index(index: usize) -> Self {
        let i = index as f32;
        Self {
            display: default_display(),
            x: 8.0,
            y: 8.0 + i * 64.0,
            width: 1.0,
            height: 1.0,
        }
        .clamp01()
    }
}
