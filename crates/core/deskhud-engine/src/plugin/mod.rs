//! 功能插件。

mod hud_contribution;
#[path = "plugin.rs"]
mod plugin_api;
mod plugin_info;

pub use hud_contribution::{HudContribution, HudFrame, HudVisual};
pub use plugin_api::Plugin;
pub use plugin_info::PluginInfo;
