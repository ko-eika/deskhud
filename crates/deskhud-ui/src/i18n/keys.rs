//! 文案键。

/// 壳文案。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MessageKey {
    /// 应用名。
    AppName,
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
    /// 侧栏：性能。
    SettingsNavPerformance,
    /// 性能页说明。
    SettingsPerformanceIntro,
    /// 性能页帧率标签。
    SettingsPerformanceFps,
    /// 自动帧率选项。
    SettingsPerformanceAuto,
    /// 动画质量标签。
    SettingsPerformanceAnimation,
    /// 低动画质量选项。
    SettingsPerformanceLow,
    /// 标准动画质量选项。
    SettingsPerformanceStandard,
    /// 高动画质量选项。
    SettingsPerformanceHigh,
    /// 性能模式标签。
    SettingsPerformancePower,
    /// 省电性能模式选项。
    SettingsPerformanceSaving,
    /// 平衡性能模式选项。
    SettingsPerformanceBalanced,
    /// 流畅性能模式选项。
    SettingsPerformanceSmooth,
    /// 气泡与阴影开关标签。
    SettingsPerformanceEffects,
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
    /// 字体预览句。
    SettingsUiFontPreview,
    /// 插件总开关关闭时的提示。
    HudPluginDisabledHint,
    /// 插件页：全局启用全部 HUD。
    HudMasterEnable,
    /// 插件页：全局已开启时的说明。
    HudMasterEnableHint,
    /// 插件页：全局已关闭时的说明。
    HudMasterDisabledHint,
    /// 调整 HUD / 插件布局。
    HudLayoutEdit,
    /// 布局编辑中提示。
    HudLayoutEditingHint,
    /// 应用 HUD 布局。
    HudLayoutDone,
    /// 取消 HUD 布局。
    HudLayoutCancel,
    /// 布局工具条提示。
    HudLayoutHint,
    /// 布局编辑：重置条目大小为 1×。
    HudLayoutResetSize,
    /// 布局编辑：重置大小悬浮说明。
    HudLayoutResetSizeHint,
    /// 右键：插件布局。
    MenuHudLayout,
    /// 全局置顶。
    SettingsTopmost,
    /// 全局置顶说明。
    SettingsTopmostHint,
    /// 简体中文。
    OptLocaleZh,
    /// 自动识别系统语言。
    OptLocaleSystem,
    /// English。
    OptLocaleEn,
}

impl MessageKey {
    /// Every shell message key, used to validate locale coverage.
    pub const ALL: &'static [Self] = &[
        Self::AppName,
        Self::ActionCancel,
        Self::ActionApply,
        Self::ActionReset,
        Self::MenuQuit,
        Self::MenuSettings,
        Self::SettingsTitle,
        Self::SettingsNavPet,
        Self::SettingsNavHud,
        Self::SettingsNavGeneral,
        Self::SettingsNavPerformance,
        Self::SettingsPerformanceIntro,
        Self::SettingsPerformanceFps,
        Self::SettingsPerformanceAuto,
        Self::SettingsPerformanceAnimation,
        Self::SettingsPerformanceLow,
        Self::SettingsPerformanceStandard,
        Self::SettingsPerformanceHigh,
        Self::SettingsPerformancePower,
        Self::SettingsPerformanceSaving,
        Self::SettingsPerformanceBalanced,
        Self::SettingsPerformanceSmooth,
        Self::SettingsPerformanceEffects,
        Self::SettingsNavAbout,
        Self::SettingsAboutIntro,
        Self::SettingsAboutVersion,
        Self::SettingsAboutLicense,
        Self::SettingsAboutStack,
        Self::SettingsPetIntro,
        Self::SettingsPetWindowSize,
        Self::SettingsPetSelected,
        Self::SettingsPetOptions,
        Self::HudSettingsIntro,
        Self::HudSettingsEmpty,
        Self::MetaAuthor,
        Self::HudItemsEnabled,
        Self::MetaHomepage,
        Self::SettingsLocale,
        Self::SettingsTheme,
        Self::OptThemeLight,
        Self::OptThemeDark,
        Self::OptThemeSystem,
        Self::SettingsUiFont,
        Self::SettingsUiFontFamily,
        Self::SettingsUiFontStyle,
        Self::SettingsUiFontSize,
        Self::SettingsUiFontPreview,
        Self::HudPluginDisabledHint,
        Self::HudMasterEnable,
        Self::HudMasterEnableHint,
        Self::HudMasterDisabledHint,
        Self::HudLayoutEdit,
        Self::HudLayoutEditingHint,
        Self::HudLayoutDone,
        Self::HudLayoutCancel,
        Self::HudLayoutHint,
        Self::HudLayoutResetSize,
        Self::HudLayoutResetSizeHint,
        Self::MenuHudLayout,
        Self::SettingsTopmost,
        Self::SettingsTopmostHint,
        Self::OptLocaleZh,
        Self::OptLocaleSystem,
        Self::OptLocaleEn,
    ];
}
