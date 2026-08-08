//! [`Plugin`] 扩展点。

use super::{HudContribution, PluginInfo};

/// 功能插件：领域能力 + 可选 HUD 贡献。
pub trait Plugin: Send + Sync {
    /// 元数据。
    fn info(&self) -> PluginInfo;

    /// 可配置 HUD 条目。
    fn hud_contributions(&self) -> &'static [HudContribution] {
        &[]
    }
}
