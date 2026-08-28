//! 应用层字体适配：扫描结果由 `deskhud-ui` 提供，应用只负责缓存。

use std::collections::{HashMap, VecDeque};
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
static FONT_DATA_CACHE: OnceLock<Mutex<FontDataCache>> = OnceLock::new();
static FONT_LOCALE_CACHE: OnceLock<Mutex<HashMap<String, Arc<[FontFamilyEntry]>>>> =
    OnceLock::new();

const MAX_CACHED_FONT_FILES: usize = 2;

#[derive(Default)]
struct FontDataCache {
    entries: HashMap<String, Arc<egui::FontData>>,
    order: VecDeque<String>,
}

fn cache() -> &'static FontCache {
    FONT_CACHE.get_or_init(|| {
        let locale = deskhud_ui::current_system_locale();
        let application = deskhud_ui::font::font_families_from_dirs(application_font_dirs());
        let system = deskhud_ui::font::system_font_families();
        let all = deskhud_ui::font::merge_font_families(application.clone(), system);
        let compatible = deskhud_ui::font::font_families_for_locale(&locale, &all);
        let selected = compatible.iter().find_map(|family| {
            is_default_font_family(family)
                .then(|| family.face_for(deskhud_ui::DEFAULT_UI_FONT_STYLE))
                .flatten()
                .map(|face| deskhud_ui::FontSelection {
                    family_key: family.family_key.clone(),
                    family_label: family.label.clone(),
                    style: face.style.clone(),
                    font_id: face.font_id.clone(),
                    builtin: face.builtin,
                })
        });
        let selected = selected.or_else(|| {
            compatible.iter().find_map(|family| {
                family
                    .face_for("Regular")
                    .map(|face| deskhud_ui::FontSelection {
                        family_key: family.family_key.clone(),
                        family_label: family.label.clone(),
                        style: face.style.clone(),
                        font_id: face.font_id.clone(),
                        builtin: face.builtin,
                    })
            })
        });
        let selected = selected.or_else(|| {
            application
                .iter()
                .find(|family| is_default_font_family(family))
                .and_then(|family| family.face_for(deskhud_ui::DEFAULT_UI_FONT_STYLE))
                .map(|face| deskhud_ui::FontSelection {
                    family_key: deskhud_ui::DEFAULT_UI_FONT_FAMILY.into(),
                    family_label: "Source Han Sans".into(),
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

/// Loads the selected face into egui, falling back to the bundled default font.
pub(crate) fn configure_context(context: &egui::Context) {
    let mut definitions = egui::FontDefinitions::default();
    let key = "deskhud_primary_font".to_owned();
    let selected_id = default_font_selection()
        .map(|selection| selection.font_id.as_str())
        .unwrap_or(DEFAULT_UI_FONT_ID);
    let Some(data) = cached_font_data(selected_id).or_else(|| cached_font_data(DEFAULT_UI_FONT_ID))
    else {
        return;
    };
    definitions.font_data.insert(key.clone(), data);
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
    let requested_id = if font_id == DEFAULT_UI_FONT_ID {
        default_font_selection()
            .map(|selection| selection.font_id.as_str())
            .unwrap_or(font_id)
    } else {
        font_id
    };
    let (selected_id, data) = if let Some(data) = cached_font_data(requested_id) {
        (requested_id.to_owned(), Some(data))
    } else if let Some(selection) = default_font_selection() {
        (
            selection.font_id.clone(),
            cached_font_data(&selection.font_id),
        )
    } else {
        (
            DEFAULT_UI_FONT_ID.to_owned(),
            cached_font_data(DEFAULT_UI_FONT_ID),
        )
    };
    let Some(data) = data else {
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
    add_ui_fallback_font(&mut definitions, &selected_id);
    context.set_fonts(definitions);
    for theme in [egui::Theme::Dark, egui::Theme::Light] {
        context.style_mut_of(theme, |style| {
            let current_body_size = style
                .text_styles
                .get(&egui::TextStyle::Body)
                .map_or(14.0, |font_id| font_id.size)
                .max(1.0);
            let scale = size.clamp(10.0, 28.0) / current_body_size;
            for font_id in style.text_styles.values_mut() {
                font_id.size = (font_id.size * scale).clamp(8.0, 48.0);
            }
        });
    }
}

/// Returns the configured base UI size used by all settings typography.
pub(crate) fn base_size(ui: &egui::Ui) -> f32 {
    ui.style()
        .text_styles
        .get(&egui::TextStyle::Body)
        .map_or(14.0, |font| font.size)
        .max(1.0)
}

/// Scales a settings font size from the configured base body size.
pub(crate) fn scaled_size(ui: &egui::Ui, ratio: f32) -> f32 {
    base_size(ui) * ratio
}

/// Builds a proportional settings font from the configured base body size.
pub(crate) fn scaled_font(ui: &egui::Ui, ratio: f32) -> egui::FontId {
    egui::FontId::proportional(scaled_size(ui, ratio))
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
    let cache = FONT_DATA_CACHE.get_or_init(|| Mutex::new(FontDataCache::default()));
    if let Ok(entries) = cache.lock() {
        if let Some(data) = entries.entries.get(font_id) {
            return Some(data.clone());
        }
    }
    let data = Arc::new(font_data_for(font_id)?);
    if let Ok(mut entries) = cache.lock() {
        if let Some(existing) = entries.entries.get(font_id) {
            return Some(existing.clone());
        }
        let font_id = font_id.to_owned();
        entries.entries.insert(font_id.clone(), data.clone());
        entries.order.push_back(font_id);
        while entries.entries.len() > MAX_CACHED_FONT_FILES {
            let Some(oldest) = entries.order.pop_front() else {
                break;
            };
            entries.entries.remove(&oldest);
        }
    }
    Some(data)
}

fn font_data_for(font_id: &str) -> Option<egui::FontData> {
    if font_id == DEFAULT_UI_FONT_ID {
        let selection = default_font_selection()?;
        if selection.font_id == font_id {
            return None;
        }
        return font_data_for(&selection.font_id);
    }
    let (path, index) = font_id
        .split_once("#face=")
        .map_or((font_id, 0), |(path, index)| {
            (path, index.parse().unwrap_or(0))
        });
    let path = resolve_font_path(path);
    let mut data = egui::FontData::from_owned(std::fs::read(path).ok()?);
    data.index = index;
    Some(data)
}

fn application_font_dirs() -> Vec<std::path::PathBuf> {
    let Some(executable_dir) = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(std::path::Path::to_path_buf))
    else {
        return Vec::new();
    };
    let mut candidates = vec![executable_dir.join("fonts")];
    // A macOS app keeps bundled resources under Contents/Resources while the
    // executable itself lives under Contents/MacOS.
    if executable_dir.file_name().and_then(|name| name.to_str()) == Some("MacOS") {
        if let Some(contents_dir) = executable_dir.parent() {
            candidates.push(contents_dir.join("Resources/fonts"));
        }
    }
    candidates
        .into_iter()
        .filter(|path| path.is_dir())
        .collect()
}

/// Stores bundled application fonts relative to the executable directory.
/// System fonts keep their absolute IDs because they are not owned by the app.
pub(crate) fn persistable_font_id(font_id: &str) -> String {
    let (path, face) = font_id
        .split_once("#face=")
        .map_or((font_id, None), |(path, face)| (path, Some(face)));
    let path = std::path::Path::new(path);
    if !path.is_absolute() {
        return font_id.to_owned();
    }
    let Some(fonts_dir) = application_font_dirs().into_iter().next() else {
        return font_id.to_owned();
    };
    let canonical_path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let canonical_fonts_dir =
        std::fs::canonicalize(&fonts_dir).unwrap_or_else(|_| fonts_dir.clone());
    let Ok(relative) = canonical_path.strip_prefix(&canonical_fonts_dir) else {
        return font_id.to_owned();
    };
    let relative = std::path::Path::new("fonts")
        .join(relative)
        .to_string_lossy()
        .replace('\\', "/");
    face.map_or(relative.clone(), |face| format!("{relative}#face={face}"))
}

fn is_default_font_family(family: &FontFamilyEntry) -> bool {
    family
        .family_key
        .strip_prefix(deskhud_ui::DEFAULT_UI_FONT_FAMILY)
        .is_some_and(|suffix| suffix.is_empty() || suffix.chars().all(|ch| ch.is_ascii_lowercase()))
}

fn resolve_font_path(path: &str) -> std::path::PathBuf {
    let path = std::path::Path::new(path);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    let Some(executable_dir) = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(std::path::Path::to_path_buf))
    else {
        return path.to_path_buf();
    };
    let executable_path = executable_dir.join(path);
    if executable_path.exists() {
        return executable_path;
    }
    if executable_dir.file_name().and_then(|name| name.to_str()) == Some("MacOS") {
        if let Some(contents_dir) = executable_dir.parent() {
            let resource_path = contents_dir.join("Resources").join(path);
            if resource_path.exists() {
                return resource_path;
            }
        }
    }
    executable_path
}
