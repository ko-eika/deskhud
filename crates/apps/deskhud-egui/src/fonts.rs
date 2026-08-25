//! 应用层字体适配：扫描结果由 `deskhud-ui` 提供，应用只负责缓存。

use std::borrow::Cow;
use std::sync::OnceLock;

use deskhud_ui::font::FontFamilyEntry;
use deskhud_ui::{
    DEFAULT_UI_FONT_FAMILY, DEFAULT_UI_FONT_ID, DEFAULT_UI_FONT_SIZE, DEFAULT_UI_FONT_STYLE,
};

struct FontCache {
    locale: deskhud_ui::LanguageTag,
    all: Vec<FontFamilyEntry>,
    compatible: Vec<FontFamilyEntry>,
    selected: Option<deskhud_ui::FontSelection>,
}

static FONT_CACHE: OnceLock<FontCache> = OnceLock::new();

fn cache() -> &'static FontCache {
    FONT_CACHE.get_or_init(|| {
        let locale = deskhud_ui::current_system_locale();
        let builtin = deskhud_ui::font::builtin_font_families();
        let system = deskhud_ui::font::system_font_families();
        let all = deskhud_ui::font::merge_font_families(builtin.clone(), system);
        let compatible = deskhud_ui::font::font_families_for_locale(&locale, &all);
        let selected = compatible.iter().find_map(|family| {
            family
                .face_for("Regular")
                .map(|face| deskhud_ui::FontSelection {
                    family_key: family.family_key.clone(),
                    family_label: family.label.clone(),
                    style: face.style.clone(),
                    font_id: face.font_id.clone(),
                    builtin: face.builtin,
                })
        });
        let selected = selected.or_else(|| {
            builtin
                .iter()
                .find(|family| family.family_key == "inter")
                .and_then(|family| family.face_for("Regular"))
                .map(|face| deskhud_ui::FontSelection {
                    family_key: "inter".into(),
                    family_label: "Inter".into(),
                    style: face.style.clone(),
                    font_id: face.font_id.clone(),
                    builtin: true,
                })
        });
        FontCache {
            locale,
            all,
            compatible,
            selected,
        }
    })
}

/// 返回当前系统地区/语言支持的字体家族列表。
pub(crate) fn list_font_families() -> &'static [FontFamilyEntry] {
    match list_font_families_for(current_locale()) {
        Cow::Borrowed(families) => families,
        Cow::Owned(_) => unreachable!("current locale must use the cached font list"),
    }
}

/// 返回全部字体，仅供诊断或管理用途使用。
pub(crate) fn list_all_font_families() -> &'static [FontFamilyEntry] {
    cache().all.as_slice()
}

/// Returns the cached operating-system locale.
pub(crate) fn current_locale() -> &'static deskhud_ui::LanguageTag {
    &cache().locale
}

/// Returns fonts that contain glyphs for the requested language tag.
pub(crate) fn list_font_families_for(
    locale: &deskhud_ui::LanguageTag,
) -> Cow<'static, [FontFamilyEntry]> {
    // The settings page currently follows the OS locale. Keep the argument in
    // the adapter API, but do not rescan files when the page repaints.
    if *locale == cache().locale {
        return Cow::Borrowed(cache().compatible.as_slice());
    }
    Cow::Owned(deskhud_ui::font::font_families_for_locale(
        locale,
        list_font_families(),
    ))
}

/// Selects the default face using the current OS locale.
pub(crate) fn default_font_selection() -> Option<&'static deskhud_ui::FontSelection> {
    cache().selected.as_ref()
}

/// 返回设置页初始字体选择，不复制字符串也不依赖 egui。
pub(crate) const fn default_font() -> (&'static str, &'static str, &'static str, f32) {
    (
        DEFAULT_UI_FONT_ID,
        DEFAULT_UI_FONT_FAMILY,
        DEFAULT_UI_FONT_STYLE,
        DEFAULT_UI_FONT_SIZE,
    )
}

/// Loads the selected face into egui, falling back to the embedded Inter TTC.
pub(crate) fn configure_context(context: &egui::Context) {
    let mut definitions = egui::FontDefinitions::default();
    let key = "deskhud_primary_font".to_owned();
    let Some(data) = selected_font_data().or_else(|| {
        deskhud_ui::font::builtin_font_data("Inter#face=0").map(|(bytes, index)| {
            let mut data = egui::FontData::from_static(bytes);
            data.index = index;
            data
        })
    }) else {
        return;
    };
    definitions
        .font_data
        .insert(key.clone(), std::sync::Arc::new(data));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        definitions
            .families
            .entry(family)
            .or_default()
            .insert(0, key.clone());
    }
    context.set_fonts(definitions);
}

fn selected_font_data() -> Option<egui::FontData> {
    let selection = default_font_selection()?;
    if selection.builtin {
        let (bytes, index) = deskhud_ui::font::builtin_font_data(&selection.font_id)?;
        let mut data = egui::FontData::from_static(bytes);
        data.index = index;
        return Some(data);
    }
    let (path, index) = selection
        .font_id
        .split_once("#face=")
        .map_or((selection.font_id.as_str(), 0), |(path, index)| {
            (path, index.parse().unwrap_or(0))
        });
    let mut data = egui::FontData::from_owned(std::fs::read(path).ok()?);
    data.index = index;
    Some(data)
}
