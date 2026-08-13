//! 桌宠外壳偏好：界面主题 / 设置窗几何（`[ui]`）；字体见 `[font]`。

use serde::{Deserialize, Serialize};

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
/// Backend-neutral graphics preferences.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphicsPreferences {
    /// Frame pacing limit.
    #[serde(default)]
    pub fps_limit: FpsLimit,
    /// Animation quality.
    #[serde(default)]
    pub animation_quality: AnimationQuality,
    /// Whether bubbles and effects are enabled.
    #[serde(default = "default_true")]
    pub effects: bool,
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
            effects: true,
            power_mode: Default::default(),
        }
    }
}

/// 界面与设置窗偏好（落盘 `[theme]` + `[settings]` + `[font]`；旧文件 `[ui]` / `[shell]` 可迁移）。
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
    /// 桌面覆盖层置顶（宠 / HUD；菜单打开时临时在宠物之上；设置窗不置顶）。
    #[serde(default = "default_topmost")]
    pub topmost: bool,
}

fn default_ui_font_id() -> String {
    "JetBrainsMono-Regular".into()
}

fn default_ui_font_family() -> String {
    "jetbrainsmono".into()
}

/// 去掉历史 `builtin.` / `system.` 前缀；旧短名映射到 stem。
pub fn migrate_ui_font_id(id: &str) -> String {
    match id {
        "builtin.noto_sans_sc" | "noto_sans_sc" => "NotoSansSC-Regular".into(),
        "builtin.jetbrains_mono" | "jetbrains_mono" => "JetBrainsMono-Regular".into(),
        other => {
            if let Some(rest) = other.strip_prefix("builtin.") {
                return rest.to_string();
            }
            if let Some(rest) = other.strip_prefix("system.") {
                return rest.to_string();
            }
            other.to_string()
        }
    }
}

/// 去掉历史 `fam.` 前缀；旧家族短名映射。
pub fn migrate_ui_font_family(key: &str) -> String {
    let key = key.strip_prefix("fam.").unwrap_or(key);
    match key {
        "builtin.noto_sans_sc" | "noto_sans_sc" => "notosanssc".into(),
        "builtin.jetbrains_mono" | "jetbrains_mono" => "jetbrainsmono".into(),
        other => other.to_string(),
    }
}

fn default_ui_font_style() -> String {
    "Regular".into()
}

fn default_ui_font_size() -> f32 {
    13.0
}

fn default_topmost() -> bool {
    true
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
            topmost: default_topmost(),
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
