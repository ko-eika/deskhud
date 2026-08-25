//! 包错误。

use thiserror::Error;

/// 读包 / 解析清单失败。
#[derive(Debug, Error)]
pub enum PackageError {
    /// IO。
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    /// TOML 解析。
    #[error("toml: {0}")]
    Toml(#[from] toml::de::Error),
    /// 清单字段不合法。
    #[error("invalid manifest: {0}")]
    InvalidManifest(String),
    /// 缺少入口或资源。
    #[error("missing entry: {0}")]
    MissingEntry(String),
    /// Zip 读写失败。
    #[error("zip: {0}")]
    Zip(String),
}
