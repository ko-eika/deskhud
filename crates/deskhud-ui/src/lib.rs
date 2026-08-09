//! # deskhud-ui
//!
//! 与 egui 无关的壳偏好：i18n（外壳 + 包目录合并）、桌宠 prefs、HUD 开关。
//! 包内 `i18n/*.toml` 的文件形状见 `deskhud-package`；合并见 [`CatalogStore`]。

#![deny(missing_docs)]

pub mod hud;
pub mod i18n;
pub mod persist;
pub mod pet;
pub mod prefs;
pub mod shell;

pub use hud::{HudConfigValue, HudPrefs, HudSlotLayout};
pub use i18n::{
    locale_file_candidates, locale_tag, seed_builtin_packs, CatalogStore, Locale, MessageKey,
};
pub use persist::{
    format_prefs, format_prefs_ordered, load, load_or_default, prefs_path, save, save_ordered,
    PersistError, PrefsWriteOrder,
};
pub use pet::PetPrefs;
pub use prefs::{migrate_pet_id, UiPreferences};
pub use shell::{PetPickerMode, ShellPrefs, UiTheme};
