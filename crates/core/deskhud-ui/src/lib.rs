//! # deskhud-ui
//!
//! 与 egui 无关的壳偏好：i18n（外壳 + 包目录合并）、桌宠 prefs、HUD 开关。
//! 包内 `i18n/<locale>/info.po` / `config.po`（发布为 `.mo`）的文件形状见 `deskhud-package`；合并见 [`CatalogStore`]。

#![deny(missing_docs)]

pub mod font;
pub mod hud;
pub mod i18n;
pub mod persist;
pub mod pet;
pub mod prefs;
pub mod settings;
pub mod shell;
pub mod system_locale;

pub use font::{FontFace, FontFamilyEntry, FontSelection};
pub use hud::{
    HUD_SIZE_FACTOR_MAX, HUD_SIZE_FACTOR_MIN, HudConfigValue, HudGroup, HudInstance,
    HudInstanceConfig, HudPrefs, HudRecoveryReport, HudSlotLayout,
};
pub use i18n::{
    CatalogStore, Locale, MessageKey, locale_file_candidates, locale_tag, normalize_locale_tag,
    seed_builtin_packs,
};
pub use persist::{
    PersistError, PrefsWriteOrder, format_prefs, format_prefs_ordered, load, load_or_default,
    prefs_path, save, save_ordered,
};
pub use pet::PetPrefs;
pub use pet::{PetPosition, PetSize};
pub use prefs::UiPreferences;
pub use settings::{
    AboutInfo, PetCardLayout, SettingsCommand, SettingsEffect, SettingsModel, SettingsTab,
    apply_general_preferences, apply_graphics_preferences, apply_pet_selection, draft_is_dirty,
    pet_card_layout, pet_card_layout_with_font,
};
pub use shell::{
    AnimationQuality, DEFAULT_UI_FONT_FAMILY, DEFAULT_UI_FONT_ID, DEFAULT_UI_FONT_SIZE,
    DEFAULT_UI_FONT_STYLE, FpsLimit, GraphicsPreferences, LayerPreference, PetPickerMode,
    PowerMode, ShellPrefs, SystemTheme, UiTheme, resolve_theme,
};
pub use system_locale::{LanguageTag, current_system_locale};
