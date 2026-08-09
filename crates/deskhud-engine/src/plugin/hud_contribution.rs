//! 插件可声明的 HUD 条目。

/// 一条可配置的 HUD 贡献。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HudContribution {
    /// 条目 ID（**插件内**唯一短名），如 `clock`；prefs 键为 `{plugin_id}.{id}.enable`。
    pub id: &'static str,
    /// 设置页显示名。
    pub label: &'static str,
    /// 默认是否开启。
    pub default_enabled: bool,
    /// 条目图标字节（svg/png/jpeg/gif/webp）；与插件一并打包。缺省时壳用默认图标。
    pub icon: Option<&'static [u8]>,
}
