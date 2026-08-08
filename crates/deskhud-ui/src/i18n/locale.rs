//! 语言。

use serde::{Deserialize, Serialize};

/// UI 语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    /// 简体中文。
    #[default]
    ZhCn,
    /// English。
    En,
}
