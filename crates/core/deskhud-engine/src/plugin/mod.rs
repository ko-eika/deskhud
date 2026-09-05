//! 功能插件。

mod hud_contribution;
mod hud_instance;
#[path = "plugin.rs"]
mod plugin_api;
mod plugin_info;

pub use hud_contribution::{
    HudConfigChoice, HudConfigDynamicChoice, HudConfigKind, HudConfigOption, HudConfigValue,
    HudContribution, HudFrame, HudTextAlign, HudVisual,
};
pub use hud_instance::{
    HudFrameCtx, HudGroupAlignment, HudGroupArrangement, HudGroupComposition, HudGroupLayout,
    HudGroupMemberPlacement, HudInstanceId, HudLogicalRect, HudLogicalSize, HudSourceId,
};
pub use plugin_api::Plugin;
pub use plugin_info::PluginInfo;
