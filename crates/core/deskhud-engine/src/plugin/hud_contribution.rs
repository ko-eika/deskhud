//! 插件可声明的 HUD 条目。

use serde::{Deserialize, Serialize};

/// A neutral persisted value supplied to one HUD instance.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HudConfigValue {
    /// Boolean value.
    Bool(bool),
    /// Integer value.
    Int(i64),
    /// Floating-point value.
    Float(f64),
    /// Logical position pair.
    Position([f64; 2]),
    /// Logical size pair.
    Size([f64; 2]),
    /// UTF-8 text value.
    String(String),
}

impl HudConfigValue {
    /// Returns this value as a boolean when possible.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }
    /// Returns this value as a finite-compatible number when possible.
    pub fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float(value) => Some(*value as f32),
            Self::Int(value) => Some(*value as f32),
            _ => None,
        }
    }
    /// Returns this value as text when possible.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }
}

/// A choice offered by a neutral HUD configuration declaration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudConfigChoice {
    /// Stable persisted value.
    pub value: &'static str,
    /// User-facing fallback label.
    pub label: &'static str,
}

/// An owned choice produced dynamically by a native HUD plugin.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudConfigDynamicChoice {
    /// Stable persisted value.
    pub value: String,
    /// User-facing label.
    pub label: String,
}

/// Host-renderable configuration control for a HUD contribution.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum HudConfigKind {
    /// Boolean switch and its default.
    Bool {
        /// Value used when the instance has no override.
        default: bool,
    },
    /// Bounded numeric editor.
    Number {
        /// Value used when the instance has no override.
        default: f32,
        /// Inclusive lower bound.
        min: f32,
        /// Inclusive upper bound.
        max: f32,
        /// Suggested editor increment.
        step: f32,
    },
    /// Bounded single-line text editor.
    Text {
        /// Value used when the instance has no override.
        default: &'static str,
        /// Maximum Unicode scalar count accepted by the host.
        max_len: usize,
    },
    /// Selection from a fixed set of choices.
    Choice {
        /// Value used when the instance has no override.
        default: &'static str,
        /// Complete set of valid persisted choices.
        choices: &'static [HudConfigChoice],
    },
    /// Searchable selection whose choices are supplied by the plugin at runtime.
    DynamicChoice {
        /// Value used when the instance has no override.
        default: &'static str,
    },
}

/// One instance-owned configuration declaration.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudConfigOption {
    /// Stable instance configuration key.
    pub key: &'static str,
    /// User-facing fallback label.
    pub label: &'static str,
    /// User-facing fallback description.
    pub description: &'static str,
    /// Control shape, validation limits and default.
    pub kind: HudConfigKind,
}

/// 一条可配置的 HUD 贡献。
#[derive(Debug, Clone, PartialEq)]
pub struct HudContribution {
    /// 条目 ID（**插件内**唯一短名），如 `clock`；prefs 键为 `{plugin_id}.{id}.enable`。
    pub id: &'static str,
    /// 设置页显示名。
    pub label: &'static str,
    /// 默认是否开启。
    pub default_enabled: bool,
    /// 条目图标字节（svg/png/jpeg/gif/webp）；与插件一并打包。缺省时壳用默认图标。
    pub icon: Option<&'static [u8]>,
    /// Generic host-rendered settings owned by each stable instance.
    pub config: &'static [HudConfigOption],
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
    /// Text anchored at a logical point inside the HUD frame.
    Label {
        /// Text content.
        text: String,
        /// Horizontal anchor in logical pixels.
        x: f32,
        /// Vertical anchor in logical pixels.
        y: f32,
        /// Alignment around the anchor point.
        align: HudTextAlign,
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
    /// A bounded progress bar positioned inside the HUD frame.
    ProgressBar {
        /// Left edge in logical pixels.
        x: f32,
        /// Top edge in logical pixels.
        y: f32,
        /// Logical width.
        width: f32,
        /// Logical height.
        height: f32,
        /// Corner radius.
        radius: f32,
        /// Filled fraction in `[0, 1]`.
        value: f32,
        /// Track RGBA color.
        background: [u8; 4],
        /// Fill RGBA color.
        fill: [u8; 4],
    },
    /// A bounded history polyline positioned inside the HUD frame.
    LineChart {
        /// Left edge in logical pixels.
        x: f32,
        /// Top edge in logical pixels.
        y: f32,
        /// Logical width.
        width: f32,
        /// Logical height.
        height: f32,
        /// Oldest-to-newest bounded samples.
        values: Vec<f32>,
        /// Lower value bound.
        min: f32,
        /// Upper value bound.
        max: f32,
        /// Logical stroke width.
        stroke_width: f32,
        /// RGBA stroke color.
        color: [u8; 4],
    },
}

/// Alignment for an anchored HUD label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HudTextAlign {
    /// The anchor is at the label's left edge.
    Left,
    /// The anchor is at the label's horizontal center.
    Center,
    /// The anchor is at the label's right edge.
    Right,
}

impl HudVisual {
    /// Maximum samples accepted by a history visual.
    pub const MAX_HISTORY_POINTS: usize = 240;

    /// Creates an anchored label with finite logical coordinates.
    pub fn label(
        text: impl Into<String>,
        x: f32,
        y: f32,
        align: HudTextAlign,
        font_size: f32,
        color: [u8; 4],
    ) -> Self {
        Self::Label {
            text: text.into(),
            x: finite(x, 0.0),
            y: finite(y, 0.0),
            align,
            font_size: finite(font_size, 14.0).clamp(1.0, 512.0),
            color,
        }
    }

    /// Creates a finite progress bar whose value is clamped to `[0, 1]`.
    #[allow(clippy::too_many_arguments)]
    pub fn progress_bar(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        value: f32,
        background: [u8; 4],
        fill: [u8; 4],
    ) -> Self {
        Self::ProgressBar {
            x: finite(x, 0.0),
            y: finite(y, 0.0),
            width: finite(width, 1.0).clamp(1.0, 16_384.0),
            height: finite(height, 1.0).clamp(1.0, 16_384.0),
            radius: finite(radius, 0.0).clamp(0.0, 8_192.0),
            value: finite(value, 0.0).clamp(0.0, 1.0),
            background,
            fill,
        }
    }

    /// Creates a finite, bounded history chart safe for every renderer.
    #[allow(clippy::too_many_arguments)]
    pub fn line_chart(
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        values: impl IntoIterator<Item = f32>,
        min: f32,
        max: f32,
        stroke_width: f32,
        color: [u8; 4],
    ) -> Self {
        let min = finite(min, 0.0);
        let max = finite(max, min + 1.0).max(min + f32::EPSILON);
        let values = values
            .into_iter()
            .take(Self::MAX_HISTORY_POINTS)
            .map(|value| finite(value, min).clamp(min, max))
            .collect();
        Self::LineChart {
            x: finite(x, 0.0),
            y: finite(y, 0.0),
            width: finite(width, 1.0).clamp(1.0, 16_384.0),
            height: finite(height, 1.0).clamp(1.0, 16_384.0),
            values,
            min,
            max,
            stroke_width: finite(stroke_width, 1.0).clamp(0.5, 32.0),
            color,
        }
    }
}

fn finite(value: f32, fallback: f32) -> f32 {
    if value.is_finite() { value } else { fallback }
}

#[cfg(test)]
mod tests {
    use super::HudVisual;

    #[test]
    fn history_visual_bounds_untrusted_samples() {
        let values = std::iter::repeat_n(f32::INFINITY, HudVisual::MAX_HISTORY_POINTS + 20);
        let HudVisual::LineChart {
            values, min, max, ..
        } = HudVisual::line_chart(0.0, 0.0, 100.0, 20.0, values, 0.0, 100.0, 2.0, [255; 4])
        else {
            panic!("expected line chart")
        };
        assert_eq!(values.len(), HudVisual::MAX_HISTORY_POINTS);
        assert!(values.iter().all(|value| *value == min));
        assert_eq!(max, 100.0);
    }

    #[test]
    fn progress_visual_clamps_fraction() {
        let HudVisual::ProgressBar { value, .. } =
            HudVisual::progress_bar(0.0, 0.0, 100.0, 8.0, 4.0, 9.0, [0; 4], [255; 4])
        else {
            panic!("expected progress bar")
        };
        assert_eq!(value, 1.0);
    }
}
