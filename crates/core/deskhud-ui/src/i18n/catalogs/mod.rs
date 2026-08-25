use crate::i18n::{Locale, MessageKey};
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn english() -> &'static BTreeMap<String, String> {
    static CACHE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        toml::from_str(include_str!("../../../../../../locales/en.toml"))
            .expect("valid English shell locale TOML")
    })
}

fn chinese() -> &'static BTreeMap<String, String> {
    static CACHE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    CACHE.get_or_init(|| {
        toml::from_str(include_str!("../../../../../../locales/zh-CN.toml"))
            .expect("valid Chinese shell locale TOML")
    })
}

pub(super) fn lookup(locale: Locale, key: MessageKey) -> &'static str {
    let key = format!("{key:?}");
    let primary = match locale.resolved() {
        Locale::ZhCn => chinese(),
        Locale::En => english(),
        Locale::System => unreachable!("system locale must be resolved"),
    };
    primary
        .get(&key)
        .or_else(|| english().get(&key))
        .map(String::as_str)
        .unwrap_or_else(|| Box::leak(key.into_boxed_str()))
}
