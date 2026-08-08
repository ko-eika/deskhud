//! HUD 插件 Guest API（骨架）。
//!
//! Phase 3 将提供：贡献声明、`hud_frame` 与导出宏。

/// 一条 HUD 贡献的静态描述。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HudItem {
    /// 插件内唯一 ID。
    pub id: &'static str,
    /// i18n 短键（宿主加 `plugin.<pack_id>.` 前缀）或字面回退。
    pub label_key: &'static str,
    /// 默认是否开启。
    pub default_enabled: bool,
    /// 包内图标相对路径（如 `assets/clock.png`）；与 `manifest.toml` `[[hud]]` 对齐。
    /// `None` 时宿主使用默认图标。
    pub icon: Option<&'static str>,
}

/// HUD 插件 Guest 钩子（设计稿）。
pub trait PluginGuest {
    /// 稳定 ID。
    fn id(&self) -> &str;

    /// 可配置 HUD 条目。
    fn hud_items(&self) -> &'static [HudItem];
}
