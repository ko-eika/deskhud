//! 插件可声明的 HUD 条目。

/// 一条可配置的 HUD 贡献。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudContribution {
    /// 条目 ID（**插件内**唯一短名），如 `clock`；prefs 键为 `{plugin_id}.{id}.enable`。
    pub id: &'static str,
    /// 设置页显示名。
    pub label: &'static str,
    /// 默认是否开启。
    pub default_enabled: bool,
    /// 条目图标字节（svg/png/jpeg/gif/webp）；与插件一并打包。缺省时壳用默认图标。
    pub icon: Option<&'static [u8]>,
}

/// A platform-independent HUD frame produced by a plugin.
#[derive(Debug, Clone, PartialEq)]
pub struct HudFrame {
    /// Visuals in back-to-front order.
    pub visuals: Vec<HudVisual>,
}

impl HudFrame {
    /// Creates an empty frame.
    pub fn empty() -> Self {
        Self {
            visuals: Vec::new(),
        }
    }

    /// Returns whether the frame contains drawable content.
    pub fn is_empty(&self) -> bool {
        self.visuals.is_empty()
    }
}

/// Platform-independent HUD visual.
#[derive(Debug, Clone, PartialEq)]
pub enum HudVisual {
    /// A single line of text in logical HUD coordinates.
    Text {
        /// Text content.
        text: String,
        /// Logical font size.
        font_size: f32,
        /// RGBA color.
        color: [u8; 4],
    },
    /// A filled rounded rectangle.
    Panel {
        /// Width in logical pixels.
        width: f32,
        /// Height in logical pixels.
        height: f32,
        /// Corner radius in logical pixels.
        radius: f32,
        /// RGBA color.
        color: [u8; 4],
    },
}
