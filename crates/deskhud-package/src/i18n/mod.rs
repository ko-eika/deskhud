//! 包内国际化目录文件。

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// 单个 locale 的键值表（`i18n/<locale>.toml` 的 `[messages]` 或扁平表）。
///
/// 合并进宿主时由 runtime/ui 加上 `pet.<id>.` / `plugin.<id>.` 前缀。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackCatalog {
    /// 文案：短键 → 译文。
    #[serde(default)]
    pub messages: BTreeMap<String, String>,
}

impl PackCatalog {
    /// 从 TOML 解析。支持顶层扁平键或 `[messages]` 表。
    pub fn parse_toml(text: &str) -> Result<Self, toml::de::Error> {
        #[derive(Deserialize)]
        struct Raw {
            #[serde(default)]
            messages: BTreeMap<String, String>,
            #[serde(flatten)]
            flat: BTreeMap<String, String>,
        }
        let raw: Raw = toml::from_str(text)?;
        let mut messages = raw.messages;
        for (k, v) in raw.flat {
            if k != "messages" {
                messages.entry(k).or_insert(v);
            }
        }
        Ok(Self { messages })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_flat_and_table() {
        let c = PackCatalog::parse_toml(
            r#"
display_name = "酷猫"
[messages]
idle = "闲逛"
"#,
        )
        .unwrap();
        assert_eq!(
            c.messages.get("display_name").map(String::as_str),
            Some("酷猫")
        );
        assert_eq!(c.messages.get("idle").map(String::as_str), Some("闲逛"));
    }
}
