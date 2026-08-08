mod en;
mod zh_cn;

use crate::i18n::{Locale, MessageKey};

pub(super) fn lookup(locale: Locale, key: MessageKey) -> &'static str {
    match locale {
        Locale::ZhCn => zh_cn::text(key),
        Locale::En => en::text(key),
    }
}
