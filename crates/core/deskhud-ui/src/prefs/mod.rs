//! UI 偏好聚合。

use crate::hud::HudPrefs;
use crate::i18n::{self, Locale, MessageKey};
use crate::pet::PetPrefs;
use crate::shell::ShellPrefs;

/// 壳偏好（内存模型；落盘形状见 `persist`）。
#[derive(Debug, Clone, PartialEq)]
pub struct UiPreferences {
    /// Rendering and animation preferences.
    pub graphics: crate::shell::GraphicsPreferences,
    /// 语言。
    pub locale: Locale,
    /// 界面 / 设置窗（落盘 `[theme]` / `[prefs]`）；字体落盘 `[font]`。
    pub shell: ShellPrefs,
    /// 宠物窗体 + 包选项（落盘 `[pet]`）。
    pub pet: PetPrefs,
    /// HUD（落盘 `[hud]`）。
    pub hud: HudPrefs,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            graphics: Default::default(),
            locale: Locale::System,
            shell: ShellPrefs::default(),
            pet: PetPrefs::default(),
            hud: HudPrefs::default(),
        }
    }
}

impl UiPreferences {
    /// 翻译。
    pub fn t(&self, key: MessageKey) -> &'static str {
        i18n::t(self.locale, key)
    }
}
