//! UI 偏好聚合。

use serde::{Deserialize, Serialize};

use crate::hud::HudPrefs;
use crate::i18n::{self, Locale, MessageKey};
use crate::pet::PetPrefs;
use crate::shell::ShellPrefs;

/// 壳偏好。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiPreferences {
    /// 语言。
    #[serde(default)]
    pub locale: Locale,
    /// 桌宠壳（窗体 / 当前宠 id）。
    #[serde(default)]
    pub shell: ShellPrefs,
    /// 宠物包配置 `[pet.config]`。
    #[serde(default)]
    pub pet: PetPrefs,
    /// HUD 配置 `[hud.config]`。
    #[serde(default)]
    pub hud: HudPrefs,
}

impl Default for UiPreferences {
    fn default() -> Self {
        Self {
            locale: Locale::ZhCn,
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

    /// 加载后规范化（旧宠物 id → `pet.<组织>.<标识>`）。
    pub fn normalize_ids(&mut self) {
        self.shell.active_pet_kind_id =
            migrate_pet_id(&self.shell.active_pet_kind_id).to_string();
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
