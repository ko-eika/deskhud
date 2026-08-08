use crate::i18n::MessageKey;

pub(super) fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppName => "DeskHud",
        MessageKey::ActionClose => "Close",
        MessageKey::ActionCancel => "Cancel",
        MessageKey::ActionApply => "Apply",
        MessageKey::ActionReset => "Reset",
        MessageKey::MenuQuit => "Quit",
        MessageKey::MenuSettings => "Settings",
        MessageKey::SettingsTitle => "Settings",
        MessageKey::SettingsNavPet => "Pet",
        MessageKey::SettingsNavHud => "Plugins",
        MessageKey::SettingsNavGeneral => "General",
        MessageKey::SettingsPetIntro => "Choose a pet; click Apply to take effect. Window size follows the skin.",
        MessageKey::SettingsPetWindowSize => "Window",
        MessageKey::SettingsPetSelected => "In use",
        MessageKey::SettingsPetViewGrid => "Grid",
        MessageKey::SettingsPetViewList => "List",
        MessageKey::SettingsPetOptions => "Active pet behavior",
        MessageKey::HudSettingsIntro => "Grouped by plugin: master switch for the pack, expand for each contribution.",
        MessageKey::HudSettingsEmpty => "No plugin contributions available.",
        MessageKey::MetaAuthor => "Author",
        MessageKey::HudItemsEnabled => "enabled",
        MessageKey::MetaHomepage => "Homepage",
        MessageKey::SettingsLocale => "Language",
        MessageKey::SettingsTopmost => "Always on top",
        MessageKey::SettingsTopmostHint => "Keep the pet window above other windows.",
        MessageKey::OptLocaleZh => "简体中文",
        MessageKey::OptLocaleEn => "English",
    }
}
