//! WASM Guest 适配（Phase 3）。
//!
//! 将社区 `guest.wasm` 适配为 [`deskhud_engine::PetKind`] / [`deskhud_engine::Plugin`]。
//! 本阶段仅占位，避免过早拉入 wasmtime。

use crate::RuntimeError;

/// 加载 WASM 宠物/插件（未实现）。
pub fn load_wasm_guest(_wasm_bytes: &[u8]) -> Result<(), RuntimeError> {
    Err(RuntimeError::Wasm(
        "WASM guest loading lands in Phase 3 (wasmtime + deskhud-sdk)".into(),
    ))
}
