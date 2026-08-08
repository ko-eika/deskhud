//! 国际化。

mod catalogs;
mod keys;
mod locale;

pub use keys::MessageKey;
pub use locale::Locale;

/// 翻译固定键。
pub fn t(locale: Locale, key: MessageKey) -> &'static str {
    catalogs::lookup(locale, key)
}
