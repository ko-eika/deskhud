//! 语言。

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// UI 语言。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Locale {
    /// Follow the operating system locale.
    #[default]
    System,
    /// 简体中文。
    ZhCn,
    /// English。
    En,
    /// A language discovered from an external PO/MO catalog.
    Custom(&'static str),
}

impl Serialize for Locale {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.tag())
    }
}
impl<'de> Deserialize<'de> for Locale {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let tag = String::deserialize(deserializer)?;
        Self::from_tag(&tag).ok_or_else(|| serde::de::Error::custom("invalid locale tag"))
    }
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
                Self::from_tag(&value)
            })
            .unwrap_or(Self::ZhCn)
    }

    /// Returns the canonical BCP-47-like tag used for catalog lookup.
    pub fn tag(&self) -> String {
        match self {
            Self::System => Self::System.resolved().tag(),
            Self::ZhCn => "zh-CN".into(),
            Self::En => "en-US".into(),
            Self::Custom(tag) => (*tag).into(),
        }
    }

    /// Creates a locale from a PO/MO directory or metadata tag.
    pub fn from_tag(tag: &str) -> Option<Self> {
        let tag = normalize_locale_tag(tag);
        if tag == "system" {
            return Some(Self::System);
        }
        if tag == "zh" || tag == "zh-CN" {
            return Some(Self::ZhCn);
        }
        if tag == "en" || tag == "en-US" {
            return Some(Self::En);
        }
        (!tag.is_empty()).then_some(Self::Custom(Box::leak(tag.into_boxed_str())))
    }

    /// Candidate tags from the most specific variant to its language family.
    pub fn fallback_tags(&self) -> Vec<String> {
        let tag = self.tag();
        let mut tags = vec![tag.clone()];
        if let Some((language, _)) = tag.split_once('-') {
            tags.push(language.to_string());
        }
        tags
    }
}

/// Normalizes BCP-47, POSIX and gettext directory spellings.
pub fn normalize_locale_tag(value: &str) -> String {
    let raw = value.trim().replace('_', "-");
    let mut parts = raw.split('-');
    let Some(language) = parts.next() else {
        return String::new();
    };
    if language.len() < 2 || !language.chars().all(|c| c.is_ascii_alphabetic()) {
        return String::new();
    }
    let mut result = language.to_ascii_lowercase();
    for part in parts {
        if !part.is_empty() {
            result.push('-');
            result.push_str(&part.to_ascii_uppercase());
        }
    }
    result
}
