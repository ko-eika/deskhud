//! # deskhud-runtime
//!
//! 本地包发现与加载：把 `packages/` 下的 `.deskhud` 变成可注册的
//! [`PetKind`](deskhud_engine::PetKind) / [`Plugin`](deskhud_engine::Plugin)。
//!
//! - **内置**：`packs/*` 原生 crate（`pet-*` / `hud-*`），由本 crate 引导注册进空的 [`EngineRegistry`](deskhud_engine::EngineRegistry)。
//! - **社区**：Phase 3 起经 WASM 适配器加载（本 crate 预留 `wasm` 模块）。

#![deny(missing_docs)]

pub mod bootstrap;
pub mod catalog;
pub mod error;
pub mod loader;
pub mod paths;
pub mod wasm;

pub use bootstrap::{Bootstrap, bootstrap_registry, bootstrap_registry_result};
pub use catalog::build_catalog_store;
pub use error::RuntimeError;
pub use loader::{DiscoveredPack, PackInstance, PackageLoader, PetInstanceSlot};
pub use paths::default_package_dirs;
