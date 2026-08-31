use crate::i18n::{Locale, MessageKey};
use std::collections::BTreeMap;
use std::sync::OnceLock;

fn english() -> &'static BTreeMap<String, String> {
    static CACHE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    CACHE.get_or_init(|| parse_po(include_str!(concat!(env!("OUT_DIR"), "/shell-en-US.po"))))
}

fn chinese() -> &'static BTreeMap<String, String> {
    static CACHE: OnceLock<BTreeMap<String, String>> = OnceLock::new();
    CACHE.get_or_init(|| parse_po(include_str!(concat!(env!("OUT_DIR"), "/shell-zh-CN.po"))))
}

fn parse_po(text: &str) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let mut id: Option<String> = None;
    let mut value: Option<String> = None;
    let mut target = false;
    for line in text.lines().chain(std::iter::once("")) {
        let line = line.trim();
        if line.is_empty() {
            if let (Some(i), Some(v)) = (id.take(), value.take())
                && !i.is_empty()
                && !v.is_empty()
            {
                out.insert(i, v);
            }
            target = false;
            continue;
        }
        if let Some(raw) = line.strip_prefix("msgid ") {
            id = Some(unquote(raw));
            target = false;
        } else if let Some(raw) = line.strip_prefix("msgstr ") {
            value = Some(unquote(raw));
            target = true;
        } else if line.starts_with('"') {
            let part = unquote(line);
            if target {
                value.get_or_insert_default().push_str(&part);
            } else {
                id.get_or_insert_default().push_str(&part);
            }
        }
    }
    out
}
fn unquote(raw: &str) -> String {
    let raw = raw.trim();
    let inner = raw
        .strip_prefix('"')
        .and_then(|v| v.strip_suffix('"'))
        .unwrap_or("");
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            out.push(match chars.next().unwrap_or('\\') {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                '"' => '"',
                '\\' => '\\',
                x => x,
            });
        } else {
            out.push(c);
        }
    }
    out
}

pub(super) fn lookup(locale: Locale, key: MessageKey) -> &'static str {
    let key = format!("{key:?}");
    let resolved = locale.resolved();
    let primary = match resolved {
        Locale::ZhCn => chinese(),
        Locale::En => english(),
        Locale::System | Locale::Custom(_) => english(),
    };
    if let Some(value) = primary.get(&key) {
        return value;
    }
    if let Some(value) = english().get(&key) {
        return value;
    }
    // The PO files are the source of truth. This single literal is only used
    // when a newly added MessageKey has not yet been translated at all.
    "未翻译"
}
