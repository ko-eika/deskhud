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
    /// 侧栏：关于。
    SettingsNavAbout,
    /// 关于页说明。
    SettingsAboutIntro,
    /// 关于：版本。
    SettingsAboutVersion,
    /// 关于：许可证。
    SettingsAboutLicense,
    /// 关于：技术栈一行说明。
    SettingsAboutStack,
    /// 宠物页说明（含第三方风险提示）。
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
    /// 插件页说明（含第三方风险提示）。
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
    /// 应用主题。
    SettingsTheme,
    /// 浅色主题。
    OptThemeLight,
    /// 深色主题。
    OptThemeDark,
    /// 跟随系统主题。
    OptThemeSystem,
    /// 界面字体分区标题。
    SettingsUiFont,
    /// 字体系列。
    SettingsUiFontFamily,
    /// 字体样式。
    SettingsUiFontStyle,
    /// 字体大小。
    SettingsUiFontSize,
    /// 样式：常规。
    OptFontRegular,
    /// 样式：粗体。
    OptFontBold,
    /// 样式：细体。
    OptFontLight,
    /// 字体预览句。
    SettingsUiFontPreview,
    /// 插件总开关关闭时的提示。
    HudPluginDisabledHint,
    /// 置顶。
    SettingsTopmost,
    /// 置顶说明。
    SettingsTopmostHint,
    /// 简体中文。
    OptLocaleZh,
    /// English。
    OptLocaleEn,
}
