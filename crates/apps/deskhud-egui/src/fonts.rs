//! 应用层字体适配：扫描结果由 `deskhud-ui` 提供，应用只负责缓存。

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

use deskhud_ui::font::FontFamilyEntry;
use deskhud_ui::{DEFAULT_UI_FONT_ID, Locale};

struct FontCache {
    locale: deskhud_ui::LanguageTag,
    all: Vec<FontFamilyEntry>,
    compatible: Vec<FontFamilyEntry>,
    selected: Option<deskhud_ui::FontSelection>,
}

static FONT_CACHE: OnceLock<FontCache> = OnceLock::new();
static FONT_DATA_CACHE: OnceLock<Mutex<HashMap<String, Arc<egui::FontData>>>> = OnceLock::new();
static FONT_LOCALE_CACHE: OnceLock<Mutex<HashMap<String, Arc<[FontFamilyEntry]>>>> =
    OnceLock::new();

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

/// Returns the cached operating-system locale.
pub(crate) fn current_locale() -> &'static deskhud_ui::LanguageTag {
    &cache().locale
}

/// Returns fonts that contain glyphs for the requested language tag.
pub(crate) fn list_font_families_for(locale: &deskhud_ui::LanguageTag) -> Arc<[FontFamilyEntry]> {
    let key = format!(
        "{}-{}",
        locale.language,
        locale.region.as_deref().unwrap_or_default()
    );
    let locale_cache = FONT_LOCALE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(entries) = locale_cache.lock() {
        if let Some(families) = entries.get(&key) {
            return families.clone();
        }
    }
    let families = if *locale == cache().locale {
        cache().compatible.clone()
    } else {
        // This is intentionally done once per language tag. The underlying
        // coverage check may read every system font file, so it must never run
        // from the egui paint loop on every frame.
        deskhud_ui::font::font_families_for_locale(locale, &cache().all)
    };
    let families: Arc<[FontFamilyEntry]> = Arc::from(families);
    if let Ok(mut entries) = locale_cache.lock() {
        entries.insert(key, families.clone());
    }
    families
}

/// Converts a user-facing locale preference to the language tag used by font coverage filtering.
pub(crate) fn language_tag_for(locale: Locale) -> deskhud_ui::LanguageTag {
    match locale.resolved() {
        Locale::ZhCn => {
            deskhud_ui::LanguageTag::parse("zh-CN").expect("built-in Chinese locale tag must parse")
        }
        Locale::En => {
            deskhud_ui::LanguageTag::parse("en-US").expect("built-in English locale tag must parse")
        }
        Locale::System => current_locale().clone(),
    }
}

/// Selects the default face using the current OS locale.
pub(crate) fn default_font_selection() -> Option<&'static deskhud_ui::FontSelection> {
    cache().selected.as_ref()
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

/// Loads the persisted UI face and size into one egui context.
pub(crate) fn configure_context_for(context: &egui::Context, font_id: &str, size: f32) {
    let Some(data) = cached_font_data(font_id).or_else(|| cached_font_data(DEFAULT_UI_FONT_ID))
    else {
        return;
    };
    let key = "deskhud_primary_font".to_owned();
    let mut definitions = egui::FontDefinitions::default();
    definitions.font_data.insert(key.clone(), data);
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        definitions
            .families
            .entry(family)
            .or_default()
            .insert(0, key.clone());
    }
    add_ui_fallback_font(&mut definitions, font_id);
    context.set_fonts(definitions);
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        context.style_mut_of(theme, |style| {
            for font_id in style.text_styles.values_mut() {
                font_id.size = size.clamp(10.0, 28.0);
            }
        });
    }
}

/// Adds one lazily loaded CJK-capable fallback after the user's selected face.
/// egui then keeps the selected font for normal text and only falls back when a
/// glyph is missing, such as the Chinese locale name in the language picker.
fn add_ui_fallback_font(definitions: &mut egui::FontDefinitions, selected_id: &str) {
    let Some(fallback_id) = cache().all.iter().find_map(|family| {
        let key = family.family_key.to_ascii_lowercase();
        let preferred = key.contains("notosanssc")
            || key.contains("notosanscjk")
            || key.contains("microsoftyahei")
            || key.contains("yahei")
            || key.contains("simsun")
            || key.contains("sourcehansans")
            || key.contains("sarasa")
            || key.contains("dengxian");
        preferred
            .then(|| family.face_for("Regular"))
            .flatten()
            .map(|face| face.font_id.clone())
    }) else {
        return;
    };
    if fallback_id == selected_id {
        return;
    }
    let Some(data) = cached_font_data(&fallback_id) else {
        return;
    };
    let key = "deskhud_ui_fallback".to_owned();
    definitions.font_data.insert(key.clone(), data);
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        let entries = definitions.families.entry(family).or_default();
        if !entries.contains(&key) {
            entries.push(key.clone());
        }
    }
}

fn cached_font_data(font_id: &str) -> Option<Arc<egui::FontData>> {
    let cache = FONT_DATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(entries) = cache.lock() {
        if let Some(data) = entries.get(font_id) {
            return Some(data.clone());
        }
    }
    let data = Arc::new(font_data_for(font_id)?);
    if let Ok(mut entries) = cache.lock() {
        entries.insert(font_id.to_owned(), data.clone());
    }
    Some(data)
}

fn font_data_for(font_id: &str) -> Option<egui::FontData> {
    if let Some((bytes, index)) = deskhud_ui::font::builtin_font_data(font_id) {
        let mut data = egui::FontData::from_static(bytes);
        data.index = index;
        return Some(data);
    }
    let (path, index) = font_id
        .split_once("#face=")
        .map_or((font_id, 0), |(path, index)| {
            (path, index.parse().unwrap_or(0))
        });
    let mut data = egui::FontData::from_owned(std::fs::read(path).ok()?);
    data.index = index;
    Some(data)
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
