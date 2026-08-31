//! 国际化。

mod builtin_packs;
mod catalog_store;
mod catalogs;
mod keys;
mod locale;

pub use builtin_packs::seed_builtin_packs;
pub use catalog_store::{CatalogStore, locale_file_candidates, locale_tag};
pub use keys::MessageKey;
pub use locale::{Locale, normalize_locale_tag};

/// 翻译固定键（外壳静态目录）。
pub fn t(locale: Locale, key: MessageKey) -> &'static str {
    catalogs::lookup(locale.resolved(), key)
}
