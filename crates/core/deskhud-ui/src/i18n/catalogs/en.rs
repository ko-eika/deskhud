use crate::i18n::MessageKey;

pub(super) fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppName => "DeskHud",
        MessageKey::ActionCancel => "Cancel",
        MessageKey::ActionApply => "Apply",
        MessageKey::ActionReset => "Reset",
        MessageKey::MenuQuit => "Quit",
        MessageKey::MenuSettings => "Settings",
        MessageKey::SettingsTitle => "Settings",
        MessageKey::SettingsNavPet => "Pet",
        MessageKey::SettingsNavHud => "Plugins",
        MessageKey::SettingsNavGeneral => "General",
        MessageKey::SettingsNavPerformance => "Performance",
        MessageKey::SettingsPerformanceIntro => {
            "Adjust frame rate, animation, and effects preferences."
        }
        MessageKey::SettingsPerformanceFps => "Frame rate limit",
        MessageKey::SettingsPerformanceAuto => "Auto",
        MessageKey::SettingsPerformanceAnimation => "Animation quality",
        MessageKey::SettingsPerformanceLow => "Low",
        MessageKey::SettingsPerformanceStandard => "Standard",
        MessageKey::SettingsPerformanceHigh => "High",
        MessageKey::SettingsPerformancePower => "Performance mode",
        MessageKey::SettingsPerformanceSaving => "Power saving",
        MessageKey::SettingsPerformanceBalanced => "Balanced",
        MessageKey::SettingsPerformanceSmooth => "Smooth",
        MessageKey::SettingsPerformanceEffects => "Bubbles and shadows",
        MessageKey::SettingsNavAbout => "About",
        MessageKey::SettingsAboutIntro => "App info and version for DeskHud.",
        MessageKey::SettingsAboutVersion => "Version",
        MessageKey::SettingsAboutLicense => "License",
        MessageKey::SettingsAboutStack => "A desktop pet engine built with Rust and egui.",
        MessageKey::SettingsPetIntro => {
            "Takes effect after you Select and Apply. Third-party pets may monitor keyboard/mouse input and pose privacy or security risks—only install packs from sources you trust."
        }
        MessageKey::SettingsPetWindowSize => "Window",
        MessageKey::SettingsPetSelected => "In use",
        MessageKey::SettingsPetOptions => "Active pet behavior",
        MessageKey::HudSettingsIntro => {
            "Takes effect after you Enable and Apply. Third-party plugins may read system info or show content and pose privacy or security risks—only enable plugins from sources you trust."
        }
        MessageKey::HudSettingsEmpty => "No plugin contributions available.",
        MessageKey::MetaAuthor => "Author",
        MessageKey::HudItemsEnabled => "enabled",
        MessageKey::MetaHomepage => "Homepage",
        MessageKey::SettingsLocale => "Language",
        MessageKey::SettingsTheme => "Theme",
        MessageKey::OptThemeLight => "Light",
        MessageKey::OptThemeDark => "Dark",
        MessageKey::OptThemeSystem => "Use system setting",
        MessageKey::SettingsUiFont => "Font",
        MessageKey::SettingsUiFontFamily => "Family",
        MessageKey::SettingsUiFontStyle => "Style",
        MessageKey::SettingsUiFontSize => "Size",
        MessageKey::SettingsUiFontPreview => "The sound of waves calms my mind. DeskHud 123",
        MessageKey::HudPluginDisabledHint => "Plugin is off; contributions stay hidden",
        MessageKey::HudMasterEnable => "Enable plugins",
        MessageKey::HudMasterEnableHint => "When off, no HUD is shown",
        MessageKey::HudMasterDisabledHint => "Off: all HUD stays hidden",
        MessageKey::HudLayoutEdit => "Plugin layout",
        MessageKey::HudLayoutEditingHint => {
            "Layout editor is open: drag chips; Reset/Cancel/Apply at the top"
        }
        MessageKey::HudLayoutDone => "Apply",
        MessageKey::HudLayoutCancel => "Cancel",
        MessageKey::HudLayoutHint => "Click to select · drag to move · corner scales by grid",
        MessageKey::HudLayoutResetSize => "Reset size",
        MessageKey::HudLayoutResetSizeHint => "Restore the default 1× size",
        MessageKey::MenuHudLayout => "Plugin layout",
        MessageKey::MenuPluginLayer => "Plugin layer",
        MessageKey::MenuLayerTop => "Always on top",
        MessageKey::MenuLayerNormal => "Normal",
        MessageKey::MenuLayerBottom => "Always on bottom",
        MessageKey::SettingsTopmost => "Keep on top",
        MessageKey::SettingsTopmostHint => {
            "Keep only the pet and HUD on top; Settings remains a normal window."
        }
        MessageKey::OptLocaleZh => "简体中文",
        MessageKey::OptLocaleSystem => "System",
        MessageKey::OptLocaleEn => "English",
    }
}
