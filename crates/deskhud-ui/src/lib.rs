//! # deskhud-ui
//!
//! 与 egui 无关的壳偏好：i18n（外壳 + 日后合并包目录）、桌宠 prefs、HUD 开关。
//! 包内 `i18n/*.toml` 的文件形状见 `deskhud-package`；合并引擎在 Phase 2 落地。

#![deny(missing_docs)]

pub mod hud;
pub mod i18n;
pub mod persist;
pub mod pet;
pub mod prefs;
pub mod shell;

pub use hud::HudPrefs;
pub use i18n::{Locale, MessageKey};
pub use persist::{load, load_or_default, prefs_path, save, PersistError};
pub use pet::PetPrefs;
pub use prefs::{migrate_pet_id, UiPreferences};
pub use shell::{PetPickerMode, ShellPrefs};
