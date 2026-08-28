//! # deskhud-engine
//!
//! 桌宠引擎契约：**宠物类型**（皮肤 + 行为）与 **HUD 插件**扩展点。
//! 内置实现在 `packs/`；社区包经 `deskhud-runtime` 适配后同样注册进
//! [`EngineRegistry`]。本 crate 不依赖 egui。

#![deny(missing_docs)]

pub mod overlay;
pub mod pet;
pub mod plugin;
pub mod registry;

/// 引擎产品 SemVer（与 workspace / 本 crate 版本一致）。
pub const ENGINE_PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 引擎兼容族：`0.x` 为 `MAJOR.MINOR`（与当前 workspace `0.7.0` / 族 `0.7` 对齐）。
pub const ENGINE_COMPAT_FAMILY: &str = "0.7";

pub use overlay::{
    OverlayBackendCapabilities, OverlayCircle, OverlayColor, OverlayDisplayTarget, OverlayEllipse,
    OverlayEvent, OverlayHitKind, OverlayHitRegion, OverlayHitShape, OverlayPoint, OverlayRect,
    OverlayRoundedRect, OverlayScene, OverlayScreenArea, OverlayText, OverlayVisual,
    OverlayWindowId, OverlayWindowLevel, OverlayWindowRole,
};
pub use pet::{
    AssetId, DockState, DragState, MouseState, Path, PetBubbleStyle, PetConfigBag, PetConfigOption,
    PetEvent, PetKey, PetKind, PetKindInfo, PetModifiers, PetMouseButton, PetPaint, PetPaintCtx,
    PetScene, PetTheme, SceneColor, SceneItem, SceneNode, SceneValidationError, Shape, Transform2D,
};
pub use plugin::{HudContribution, HudFrame, HudVisual, Plugin, PluginInfo};
pub use registry::EngineRegistry;
