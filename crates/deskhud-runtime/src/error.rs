//! Runtime 错误。

use thiserror::Error;

use deskhud_package::PackageError;

/// 发现或加载包失败。
#[derive(Debug, Error)]
pub enum RuntimeError {
    /// 包格式。
    #[error(transparent)]
    Package(#[from] PackageError),
    /// IO。
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// WASM 尚未接入或加载失败。
    #[error("wasm: {0}")]
    Wasm(String),
}
