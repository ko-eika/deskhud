//! # deskhud-runtime
//!
//! 本地包发现与加载：把 `packages/` 下的 `.deskhud` 变成可注册的
//! [`PetKind`](deskhud_host::PetKind) / [`Plugin`](deskhud_host::Plugin)。
//!
//! - **内置**：仍由 [`HostRegistry`](deskhud_host::HostRegistry) 原生注册。
//! - **社区**：Phase 3 起经 WASM 适配器加载（本 crate 预留 `wasm` 模块）。

#![deny(missing_docs)]

pub mod bootstrap;
pub mod catalog;
pub mod error;
pub mod loader;
pub mod paths;
pub mod wasm;

pub use bootstrap::{bootstrap_registry, bootstrap_registry_result, Bootstrap};
pub use catalog::build_catalog_store;
pub use error::RuntimeError;
pub use loader::{DiscoveredPack, PackageLoader};
pub use paths::default_package_dirs;
