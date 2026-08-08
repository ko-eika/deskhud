//! # deskhud-host
//!
//! 桌宠宿主契约：**宠物类型**（皮肤 + 行为）与 **HUD 插件**扩展点。
//! 内置实现放本 crate；社区包经 `deskhud-runtime` 适配后同样注册进
//! [`HostRegistry`]。本 crate 不依赖 egui。

#![deny(missing_docs)]

pub mod pet;
pub mod plugin;
pub mod registry;

pub use pet::{
    BuiltinBlobPet, BuiltinSpecsPet, DockState, DragState, MouseState, PetConfigBag,
    PetConfigOption, PetEvent, PetKey, PetKind, PetKindInfo, PetModifiers, PetMouseButton, PetPaint,
    PetPaintCtx,
};
pub use plugin::{DemoHudPlugin, HudContribution, Plugin, PluginInfo};
pub use registry::HostRegistry;
