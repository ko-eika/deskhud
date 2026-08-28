//! Toolkit-independent font container metadata.

mod scan;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "macos.rs"]
mod platform;
#[cfg(all(unix, not(target_os = "macos")))]
#[path = "unix.rs"]
mod platform;
#[cfg(not(any(windows, unix)))]
#[path = "fallback.rs"]
mod platform;

pub use scan::{font_families_from_dirs, system_font_families};

use std::fs;
use std::path::Path;

use crate::LanguageTag;

/// Metadata for one face in a TTF/OTF file or TTC collection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontContainerFace {
    /// Index within the collection; zero for a standalone font.
    pub face_index: u32,
    /// Family name from the font `name` table.
    pub family: Option<String>,
    /// Subfamily/style name from the font `name` table.
    pub subfamily: Option<String>,
    /// PostScript name from the font `name` table.
    pub postscript: Option<String>,
    /// All localized family names from the font `name` table.
    pub family_names: Vec<FontNameRecord>,
    /// All localized subfamily names from the font `name` table.
    pub subfamily_names: Vec<FontNameRecord>,
}

/// A localized string from an OpenType `name` table record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontNameRecord {
    /// OpenType name table identifier, for example 1 or 16.
    pub name_id: u16,
    /// OpenType platform identifier.
    pub platform_id: u16,
    /// Platform-specific language identifier.
    pub language_id: u16,
    /// Decoded name value.
    pub value: String,
}

/// A font face usable by a UI backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFace {
    /// Style name, such as `Regular` or `Bold Italic`.
    pub style: String,
    /// Backend-specific source identifier.
    pub font_id: String,
    /// Whether the face is supplied by the application rather than the OS.
    pub builtin: bool,
}

/// A family containing one or more font faces.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFamilyEntry {
    /// Stable normalized family key.
    pub family_key: String,
    /// Display label.
    pub label: String,
    /// Search aliases.
    pub search_terms: Vec<String>,
    /// Available faces.
    pub faces: Vec<FontFace>,
    /// Localized family names collected from all faces in the family.
    pub localized_names: Vec<FontNameRecord>,
}

/// Platform-neutral font family catalog used by scanners and UI adapters.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct FontCatalog {
    families: std::collections::BTreeMap<String, FontFamilyEntry>,
}

impl FontCatalog {
    /// Adds a face and merges it with an existing family/style entry.
    pub fn upsert(
        &mut self,
        family_key: String,
        label: String,
        aliases: impl IntoIterator<Item = String>,
        face: FontFace,
    ) {
        self.upsert_with_names(family_key, label, aliases, [], face);
    }

    /// Adds a face and its localized family names from font metadata.
    pub fn upsert_with_names(
        &mut self,
        family_key: String,
        label: String,
        aliases: impl IntoIterator<Item = String>,
        names: impl IntoIterator<Item = FontNameRecord>,
        face: FontFace,
    ) {
        let entry = self
            .families
            .entry(family_key.clone())
            .or_insert_with(|| FontFamilyEntry {
                family_key,
                label: label.clone(),
                search_terms: Vec::new(),
                faces: Vec::new(),
                localized_names: Vec::new(),
            });
        if entry.label.is_empty() {
            entry.label = label;
        }
        for name in names {
            if !entry
                .localized_names
                .iter()
                .any(|existing| existing == &name)
            {
                entry.localized_names.push(name);
            }
        }
        entry.upsert_face(face, aliases);
    }

    /// Merges already-scanned families while preserving bundled faces over
    /// system faces with the same family and style.
    pub fn extend(&mut self, families: impl IntoIterator<Item = FontFamilyEntry>) {
        for family in families {
            for face in family.faces {
                self.upsert_with_names(
                    family.family_key.clone(),
                    family.label.clone(),
                    family.search_terms.clone(),
                    family.localized_names.clone(),
                    face,
                );
            }
        }
    }

    /// Returns merged families sorted for display.
    pub fn into_entries(self) -> Vec<FontFamilyEntry> {
        let mut entries: Vec<_> = self.families.into_values().collect();
        for family in &mut entries {
            family.faces.sort_by_key(|face| style_sort_key(&face.style));
        }
        entries.sort_by_key(|family| family.label.to_lowercase());
        entries
    }
}

/// Returns the canonical identifier used for a font file supplied by the OS.
///
/// The identifier is deliberately a normalized path: the UI backend can use it
/// to load the file later, while this crate remains independent of any window
/// or rendering toolkit.
pub fn system_font_id(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Merges bundled and system families, preserving bundled faces on conflicts.
pub fn merge_font_families(
    builtin: impl IntoIterator<Item = FontFamilyEntry>,
    system: impl IntoIterator<Item = FontFamilyEntry>,
) -> Vec<FontFamilyEntry> {
    let mut catalog = FontCatalog::default();
    catalog.extend(builtin);
    catalog.extend(system);
    catalog.into_entries()
}

/// A selected face together with the family it belongs to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontSelection {
    /// Stable family key.
    pub family_key: String,
    /// Display family label.
    pub family_label: String,
    /// Selected style.
    pub style: String,
    /// Backend source identifier.
    pub font_id: String,
    /// Whether the face is bundled with the application.
    pub builtin: bool,
}

/// Filters font families by actual glyph coverage for a locale.
pub fn font_families_for_locale(
    locale: &LanguageTag,
    families: &[FontFamilyEntry],
) -> Vec<FontFamilyEntry> {
    families
        .iter()
        .filter(|family| {
            family
                .faces
                .iter()
                .any(|face| face_supports_locale(face, family, locale))
        })
        .cloned()
        .collect()
}

/// Scans the installed fonts and returns only families covering `locale`.
pub fn system_font_families_for_locale(locale: &LanguageTag) -> Vec<FontFamilyEntry> {
    font_families_for_locale(locale, &system_font_families())
}

/// Scans the installed fonts and selects the first face covering `locale`.
pub fn select_system_font_for_locale(locale: &LanguageTag) -> Option<FontSelection> {
    select_font_for_locale(locale, &[], &system_font_families())
}

/// Selects the first suitable face, preferring bundled faces over system faces.
pub fn select_font_for_locale(
    locale: &LanguageTag,
    builtin: &[FontFamilyEntry],
    system: &[FontFamilyEntry],
) -> Option<FontSelection> {
    builtin
        .iter()
        .chain(system)
        .filter_map(|family| {
            family
                .faces
                .iter()
                .find(|face| face_supports_locale(face, family, locale))
                .map(|face| FontSelection {
                    family_key: family.family_key.clone(),
                    family_label: family.label.clone(),
                    style: face.style.clone(),
                    font_id: face.font_id.clone(),
                    builtin: face.builtin,
                })
        })
        .next()
}

/// Uses the detected system locale and the same bundled-first policy.
pub fn select_default_font(
    builtin: &[FontFamilyEntry],
    system: &[FontFamilyEntry],
) -> Option<FontSelection> {
    select_font_for_locale(&crate::current_system_locale(), builtin, system)
}

fn face_supports_locale(face: &FontFace, family: &FontFamilyEntry, locale: &LanguageTag) -> bool {
    if face.builtin {
        // Bundled faces are not necessarily file-backed. Use known family
        // aliases for them; callers that have bytes can pre-filter entries
        // with `face_supports_text` before calling the selector.
        return builtin_family_supports(family, locale);
    }
    let (path, index) = face
        .font_id
        .split_once("#face=")
        .map_or((face.font_id.as_str(), 0), |(path, index)| {
            (path, index.parse().unwrap_or(0))
        });
    std::fs::read(path)
        .ok()
        .is_some_and(|bytes| face_supports_text(&bytes, index, locale.sample_text()))
}

fn builtin_family_supports(family: &FontFamilyEntry, locale: &LanguageTag) -> bool {
    let haystack = std::iter::once(family.label.as_str())
        .chain(family.search_terms.iter().map(String::as_str))
        .collect::<Vec<_>>()
        .join(" ")
        .to_ascii_lowercase();
    match locale.language.as_str() {
        "zh" => {
            haystack.contains("noto sans sc")
                || haystack.contains("cjk")
                || haystack.contains("source han")
                || haystack.contains("yahei")
                || haystack.contains("pingfang")
        }
        "ja" => {
            haystack.contains("noto")
                || haystack.contains("cjk")
                || haystack.contains("source han")
                || haystack.contains("gothic")
                || haystack.contains("hiragino")
        }
        "ko" => {
            haystack.contains("noto")
                || haystack.contains("cjk")
                || haystack.contains("source han")
                || haystack.contains("malgun")
        }
        _ => true,
    }
}

impl FontFamilyEntry {
    /// Merges aliases and de-duplicates a face by normalized style.
    pub fn upsert_face(&mut self, face: FontFace, aliases: impl IntoIterator<Item = String>) {
        for alias in aliases {
            if !self.search_terms.iter().any(|existing| existing == &alias) {
                self.search_terms.push(alias);
            }
        }
        let style = normalize_style_name(&face.style);
        if let Some(existing) = self
            .faces
            .iter_mut()
            .find(|existing| normalize_style_name(&existing.style) == style)
        {
            if face.builtin && !existing.builtin {
                *existing = face;
            }
        } else {
            self.faces.push(face);
        }
    }
}

impl FontFamilyEntry {
    /// Finds the requested style, falling back to Regular or the first face.
    pub fn face_for(&self, style: &str) -> Option<&FontFace> {
        let wanted = normalize_style(style);
        self.faces
            .iter()
            .find(|face| normalize_style(&face.style) == wanted)
            .or_else(|| {
                self.faces
                    .iter()
                    .find(|face| normalize_style(&face.style) == "regular")
            })
            .or_else(|| self.faces.first())
    }

    /// Returns unique styles in stable weight order.
    pub fn style_names(&self) -> Vec<String> {
        let mut styles: Vec<_> = self.faces.iter().map(|face| face.style.clone()).collect();
        styles.sort_by_key(|style| style_sort_key(style));
        styles.dedup();
        styles
    }

    /// Selects the closest localized family name, then falls back to the first
    /// metadata name and finally the existing display label.
    pub fn label_for_locale(&self, locale: &crate::LanguageTag) -> String {
        self.localized_names
            .iter()
            .filter_map(|name| {
                language_match_score(name, locale).map(|score| (score, name.name_id, &name.value))
            })
            .max_by_key(|(score, name_id, value)| {
                (
                    *score,
                    *name_id == 16,
                    std::cmp::Reverse(value.chars().count()),
                )
            })
            .map(|(_, _, value)| value.clone())
            .or_else(|| self.localized_names.first().map(|name| name.value.clone()))
            .unwrap_or_else(|| self.label.clone())
    }
}

fn language_match_score(name: &FontNameRecord, locale: &crate::LanguageTag) -> Option<u8> {
    if name.platform_id == 0 {
        return Some(1);
    }
    if name.platform_id != 3 {
        return None;
    }
    let primary = name.language_id & 0x03ff;
    let wanted = match locale.language.as_str() {
        "en" => 0x09,
        "zh" => 0x04,
        "ja" => 0x11,
        "ko" => 0x12,
        "de" => 0x07,
        "fr" => 0x0c,
        "es" => 0x0a,
        "ru" => 0x19,
        _ => return Some(1),
    };
    if primary != wanted {
        return None;
    }
    let region_match = match locale.region.as_deref() {
        Some("US") => name.language_id == 0x0409,
        Some("GB") => name.language_id == 0x0809,
        Some("CN") => name.language_id == 0x0804,
        Some("TW") => name.language_id == 0x0404,
        Some("HK") => name.language_id == 0x0c04,
        _ => false,
    };
    // Typographic Family Name (16) is the family grouping used when a font
    // separates weight/style into Subfamily (17). Legacy Family Name (1) may
    // contain the style itself, e.g. "Family Light".
    let family_name_bonus = if name.name_id == 16 { 2 } else { 1 };
    Some((if region_match { 3 } else { 2 }) * 3 + family_name_bonus)
}

fn normalize_style(style: &str) -> String {
    style.to_ascii_lowercase().replace(' ', "")
}

/// Normalizes localized and common font style names to stable identifiers.
pub fn normalize_style_name(s: &str) -> String {
    match s.trim().to_ascii_lowercase().as_str() {
        "" | "regular" | "normal" => "Regular".into(),
        "bold" | "negreta" | "negrita" | "gras" | "fett" | "grassetto" => "Bold".into(),
        "light" => "Light".into(),
        "thin" => "Thin".into(),
        "demilight" | "demi light" => "DemiLight".into(),
        "medium" | "book" => "Medium".into(),
        "semibold" | "semi bold" | "demibold" => "SemiBold".into(),
        "extrabold" | "extra bold" => "ExtraBold".into(),
        "extralight" | "extra light" => "ExtraLight".into(),
        "black" | "heavy" => "Black".into(),
        other => {
            if s.chars().any(|c| c.is_uppercase()) {
                s.to_string()
            } else {
                let mut chars = other.chars();
                chars
                    .next()
                    .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
                    .unwrap_or_default()
            }
        }
    }
}

/// Classifies a font filename stem into a stable family key, display label,
/// style and searchable aliases.
pub fn classify_stem(stem: &str) -> (String, String, String, Vec<String>) {
    if let Some((family, raw_style)) = stem.split_once('-') {
        let (family_key, label, _, aliases) = classify_stem(family);
        let style = raw_style
            .replace("BoldItalic", "Bold Italic")
            .replace("ExtraBoldItalic", "ExtraBold Italic")
            .replace("SemiBoldItalic", "SemiBold Italic");
        return (family_key, label, style, aliases);
    }
    let lower = stem.to_ascii_lowercase();
    let (family_part, style) = [
        ("bolditalic", "Bold Italic"),
        ("extrabolditalic", "ExtraBold Italic"),
        ("semibolditalic", "SemiBold Italic"),
        ("mediumitalic", "Medium Italic"),
        ("lightitalic", "Light Italic"),
        ("extralightitalic", "ExtraLight Italic"),
        ("thinitalic", "Thin Italic"),
        ("extrabold", "ExtraBold"),
        ("semibold", "SemiBold"),
        ("extralight", "ExtraLight"),
        ("bold", "Bold"),
        ("medium", "Medium"),
        ("light", "Light"),
        ("thin", "Thin"),
        ("italic", "Italic"),
        ("regular", "Regular"),
    ]
    .iter()
    .find_map(|(suffix, style)| lower.strip_suffix(suffix).map(|base| (base, *style)))
    .unwrap_or((lower.as_str(), "Regular"));
    let family_key: String = family_part
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let family_key = family_key.to_ascii_lowercase();
    let label = match family_key.as_str() {
        "jetbrainsmono" => "JetBrains Mono".into(),
        "jetbrainsmononl" => "JetBrains Mono NL".into(),
        "notosanssc" => "Noto Sans SC".into(),
        "notosans" => "Noto Sans".into(),
        _ => humanize_family_code(family_part),
    };
    let mut aliases = vec![
        label.to_lowercase(),
        family_key.clone(),
        style.to_lowercase(),
    ];
    aliases.push(stem.to_ascii_lowercase());
    (family_key, label, style.into(), aliases)
}

fn humanize_family_code(code: &str) -> String {
    let chars: Vec<char> = code.chars().collect();
    let mut out = String::new();
    for (index, ch) in chars.iter().enumerate() {
        if index > 0 && ch.is_uppercase() {
            let previous = chars[index - 1];
            let next_lower = chars.get(index + 1).is_some_and(|next| next.is_lowercase());
            if previous.is_lowercase() || next_lower {
                out.push(' ');
            }
        }
        out.push(*ch);
    }
    let mut chars = out.chars();
    chars
        .next()
        .map(|first| first.to_uppercase().collect::<String>() + chars.as_str())
        .unwrap_or_default()
}

/// Returns a stable weight/italic sort key for a style name.
pub fn style_sort_key(style: &str) -> (u8, u8) {
    let lower = normalize_style(style);
    let italic = u8::from(lower.contains("italic"));
    let base = lower.replace("italic", "");
    let weight = match base.as_str() {
        "thin" => 0,
        "extralight" => 1,
        "light" => 2,
        "regular" => 4,
        "medium" => 5,
        "semibold" => 6,
        "bold" => 7,
        "extrabold" => 8,
        "black" | "heavy" => 9,
        _ => 40,
    };
    (weight, italic)
}

/// Reads font-container metadata without loading glyph data or depending on a UI toolkit.
pub fn inspect_font_file(path: impl AsRef<Path>) -> std::io::Result<Vec<FontContainerFace>> {
    let bytes = fs::read(path)?;
    inspect_font_bytes(&bytes)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))
}

/// Reads font-container metadata from TTF/OTF/TTC bytes.
pub fn inspect_font_bytes(bytes: &[u8]) -> Result<Vec<FontContainerFace>, String> {
    let mut faces = Vec::new();
    for face_index in 0.. {
        let Ok(face) = ttf_parser::Face::parse(bytes, face_index) else {
            break;
        };
        let mut family_names = Vec::new();
        let mut subfamily_names = Vec::new();
        for name in face.names() {
            if !matches!(name.name_id, 1 | 2 | 16 | 17) {
                continue;
            }
            let Some(value) = name.to_string() else {
                continue;
            };
            let record = FontNameRecord {
                name_id: name.name_id,
                platform_id: name.platform_id as u16,
                language_id: name.language_id,
                value,
            };
            if matches!(name.name_id, 1 | 16) {
                family_names.push(record);
            } else {
                subfamily_names.push(record);
            }
        }
        let family = preferred_name(&face, &[16, 1]);
        let subfamily = preferred_name(&face, &[17, 2]);
        let postscript = preferred_name(&face, &[6]);
        faces.push(FontContainerFace {
            face_index,
            family,
            subfamily,
            postscript,
            family_names,
            subfamily_names,
        });
    }
    if faces.is_empty() {
        Err("font has no parseable faces".into())
    } else {
        Ok(faces)
    }
}

fn preferred_name(face: &ttf_parser::Face<'_>, ids: &[u16]) -> Option<String> {
    ids.iter().find_map(|wanted| {
        face.names()
            .into_iter()
            .filter(|name| name.name_id == *wanted)
            .find_map(|name| name.to_string())
    })
}

/// Returns whether a specific face contains glyphs for every character in `text`.
pub fn face_supports_text(bytes: &[u8], face_index: u32, text: &str) -> bool {
    let Ok(face) = ttf_parser::Face::parse(bytes, face_index) else {
        return false;
    };
    text.chars()
        .all(|character| face.glyph_index(character).is_some())
}

#[cfg(test)]
mod tests {
    use super::{
        FontCatalog, FontFace, FontFamilyEntry, classify_stem, face_supports_text,
        font_families_from_dirs, inspect_font_file, normalize_style_name, select_font_for_locale,
    };
    use crate::LanguageTag;

    #[test]
    fn classifies_common_font_stems_and_localized_styles() {
        let (family, label, style, _) = classify_stem("Inter-BoldItalic");
        assert_eq!(family, "inter");
        assert_eq!(label, "Inter");
        assert_eq!(style, "Bold Italic");
        assert_eq!(normalize_style_name("Negreta"), "Bold");
        assert_eq!(normalize_style_name("Normal"), "Regular");
    }

    #[test]
    fn scans_external_source_han_sans_collection() {
        let directory =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../assets/fonts");
        let families = font_families_from_dirs([directory]);
        assert!(families.iter().any(|family| {
            family.family_key == "sourcehansans" && family.faces.iter().any(|face| face.builtin)
        }));
    }

    #[test]
    fn font_catalog_merges_styles_and_prefers_builtin_faces() {
        let mut catalog = FontCatalog::default();
        catalog.upsert(
            "demo".into(),
            "Demo".into(),
            ["demo".into()],
            FontFace {
                style: "Regular".into(),
                font_id: "system-demo".into(),
                builtin: false,
            },
        );
        catalog.upsert(
            "demo".into(),
            "Demo".into(),
            ["demo".into()],
            FontFace {
                style: "Normal".into(),
                font_id: "builtin-demo".into(),
                builtin: true,
            },
        );
        let entries = catalog.into_entries();
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].faces.len(), 1);
        assert_eq!(entries[0].faces[0].font_id, "builtin-demo");
    }

    #[test]
    fn selects_the_first_builtin_face_that_matches_the_locale() {
        let builtin = vec![
            FontFamilyEntry {
                family_key: "latin".into(),
                label: "Latin Font".into(),
                search_terms: vec![],
                faces: vec![FontFace {
                    style: "Regular".into(),
                    font_id: "latin".into(),
                    builtin: true,
                }],
                localized_names: vec![],
            },
            FontFamilyEntry {
                family_key: "notosanssc".into(),
                label: "Noto Sans SC".into(),
                search_terms: vec![],
                faces: vec![FontFace {
                    style: "Regular".into(),
                    font_id: "noto".into(),
                    builtin: true,
                }],
                localized_names: vec![],
            },
        ];
        let selection =
            select_font_for_locale(&LanguageTag::parse("zh-CN").unwrap(), &builtin, &[]).unwrap();
        assert_eq!(selection.family_key, "notosanssc");
    }

    #[cfg(windows)]
    #[test]
    fn inspects_inter_collection_faces_when_available() {
        let path =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/Inter.ttc");
        let Ok(faces) = inspect_font_file(path) else {
            return;
        };
        assert_eq!(faces.len(), 36);
        assert!(faces.iter().all(|face| face.face_index < 36));
        assert!(
            faces
                .iter()
                .any(|face| face.family.as_deref() == Some("Inter"))
        );
        let styles: std::collections::BTreeSet<_> = faces
            .iter()
            .filter_map(|face| face.subfamily.as_deref())
            .collect();
        assert!(
            styles.len() > 1,
            "TTC faces did not expose distinct styles: {styles:?}"
        );
        let bytes = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/fonts/Inter.ttc"),
        )
        .unwrap();
        assert!(face_supports_text(&bytes, 0, "Inter"));
    }
}
