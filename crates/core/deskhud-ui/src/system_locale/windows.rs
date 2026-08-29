use super::LanguageTag;

pub(super) fn system_locale() -> Option<LanguageTag> {
    // Unlike Unix, Windows does not require LANG/LC_* to be present. Query
    // the user locale first, then retain environment support for Wine and
    // terminal compatibility layers.
    let mut buffer = [0u16; 85];
    let length = unsafe {
        windows_sys::Win32::Globalization::GetUserDefaultLocaleName(
            buffer.as_mut_ptr(),
            buffer.len() as i32,
        )
    };
    if length > 1
        && let Ok(value) = String::from_utf16(&buffer[..length as usize - 1])
        && let Some(locale) = LanguageTag::parse(&value)
    {
        return Some(locale);
    }
    ["LANGUAGE", "LC_ALL", "LANG"].into_iter().find_map(|name| {
        std::env::var(name)
            .ok()
            .and_then(|value| LanguageTag::parse(&value))
    })
}
