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
    /// 常规页说明。
    SettingsGeneralIntro,
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
    /// 效果配置卡标题。
    SettingsPerformanceEffects,
    /// 气泡开关标签。
    SettingsPerformanceBubbles,
    /// 阴影开关标签。
    SettingsPerformanceShadows,
    /// 侧栏：关于。
    SettingsNavAbout,
    /// 关于页说明。
    SettingsAboutIntro,
    /// 关于：版本。
    SettingsAboutVersion,
    /// 关于：许可证。
    SettingsAboutLicense,
    /// 关于：作者。
    SettingsAboutAuthors,
    /// 关于：技术栈一行说明。
    SettingsAboutStack,
    /// 关于：技术栈。
    SettingsAboutStackLabel,
    /// 关于：主页。
    SettingsAboutHomepage,
    /// 宠物页说明（含第三方风险提示）。
    SettingsPetIntro,
    /// 窗尺寸标签。
    SettingsPetWindowSize,
    /// 当前宠物的行为配置区标题。
    SettingsPetOptions,
    /// 宠物选择器卡片标题。
    SettingsPetList,
    /// 当前宠物配置卡片标题。
    SettingsPetConfig,
    /// 宠物没有可配置项。
    SettingsPetEmpty,
    /// 宠物全局配置标题。
    SettingsPetGlobal,
    /// 消息气泡开关。
    SettingsPetBubbles,
    /// 消息气泡说明。
    SettingsPetBubblesHint,
    /// 全局键盘监听开关。
    SettingsPetKeyboardInput,
    /// 全局键盘监听说明。
    SettingsPetKeyboardInputHint,
    /// 全局鼠标监听开关。
    SettingsPetMouseInput,
    /// 全局鼠标监听说明。
    SettingsPetMouseInputHint,
    /// 插件页说明（含第三方风险提示）。
    HudSettingsIntro,
    /// 插件页：全局配置卡片标题。
    HudGlobalConfig,
    /// 插件页：插件列表卡片标题。
    HudPluginList,
    /// 插件页：当前插件配置卡片标题。
    HudPluginConfig,
    /// 尚无插件贡献。
    HudSettingsEmpty,
    /// 元数据：作者前缀。
    MetaAuthor,
    /// 元数据：版本前缀。
    MetaVersion,
    /// 元数据：引擎前缀。
    MetaEngine,
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
    /// HUD 右键调整窗口标题。
    HudAdjustTitle,
    /// HUD 位置字段。
    HudAdjustPosition,
    /// HUD 大小字段。
    HudAdjustSize,
    /// HUD 背景透明度字段。
    HudAdjustBackgroundOpacity,
    /// HUD 背景模糊字段。
    HudAdjustBackgroundBlur,
    /// HUD 内容透明度字段。
    HudAdjustContentOpacity,
    /// HUD X position field.
    HudAdjustX,
    /// HUD Y position field.
    HudAdjustY,
    /// HUD width field.
    HudAdjustWidth,
    /// HUD height field.
    HudAdjustHeight,
    /// HUD aspect-ratio lock.
    HudAdjustLockRatio,
    /// HUD visual effects section heading.
    HudAdjustEffects,
    /// HUD layout editor: snap positions to a grid.
    HudAdjustSnapGrid,
    /// Percentage unit option.
    HudAdjustPercent,
    /// Pixel unit option.
    HudAdjustPixels,
    /// 右键：插件布局。
    MenuHudLayout,
    /// 右键：插件窗口层级。
    MenuPluginLayer,
    /// 插件窗口层级：始终置顶。
    MenuLayerTop,
    /// 插件窗口层级：普通。
    MenuLayerNormal,
    /// 插件窗口层级：始终置底。
    MenuLayerBottom,
    /// 全局置顶。
    SettingsTopmost,
    /// 全局置顶说明。
    SettingsTopmostHint,
    /// 宠物层级。
    SettingsPetLayer,
    /// 宠物层级说明。
    SettingsPetLayerHint,
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
        Self::SettingsGeneralIntro,
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
        Self::SettingsPerformanceBubbles,
        Self::SettingsPerformanceShadows,
        Self::SettingsNavAbout,
        Self::SettingsAboutIntro,
        Self::SettingsAboutVersion,
        Self::SettingsAboutLicense,
        Self::SettingsAboutAuthors,
        Self::SettingsAboutStack,
        Self::SettingsAboutStackLabel,
        Self::SettingsAboutHomepage,
        Self::SettingsPetIntro,
        Self::SettingsPetWindowSize,
        Self::SettingsPetOptions,
        Self::SettingsPetList,
        Self::SettingsPetConfig,
        Self::SettingsPetEmpty,
        Self::SettingsPetGlobal,
        Self::SettingsPetBubbles,
        Self::SettingsPetBubblesHint,
        Self::SettingsPetKeyboardInput,
        Self::SettingsPetKeyboardInputHint,
        Self::SettingsPetMouseInput,
        Self::SettingsPetMouseInputHint,
        Self::HudSettingsIntro,
        Self::HudGlobalConfig,
        Self::HudPluginList,
        Self::HudPluginConfig,
        Self::HudSettingsEmpty,
        Self::MetaAuthor,
        Self::MetaVersion,
        Self::MetaEngine,
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
        Self::HudAdjustTitle,
        Self::HudAdjustPosition,
        Self::HudAdjustSize,
        Self::HudAdjustBackgroundOpacity,
        Self::HudAdjustBackgroundBlur,
        Self::HudAdjustContentOpacity,
        Self::HudAdjustX,
        Self::HudAdjustY,
        Self::HudAdjustWidth,
        Self::HudAdjustHeight,
        Self::HudAdjustLockRatio,
        Self::HudAdjustEffects,
        Self::HudAdjustSnapGrid,
        Self::HudAdjustPercent,
        Self::HudAdjustPixels,
        Self::MenuHudLayout,
        Self::MenuPluginLayer,
        Self::MenuLayerTop,
        Self::MenuLayerNormal,
        Self::MenuLayerBottom,
        Self::SettingsTopmost,
        Self::SettingsTopmostHint,
        Self::SettingsPetLayer,
        Self::SettingsPetLayerHint,
        Self::OptLocaleZh,
        Self::OptLocaleSystem,
        Self::OptLocaleEn,
    ];
}
