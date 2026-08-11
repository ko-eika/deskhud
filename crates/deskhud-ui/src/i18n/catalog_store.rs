//! 多源文案合并：外壳静态目录 + 已加载包 `i18n/*.toml`。

use std::collections::BTreeMap;

use super::{Locale, MessageKey, catalogs};

/// 动态键值表（按 locale 分桶）；缺键回退到 `en`。
#[derive(Debug, Clone, Default)]
pub struct CatalogStore {
    /// locale 标签（如 `zh-CN` / `en`）→ 键 → 文案。
    layers: BTreeMap<String, BTreeMap<String, String>>,
}

impl CatalogStore {
    /// 空目录。
    pub fn new() -> Self {
        Self::default()
    }

    /// 合并一层键值（后者覆盖同键）。
    pub fn merge_layer(&mut self, locale: &str, messages: &BTreeMap<String, String>) {
        let bucket = self.layers.entry(locale.to_string()).or_default();
        for (k, v) in messages {
            bucket.insert(k.clone(), v.clone());
        }
    }

    /// 合并包目录；`prefix` 形如 `pet.org.id.` / `hud.org.id.`。
    pub fn merge_prefixed(
        &mut self,
        locale: &str,
        prefix: &str,
        messages: &BTreeMap<String, String>,
    ) {
        let mut prefixed = BTreeMap::new();
        for (k, v) in messages {
            prefixed.insert(format!("{prefix}{k}"), v.clone());
        }
        self.merge_layer(locale, &prefixed);
    }

    /// 查动态键；顺序：当前 locale → `en` → `None`。
    pub fn get(&self, locale: Locale, key: &str) -> Option<&str> {
        let primary = locale_tag(locale);
        if let Some(v) = self.layers.get(primary).and_then(|m| m.get(key)) {
            return Some(v.as_str());
        }
        if primary != "en" {
            if let Some(v) = self.layers.get("en").and_then(|m| m.get(key)) {
                return Some(v.as_str());
            }
        }
        None
    }

    /// 外壳固定键（静态目录）。
    pub fn t_shell(locale: Locale, key: MessageKey) -> &'static str {
        catalogs::lookup(locale, key)
    }

    /// 动态键；缺失时返回 `fallback`。
    pub fn t<'a>(&'a self, locale: Locale, key: &str, fallback: &'a str) -> &'a str {
        self.get(locale, key).unwrap_or(fallback)
    }
}

/// UI / 文件用的 locale 标签。
pub fn locale_tag(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "zh-CN",
        Locale::En => "en",
    }
}

/// 尝试的 locale 文件名（含别名）。
pub fn locale_file_candidates(locale: Locale) -> &'static [&'static str] {
    match locale {
        Locale::ZhCn => &["zh-CN", "zh_cn", "zh"],
        Locale::En => &["en", "en-US"],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_prefix_and_fallback() {
        let mut store = CatalogStore::new();
        let mut zh = BTreeMap::new();
        zh.insert("display_name".into(), "酷猫".into());
        store.merge_prefixed("zh-CN", "pet.example.cat.", &zh);

        assert_eq!(
            store.get(Locale::ZhCn, "pet.example.cat.display_name"),
            Some("酷猫")
        );
        assert_eq!(store.get(Locale::En, "pet.example.cat.display_name"), None);
        assert_eq!(
            store.t(Locale::En, "pet.example.cat.display_name", "Cat"),
            "Cat"
        );
        assert_eq!(
            CatalogStore::t_shell(Locale::ZhCn, MessageKey::AppName),
            catalogs::lookup(Locale::ZhCn, MessageKey::AppName)
        );
    }
}
