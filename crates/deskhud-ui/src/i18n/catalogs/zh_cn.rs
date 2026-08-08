use crate::i18n::MessageKey;

pub(super) fn text(key: MessageKey) -> &'static str {
    match key {
        MessageKey::AppName => "DeskHud",
        MessageKey::ActionClose => "关闭",
        MessageKey::ActionCancel => "取消",
        MessageKey::ActionApply => "应用",
        MessageKey::ActionReset => "重置",
        MessageKey::MenuQuit => "退出",
        MessageKey::MenuSettings => "设置",
        MessageKey::SettingsTitle => "设置",
        MessageKey::SettingsNavPet => "宠物",
        MessageKey::SettingsNavHud => "插件",
        MessageKey::SettingsNavGeneral => "常规",
        MessageKey::SettingsPetIntro => "选择宠物；点「应用」后生效，主窗大小随皮肤变化。",
        MessageKey::SettingsPetWindowSize => "窗口",
        MessageKey::SettingsPetSelected => "使用中",
        MessageKey::SettingsPetViewGrid => "网格",
        MessageKey::SettingsPetViewList => "列表",
        MessageKey::SettingsPetOptions => "当前宠物行为",
        MessageKey::HudSettingsIntro => "按插件管理：总开关控制整组，展开后可开关单条贡献。",
        MessageKey::HudSettingsEmpty => "当前没有可配置的插件贡献。",
        MessageKey::MetaAuthor => "作者",
        MessageKey::HudItemsEnabled => "条开启",
        MessageKey::MetaHomepage => "主页",
        MessageKey::SettingsLocale => "语言",
        MessageKey::SettingsTopmost => "始终置顶",
        MessageKey::SettingsTopmostHint => "桌宠窗口保持在其它窗口之上。",
        MessageKey::OptLocaleZh => "简体中文",
        MessageKey::OptLocaleEn => "English",
    }
}
