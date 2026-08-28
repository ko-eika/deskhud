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
    /// 包内资源声明或资源文件不合法。
    #[error("invalid resource: {0}")]
    InvalidResource(String),
    /// 包内资源存在但无法解析。
    #[error("damaged resource `{path}`: {reason}")]
    DamagedResource {
        /// 出错资源的包内路径。
        path: String,
        /// 解码或尺寸校验原因。
        reason: String,
    },
    /// Zip 读写失败。
    #[error("zip: {0}")]
    Zip(String),
}
