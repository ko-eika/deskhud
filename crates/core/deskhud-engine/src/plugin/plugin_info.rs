//! 插件元数据。

/// 插件描述。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PluginInfo {
    /// 稳定 ID，格式 `hud.<组织>.<标识>`，如 `hud.deskhud.demo`。
    pub id: &'static str,
    /// 显示名。
    pub display_name: &'static str,
    /// 说明。
    pub description: &'static str,
    /// 作者 / 来源。
    pub author: &'static str,
    /// 主页或仓库 URL（可选）。
    pub homepage: Option<&'static str>,
    /// 包自身 SemVer（展示用）。
    pub version: &'static str,
    /// 适配的引擎兼容族（如 `"0.2"` 或 `"1"`）。
    pub engine: &'static str,
    /// 插件图标字节（svg/png/jpeg/gif/webp）；与包一并分发。缺省时壳用默认图标。
    pub icon: Option<&'static [u8]>,
}
