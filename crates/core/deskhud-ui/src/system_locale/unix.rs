use super::LanguageTag;

pub(super) fn system_locale() -> Option<LanguageTag> {
    ["LC_ALL", "LC_MESSAGES", "LANG", "LANGUAGE"]
        .into_iter()
        .filter_map(|name| std::env::var(name).ok())
        .find_map(|value| LanguageTag::parse(&value))
}
