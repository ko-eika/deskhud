//! # deskhud-sdk
//!
//! 社区作者编写 **宠物包** / **HUD 插件** 时依赖本 crate，目标
//! `wasm32-unknown-unknown`，再打成 `.deskhud`。
//!
//! 宿主 **不** 依赖本 crate；ABI 由 `deskhud-runtime` 的 WASM 适配器对接。
//!
//! ## 作者流程（目标）
//!
//! 1. `cargo new --lib my-pet`，依赖 `deskhud-sdk`，`crate-type = ["cdylib"]`
//! 2. 实现 [`pet`] 或 [`plugin`] 入口
//! 3. `cargo build --target wasm32-unknown-unknown`
//! 4. 与 `manifest.toml` / `i18n/` / `assets/` 一并打成 `.deskhud`

#![deny(missing_docs)]

pub mod pet;
pub mod plugin;

/// Guest ABI 主版本；须与 [`deskhud_package::PackManifest::SUPPORTED_API_VERSION`] 一致。
pub const API_VERSION: u32 = 1;
