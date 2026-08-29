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
    /// 将旧版内置宠物身份迁移到当前宠物，并保留包选项。
    pub fn normalize_ids(&mut self) {
        let old_kind = self.pet.kind.clone();
        if let Some(new_kind) = migrate_pet_id(&old_kind) {
            self.pet.kind = new_kind.to_string();
        }
        let mut migrated = std::mem::take(&mut self.pet.options);
        for (key, value) in migrated.drain() {
            let key = migrate_pet_option_key(&key);
            self.pet.options.entry(key).or_insert(value);
        }
    }

    /// 翻译。
    pub fn t(&self, key: MessageKey) -> &'static str {
        i18n::t(self.locale, key)
    }
}

fn migrate_pet_id(id: &str) -> Option<&'static str> {
    match id {
        "pet.deskhud.specs" => Some("pet.deskhud.dumpling"),
        "pet.deskhud.tangyuan" => Some("pet.deskhud.dumpling"),
        _ => None,
    }
}

fn migrate_pet_option_key(key: &str) -> String {
    for (old, new) in [
        ("pet.deskhud.specs.", "pet.deskhud.dumpling."),
        ("pet.deskhud.tangyuan.", "pet.deskhud.dumpling."),
    ] {
        if let Some(rest) = key.strip_prefix(old) {
            return format!("{new}{rest}");
        }
    }
    key.to_string()
}

#[cfg(test)]
mod tests {
    use super::{UiPreferences, migrate_pet_id};

    #[test]
    fn migrates_builtin_pet_ids_and_option_prefixes() {
        let mut prefs = UiPreferences::default();
        prefs.pet.kind = "pet.deskhud.specs".into();
        prefs.pet.set_bool("pet.deskhud.specs.key_tips", true);

        prefs.normalize_ids();

        assert_eq!(prefs.pet.kind, "pet.deskhud.dumpling");
        assert_eq!(
            migrate_pet_id("pet.deskhud.specs"),
            Some("pet.deskhud.dumpling")
        );
        assert!(prefs.pet.get_bool("pet.deskhud.dumpling.key_tips", false));
    }
}
