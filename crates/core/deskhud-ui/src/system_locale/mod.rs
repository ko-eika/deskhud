//! Operating-system language/region discovery.

#[cfg(windows)]
mod windows;
#[cfg(windows)]
use windows::system_locale;

#[cfg(unix)]
mod unix;
#[cfg(unix)]
use unix::system_locale;

#[cfg(not(any(windows, unix)))]
mod fallback;
#[cfg(not(any(windows, unix)))]
use fallback::system_locale;

/// Normalized BCP-47-like language tag used by locale-aware font selection.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LanguageTag {
    /// Lowercase ISO 639 language code.
    pub language: String,
    /// Optional uppercase ISO 3166 region code.
    pub region: Option<String>,
}

impl LanguageTag {
    /// Parses common BCP-47, POSIX and Windows locale spellings.
    pub fn parse(value: &str) -> Option<Self> {
        let value = value.trim().split('.').next()?.split('@').next()?;
        let mut parts = value.split(['-', '_']);
        let language = parts.next()?.trim().to_ascii_lowercase();
        if language.len() < 2 || !language.chars().all(|ch| ch.is_ascii_alphabetic()) {
            return None;
        }
        let region = parts
            .find(|part| part.len() == 2 && part.chars().all(|ch| ch.is_ascii_alphabetic()))
            .map(|part| part.to_ascii_uppercase());
        Some(Self { language, region })
    }

    /// Returns the process/OS locale, falling back to English.
    pub fn system() -> Self {
        system_locale().unwrap_or_else(|| Self {
            language: "en".into(),
            region: Some("US".into()),
        })
    }

    /// Returns a short representative sample for glyph coverage checks.
    pub fn sample_text(&self) -> &'static str {
        match self.language.as_str() {
            "zh" => match self.region.as_deref() {
                Some("TW" | "HK" | "MO") => "繁體中文測試",
                _ => "简体中文测试",
            },
            "ja" => "日本語テスト",
            "ko" => "한국어 테스트",
            "ar" => "اختبار العربية",
            "he" => "בדיקת עברית",
            "th" => "ทดสอบภาษาไทย",
            "ru" | "uk" | "bg" | "sr" => "Русский тест",
            _ => "Aa Zz 0123",
        }
    }
}

/// Detects the current operating-system language and region.
pub fn current_system_locale() -> LanguageTag {
    LanguageTag::system()
}

#[cfg(test)]
mod tests {
    use super::LanguageTag;

    #[test]
    fn parses_common_locale_spellings() {
        assert_eq!(
            LanguageTag::parse("zh_CN.UTF-8").unwrap().region.as_deref(),
            Some("CN")
        );
        assert_eq!(LanguageTag::parse("ja-JP").unwrap().language, "ja");
        assert_eq!(LanguageTag::parse("en").unwrap().region, None);
    }
}
