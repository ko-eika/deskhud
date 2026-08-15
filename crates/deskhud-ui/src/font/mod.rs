//! Toolkit-independent font container metadata.

use std::fs;
use std::path::Path;

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
}

/// A font face usable by a UI backend.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFace {
    /// Style name, such as `Regular` or `Bold Italic`.
    pub style: String,
    /// Backend-specific source identifier.
    pub font_id: String,
    /// Whether the face is bundled with the application.
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
        let entry = self
            .families
            .entry(family_key.clone())
            .or_insert_with(|| FontFamilyEntry {
                family_key,
                label: label.clone(),
                search_terms: Vec::new(),
                faces: Vec::new(),
            });
        if entry.label.is_empty() {
            entry.label = label;
        }
        entry.upsert_face(face, aliases);
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

type FontNames = (Option<String>, Option<String>, Option<String>);

/// Reads font-container metadata without loading glyph data or depending on a UI toolkit.
pub fn inspect_font_file(path: impl AsRef<Path>) -> std::io::Result<Vec<FontContainerFace>> {
    let bytes = fs::read(path)?;
    inspect_font_bytes(&bytes)
        .map_err(|message| std::io::Error::new(std::io::ErrorKind::InvalidData, message))
}

/// Reads font-container metadata from TTF/OTF/TTC bytes.
pub fn inspect_font_bytes(bytes: &[u8]) -> Result<Vec<FontContainerFace>, String> {
    let offsets = if bytes.get(0..4) == Some(b"ttcf") {
        let count = be_u32(bytes, 8)?;
        (0..count)
            .map(|i| be_u32(bytes, 12 + i * 4))
            .collect::<Result<Vec<_>, _>>()?
    } else {
        vec![0]
    };
    offsets
        .into_iter()
        .enumerate()
        .map(|(index, offset)| {
            let tables = table_directory(bytes, offset)?;
            let name = tables
                .get(b"name".as_slice())
                .ok_or("font has no name table")?;
            let (family, subfamily, postscript) = read_names(bytes, name.0, name.1)?;
            Ok(FontContainerFace {
                face_index: index as u32,
                family,
                subfamily,
                postscript,
            })
        })
        .collect()
}

/// Returns whether a specific face contains glyphs for every character in `text`.
pub fn face_supports_text(bytes: &[u8], face_index: u32, text: &str) -> bool {
    let Ok(face) = ttf_parser::Face::parse(bytes, face_index) else {
        return false;
    };
    text.chars()
        .all(|character| face.glyph_index(character).is_some())
}

fn be_u16(bytes: &[u8], at: u32) -> Result<u16, String> {
    let at = at as usize;
    bytes
        .get(at..at + 2)
        .map(|v| u16::from_be_bytes([v[0], v[1]]))
        .ok_or_else(|| "truncated font".into())
}

fn be_u32(bytes: &[u8], at: u32) -> Result<u32, String> {
    let at = at as usize;
    bytes
        .get(at..at + 4)
        .map(|v| u32::from_be_bytes([v[0], v[1], v[2], v[3]]))
        .ok_or_else(|| "truncated font".into())
}

fn table_directory(
    bytes: &[u8],
    offset: u32,
) -> Result<std::collections::BTreeMap<Vec<u8>, (u32, u32)>, String> {
    let count = be_u16(bytes, offset + 4)? as u32;
    let mut tables = std::collections::BTreeMap::new();
    for i in 0..count {
        let at = offset + 12 + i * 16;
        let tag = bytes
            .get(at as usize..at as usize + 4)
            .ok_or("truncated table directory")?
            .to_vec();
        tables.insert(tag, (be_u32(bytes, at + 8)?, be_u32(bytes, at + 12)?));
    }
    Ok(tables)
}

fn read_names(bytes: &[u8], offset: u32, length: u32) -> Result<FontNames, String> {
    let end = offset.checked_add(length).ok_or("invalid name table")?;
    let count = be_u16(bytes, offset + 2)? as u32;
    let string_offset = be_u16(bytes, offset + 4)? as u32;
    // Prefer OpenType typographic family/subfamily (16/17). Some collections
    // such as Inter use those names to distinguish every weight, while the
    // legacy family/subfamily names (1/2) intentionally collapse them.
    let mut values: [Option<String>; 3] = [None, None, None];
    let mut legacy: [Option<String>; 3] = [None, None, None];
    for i in 0..count {
        let at = offset + 6 + i * 12;
        let platform = be_u16(bytes, at)?;
        let name_id = be_u16(bytes, at + 6)?;
        let length = be_u16(bytes, at + 8)? as u32;
        let relative = be_u16(bytes, at + 10)? as u32;
        if !(name_id == 1 || name_id == 2 || name_id == 6 || name_id == 16 || name_id == 17) {
            continue;
        }
        let start = offset + string_offset + relative;
        let finish = start.checked_add(length).ok_or("invalid name record")?;
        if finish > end {
            continue;
        }
        let raw = &bytes[start as usize..finish as usize];
        let text = if platform == 0 || platform == 3 {
            decode_utf16(raw)
        } else {
            String::from_utf8_lossy(raw).into_owned()
        };
        let (slot, target) = match name_id {
            1 => (0, &mut legacy),
            2 => (1, &mut legacy),
            6 => (2, &mut values),
            16 => (0, &mut values),
            17 => (1, &mut values),
            _ => unreachable!(),
        };
        if target[slot].is_none() && !text.is_empty() {
            target[slot] = Some(text);
        }
    }
    if values[0].is_none() {
        values[0] = legacy[0].take();
    }
    if values[1].is_none() {
        values[1] = legacy[1].take();
    }
    Ok((values[0].take(), values[1].take(), values[2].take()))
}

fn decode_utf16(raw: &[u8]) -> String {
    raw.chunks_exact(2)
        .map(|v| u16::from_be_bytes([v[0], v[1]]))
        .collect::<Vec<_>>()
        .iter()
        .copied()
        .map(|v| char::from_u32(v as u32).unwrap_or('\u{FFFD}'))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        FontCatalog, FontFace, classify_stem, face_supports_text, inspect_font_file,
        normalize_style_name,
    };

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
    fn inspects_inter_collection_faces_when_available() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fonts/Inter.ttc");
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
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fonts/Inter.ttc"),
        )
        .unwrap();
        assert!(face_supports_text(&bytes, 0, "Inter"));
    }
}
