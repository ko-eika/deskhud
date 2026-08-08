//! 文案键。

/// 壳文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKey {
    /// 应用名。
    AppName,
    /// 关闭。
    ActionClose,
    /// 取消。
    ActionCancel,
    /// 应用。
    ActionApply,
    /// 重置。
    ActionReset,
    /// 退出。
    MenuQuit,
    /// 右键：设置。
    MenuSettings,
    /// 设置窗标题。
    SettingsTitle,
    /// 侧栏：宠物。
    SettingsNavPet,
    /// 侧栏：插件。
    SettingsNavHud,
    /// 侧栏：常规。
    SettingsNavGeneral,
    /// 宠物页说明。
    SettingsPetIntro,
    /// 窗尺寸标签。
    SettingsPetWindowSize,
    /// 当前选中标记。
    SettingsPetSelected,
    /// 宠物选择：网格。
    SettingsPetViewGrid,
    /// 宠物选择：列表。
    SettingsPetViewList,
    /// 当前宠物的行为配置区标题。
    SettingsPetOptions,
    /// 插件页说明。
    HudSettingsIntro,
    /// 尚无插件贡献。
    HudSettingsEmpty,
    /// 元数据：作者前缀。
    MetaAuthor,
    /// 「N/M 条开启」后缀。
    HudItemsEnabled,
    /// 主页链接标签。
    MetaHomepage,
    /// 语言。
    SettingsLocale,
    /// 置顶。
    SettingsTopmost,
    /// 置顶说明。
    SettingsTopmostHint,
    /// 简体中文。
    OptLocaleZh,
    /// English。
    OptLocaleEn,
}
