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
    /// 界面 / 设置窗（落盘 `[ui]`）；字体落盘 `[font]`。
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

    /// 加载后规范化（旧宠物 id → `pet.<组织>.<标识>`；字体/全局键迁移）。
    pub fn normalize_ids(&mut self) {
        self.pet.kind = migrate_pet_id(&self.pet.kind).to_string();
        self.shell.ui_font_id = crate::shell::migrate_ui_font_id(&self.shell.ui_font_id);
        self.shell.ui_font_family =
            crate::shell::migrate_ui_font_family(&self.shell.ui_font_family);
        self.hud.migrate_global_keys();
        // 旧 `topmost` 已在 persist 合入 `[settings]`；确保不与 options 里残留键冲突
        self.pet.options.remove("topmost");
        self.pet.options.remove("pet_topmost");
        self.pet
            .options
            .remove(crate::pet::PetPrefs::LEGACY_GLOBAL_TOPMOST_KEY);
    }
}

/// 旧宠物 id 迁移。
pub fn migrate_pet_id(id: &str) -> &str {
    match id {
        "builtin.specs" => "pet.deskhud.specs",
        "builtin.blob" => "pet.deskhud.blob",
        other => other,
    }
}
