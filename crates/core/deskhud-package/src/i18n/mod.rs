//! 包内国际化目录文件。

use std::collections::BTreeMap;
use std::str;

use serde::{Deserialize, Serialize};

/// 单个 locale 的键值表（PO/MO 的 `msgid`/`msgstr`）。
///
/// 合并进宿主时由 runtime/ui 加上 `pet.<id>.` / `plugin.<id>.` 前缀。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PackCatalog {
    /// 文案：短键 → 译文。
    #[serde(default)]
    pub messages: BTreeMap<String, String>,
}

impl PackCatalog {
    /// Parses a UTF-8 gettext PO file or a binary GNU MO file.
    /// Invalid files are rejected so callers can safely keep the prior catalog.
    pub fn parse_gettext(bytes: &[u8]) -> Result<Self, String> {
        if bytes.starts_with(b"\xDE\x12\x04\x95") || bytes.starts_with(b"\x95\x04\x12\xDE") {
            parse_mo(bytes)
        } else {
            let text = str::from_utf8(bytes).map_err(|e| format!("PO is not UTF-8: {e}"))?;
            parse_po(text)
        }
    }

    /// Encodes this catalog as a little-endian GNU MO file for distribution.
    pub fn to_mo(&self) -> Vec<u8> {
        let mut entries: Vec<_> = self.messages.iter().collect();
        entries.sort_by_key(|(a, _)| *a);
        let n = entries.len() as u32;
        let original_table = 28;
        let translation_table = original_table + n * 8;
        let mut data_offset = translation_table + n * 8;
        let mut originals = Vec::with_capacity(entries.len());
        let mut translations = Vec::with_capacity(entries.len());
        let mut strings = Vec::new();
        for (original, translation) in entries {
            originals.push((original.len() as u32, data_offset));
            strings.extend(original.as_bytes());
            strings.push(0);
            data_offset += original.len() as u32 + 1;
            translations.push((translation.len() as u32, data_offset));
            strings.extend(translation.as_bytes());
            strings.push(0);
            data_offset += translation.len() as u32 + 1;
        }
        let mut out = Vec::with_capacity(data_offset as usize);
        for value in [
            0x9504_12de_u32,
            0,
            n,
            original_table,
            translation_table,
            0,
            0,
        ] {
            out.extend(value.to_le_bytes());
        }
        for (length, offset) in originals.into_iter().chain(translations) {
            out.extend(length.to_le_bytes());
            out.extend(offset.to_le_bytes());
        }
        out.extend(strings);
        out
    }
}

fn parse_po(text: &str) -> Result<PackCatalog, String> {
    let mut messages = BTreeMap::new();
    let mut id = None::<String>;
    let mut value = None::<String>;
    let mut active = None::<bool>;
    for line in text.lines().chain(std::iter::once("")) {
        let line = line.trim();
        if line.is_empty() {
            if let (Some(id), Some(value)) = (id.take(), value.take())
                && !id.is_empty()
                && !value.is_empty()
            {
                messages.insert(id, value);
            }
            active = None;
            continue;
        }
        let (field, raw) = line.split_once(' ').unwrap_or(("", ""));
        if matches!(field, "msgid" | "msgstr" | "msgstr[0]") {
            let parsed = parse_po_string(raw)?;
            match field {
                "msgid" => {
                    id = Some(parsed);
                    active = Some(false);
                }
                _ => {
                    value = Some(parsed);
                    active = Some(true);
                }
            }
        } else if line.starts_with('"') {
            let parsed = parse_po_string(line)?;
            match active {
                Some(false) => id.get_or_insert_default().push_str(&parsed),
                Some(true) => value.get_or_insert_default().push_str(&parsed),
                None => {}
            }
        }
    }
    Ok(PackCatalog { messages })
}

fn parse_po_string(raw: &str) -> Result<String, String> {
    let raw = raw.trim();
    if !raw.starts_with('"') || !raw.ends_with('"') {
        return Err("invalid PO string".into());
    }
    let inner = &raw[1..raw.len() - 1];
    let mut out = String::new();
    let mut chars = inner.chars();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        out.push(match chars.next().ok_or("truncated PO escape")? {
            'n' => '\n',
            'r' => '\r',
            't' => '\t',
            '"' => '"',
            '\\' => '\\',
            other => other,
        });
    }
    Ok(out)
}

fn parse_mo(bytes: &[u8]) -> Result<PackCatalog, String> {
    if bytes.len() < 28 {
        return Err("MO header is truncated".into());
    }
    let little = bytes.starts_with(b"\xDE\x12\x04\x95");
    let u32_at = |offset: usize| -> Result<usize, String> {
        let end = offset.checked_add(4).ok_or("MO offset overflow")?;
        let chunk = bytes.get(offset..end).ok_or("MO table is truncated")?;
        let n = if little {
            u32::from_le_bytes(chunk.try_into().unwrap())
        } else {
            u32::from_be_bytes(chunk.try_into().unwrap())
        };
        usize::try_from(n).map_err(|_| "MO value is too large".into())
    };
    if u32_at(0)? != 0x9504_12de {
        return Err("invalid MO magic".into());
    }
    let count = u32_at(8)?;
    let originals = u32_at(12)?;
    let translations = u32_at(16)?;
    let mut messages = BTreeMap::new();
    for i in 0..count {
        let oi = originals
            .checked_add(i.checked_mul(8).ok_or("MO overflow")?)
            .ok_or("MO overflow")?;
        let ti = translations
            .checked_add(i.checked_mul(8).ok_or("MO overflow")?)
            .ok_or("MO overflow")?;
        let olen = u32_at(oi)?;
        let ooff = u32_at(oi + 4)?;
        let tlen = u32_at(ti)?;
        let toff = u32_at(ti + 4)?;
        let original = str::from_utf8(
            bytes
                .get(ooff..ooff + olen)
                .ok_or("MO original out of bounds")?,
        )
        .map_err(|e| e.to_string())?;
        let translated = str::from_utf8(
            bytes
                .get(toff..toff + tlen)
                .ok_or("MO translation out of bounds")?,
        )
        .map_err(|e| e.to_string())?;
        let translated = translated.split('\0').next().unwrap_or("");
        if !original.is_empty() && !translated.is_empty() {
            messages.insert(original.to_owned(), translated.to_owned());
        }
    }
    Ok(PackCatalog { messages })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_utf8_po() {
        let c = PackCatalog::parse_gettext(
            "msgid \"\"\nmsgstr \"Language\"\n\nmsgid \"hello\"\nmsgstr \"你好\"\n\nmsgid \"missing\"\nmsgstr \"\"\n".as_bytes(),
        )
        .unwrap();
        assert_eq!(c.messages.get("hello").map(String::as_str), Some("你好"));
        assert!(!c.messages.contains_key("missing"));
    }

    #[test]
    fn parse_little_endian_mo() {
        let original = b"hello";
        let translated = "你好".as_bytes();
        let mut bytes = Vec::new();
        for n in [
            0x9504_12de_u32,
            0,
            1,
            28,
            36,
            0,
            0,
            original.len() as u32,
            44,
            translated.len() as u32,
            50,
        ] {
            bytes.extend(n.to_le_bytes());
        }
        bytes.extend(original);
        bytes.push(0);
        bytes.extend(translated);
        bytes.push(0);
        let c = PackCatalog::parse_gettext(&bytes).unwrap();
        assert_eq!(c.messages.get("hello").map(String::as_str), Some("你好"));
    }
}
