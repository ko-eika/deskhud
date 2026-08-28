//! 桌宠外壳偏好：界面主题 / 设置窗几何；字体见 `[font]`。

use serde::{Deserialize, Serialize};

/// Default bundled font face used when no user preference exists.
pub const DEFAULT_UI_FONT_ID: &str = "sourcehansans#face=0";
/// Default bundled font family.
pub const DEFAULT_UI_FONT_FAMILY: &str = "sourcehansans";
/// Default bundled font style.
pub const DEFAULT_UI_FONT_STYLE: &str = "Regular";
/// Default UI font size in logical pixels.
pub const DEFAULT_UI_FONT_SIZE: f32 = 13.0;

/// 设置页宠物选择视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PetPickerMode {
    /// 网格卡片。
    #[default]
    Grid,
    /// 列表行。
    List,
}

/// 应用主题偏好。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum UiTheme {
    /// 跟随系统。
    #[default]
    System,
    /// 浅色。
    Light,
    /// 深色。
    Dark,
}

/// 当前系统主题，用于解析 [`UiTheme::System`]。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SystemTheme {
    /// 浅色系统主题。
    Light,
    /// 深色系统主题。
    Dark,
}

/// 将用户偏好解析为实际使用的主题。
pub fn resolve_theme(preference: UiTheme, system: Option<SystemTheme>) -> SystemTheme {
    match preference {
        UiTheme::Light => SystemTheme::Light,
        UiTheme::Dark => SystemTheme::Dark,
        UiTheme::System => system.unwrap_or(SystemTheme::Dark),
    }
}

/// Frame-rate pacing options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum FpsLimit {
    /// Backend-selected pacing.
    #[default]
    Auto,
    /// 30 frames per second.
    Fps30,
    /// 60 frames per second.
    Fps60,
    /// 120 frames per second.
    Fps120,
}
/// Animation detail options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum AnimationQuality {
    /// Reduced animation work.
    Low,
    /// Default animation quality.
    #[default]
    Standard,
    /// Full animation quality.
    High,
}
/// Power and smoothness trade-off options.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PowerMode {
    /// Prefer lower power use.
    Saving,
    /// Balance power and smoothness.
    #[default]
    Balanced,
    /// Prefer smooth motion.
    Smooth,
}

/// 桌面覆盖层的用户层级。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LayerPreference {
    /// 始终位于其它普通窗口之上。
    #[default]
    Top,
    /// 跟随系统普通窗口层级。
    Normal,
    /// 尽量位于其它普通窗口之下。
    Bottom,
}

/// Backend-neutral graphics preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphicsPreferences {
    /// Frame pacing limit.
    #[serde(default)]
    pub fps_limit: FpsLimit,
    /// Animation quality.
    #[serde(default)]
    pub animation_quality: AnimationQuality,
    /// Whether shadows are enabled.
    #[serde(default = "default_true")]
    pub shadows: bool,
    /// Power versus smoothness preference.
    #[serde(default)]
    pub power_mode: PowerMode,
}
fn default_true() -> bool {
    true
}

impl Default for GraphicsPreferences {
    fn default() -> Self {
        Self {
            fps_limit: Default::default(),
            animation_quality: Default::default(),
            shadows: true,
            power_mode: Default::default(),
        }
    }
}

/// 界面与设置窗偏好（落盘 `[theme]` + `[settings]` + `[font]`）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellPrefs {
    /// 具体字面 ID：内置为文件 stem（如 `JetBrainsMono-Regular`）；系统为字体路径（`/` 分隔）。
    /// 与 `ui_font_family` 不同：同一家族可有 Regular/Bold 多个 id。
    #[serde(default = "default_ui_font_id")]
    pub ui_font_id: String,
    /// 字体系列键（规范化小写码，无前缀），用于设置页下拉分组。
    #[serde(default = "default_ui_font_family")]
    pub ui_font_family: String,
    /// 字体样式名（Regular / Bold …），与 family 一起解析出 `ui_font_id`。
    #[serde(default = "default_ui_font_style")]
    pub ui_font_style: String,
    /// 界面字号（逻辑像素）。
    #[serde(default = "default_ui_font_size")]
    pub ui_font_size: f32,
    /// 应用主题。
    #[serde(default)]
    pub ui_theme: UiTheme,
    /// 设置窗宽（逻辑像素）。
    #[serde(default)]
    pub settings_width: Option<f32>,
    /// 设置窗高。
    #[serde(default)]
    pub settings_height: Option<f32>,
    /// 设置窗左上角 X。
    #[serde(default)]
    pub settings_pos_x: Option<f32>,
    /// 设置窗左上角 Y。
    #[serde(default)]
    pub settings_pos_y: Option<f32>,
}

fn default_ui_font_id() -> String {
    DEFAULT_UI_FONT_ID.into()
}

fn default_ui_font_family() -> String {
    DEFAULT_UI_FONT_FAMILY.into()
}

fn default_ui_font_style() -> String {
    DEFAULT_UI_FONT_STYLE.into()
}

fn default_ui_font_size() -> f32 {
    DEFAULT_UI_FONT_SIZE
}

impl Default for ShellPrefs {
    fn default() -> Self {
        Self {
            ui_font_id: default_ui_font_id(),
            ui_font_family: default_ui_font_family(),
            ui_font_style: default_ui_font_style(),
            ui_font_size: default_ui_font_size(),
            ui_theme: UiTheme::default(),
            settings_width: None,
            settings_height: None,
            settings_pos_x: None,
            settings_pos_y: None,
        }
    }
}

impl ShellPrefs {
    /// 设置窗最小内尺寸（16:9）。
    pub const SETTINGS_MIN_W: f32 = 800.0;
    /// 设置窗最小内高度（16:9）。
    pub const SETTINGS_MIN_H: f32 = 450.0;
    /// 设置窗默认宽（16:9）。
    pub const SETTINGS_DEFAULT_W: f32 = 1600.0;
    /// 设置窗默认高（16:9）。
    pub const SETTINGS_DEFAULT_H: f32 = 900.0;

    /// 设置窗默认 / 已存尺寸。
    pub fn settings_size(&self) -> [f32; 2] {
        [
            self.settings_width
                .unwrap_or(Self::SETTINGS_DEFAULT_W)
                .clamp(Self::SETTINGS_MIN_W, 1600.0),
            self.settings_height
                .unwrap_or(Self::SETTINGS_DEFAULT_H)
                .clamp(Self::SETTINGS_MIN_H, 900.0),
        ]
    }

    /// 设置窗已存位置。
    pub fn settings_pos(&self) -> Option<[f32; 2]> {
        Some([self.settings_pos_x?, self.settings_pos_y?])
    }

    /// 写入设置窗几何（尺寸 + 外框位置）。
    pub fn set_settings_geometry(&mut self, width: f32, height: f32, pos_x: f32, pos_y: f32) {
        self.settings_width = Some(width.clamp(Self::SETTINGS_MIN_W, 1600.0));
        self.settings_height = Some(height.clamp(Self::SETTINGS_MIN_H, 900.0));
        self.settings_pos_x = Some(pos_x);
        self.settings_pos_y = Some(pos_y);
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemTheme, UiTheme, resolve_theme};

    #[test]
    fn explicit_theme_overrides_system() {
        assert_eq!(
            resolve_theme(UiTheme::Light, Some(SystemTheme::Dark)),
            SystemTheme::Light
        );
        assert_eq!(
            resolve_theme(UiTheme::Dark, Some(SystemTheme::Light)),
            SystemTheme::Dark
        );
    }

    #[test]
    fn system_theme_has_safe_dark_fallback() {
        assert_eq!(
            resolve_theme(UiTheme::System, Some(SystemTheme::Light)),
            SystemTheme::Light
        );
        assert_eq!(resolve_theme(UiTheme::System, None), SystemTheme::Dark);
    }
}
