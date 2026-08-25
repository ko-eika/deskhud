//! 语言。

use serde::{Deserialize, Serialize};

/// UI 语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    /// Follow the operating system locale.
    #[default]
    System,
    /// 简体中文。
    ZhCn,
    /// English。
    En,
}

impl Locale {
    /// Resolve the automatic choice using common process locale variables.
    pub fn resolved(self) -> Self {
        if self != Self::System {
            return self;
        }
        ["LANGUAGE", "LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .filter_map(|key| std::env::var(key).ok())
            .find_map(|value| {
                let value = value.to_ascii_lowercase();
                if value.starts_with("en") {
                    Some(Self::En)
                } else if value.starts_with("zh") {
                    Some(Self::ZhCn)
                } else {
                    None
                }
            })
            .unwrap_or(Self::ZhCn)
    }
}
