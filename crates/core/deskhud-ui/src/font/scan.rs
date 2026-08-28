//! System font discovery, independent from a UI toolkit.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::platform::{font_dirs, priority_system_cjk};
use super::{
    FontCatalog, FontFace, FontFamilyEntry, classify_stem, inspect_font_file, system_font_id,
};

const MAX_SYSTEM_FONTS: usize = 480;

/// Scans installed TTF, OTF and TTC files and groups their faces by family.
///
/// Discovery is best-effort: inaccessible directories and malformed files are
/// skipped so that a single system font cannot prevent the settings page from
/// opening.
pub fn system_font_families() -> Vec<FontFamilyEntry> {
    let mut catalog = FontCatalog::default();
    let mut seen = BTreeSet::new();
    let mut face_count = 0;

    for path in priority_system_cjk() {
        ingest(&path, &mut catalog, &mut seen, &mut face_count, false);
    }
    for directory in font_dirs() {
        collect(&directory, &mut catalog, &mut seen, &mut face_count, false);
        if face_count >= MAX_SYSTEM_FONTS {
            break;
        }
    }
    catalog.into_entries()
}

/// Scans application-provided font directories.
///
/// Files in these directories are marked as application fonts so they win
/// over an OS font with the same family and style. The paths remain external
/// and are loaded only when the UI selects a face.
pub fn font_families_from_dirs(
    directories: impl IntoIterator<Item = PathBuf>,
) -> Vec<FontFamilyEntry> {
    let mut catalog = FontCatalog::default();
    let mut seen = BTreeSet::new();
    let mut face_count = 0;
    for directory in directories {
        collect(&directory, &mut catalog, &mut seen, &mut face_count, true);
        if face_count >= MAX_SYSTEM_FONTS {
            break;
        }
    }
    catalog.into_entries()
}

fn collect(
    path: &Path,
    catalog: &mut FontCatalog,
    seen: &mut BTreeSet<String>,
    count: &mut usize,
    application_font: bool,
) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        if *count >= MAX_SYSTEM_FONTS {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            collect(&path, catalog, seen, count, application_font);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        {
            ingest(&path, catalog, seen, count, application_font);
        }
    }
}

fn ingest(
    path: &Path,
    catalog: &mut FontCatalog,
    seen: &mut BTreeSet<String>,
    count: &mut usize,
    application_font: bool,
) {
    let Some(stem) = path.file_stem().and_then(|value| value.to_str()) else {
        return;
    };
    let lower = stem.to_ascii_lowercase();
    if [
        "emoji",
        "symbol",
        "webdings",
        "wingdings",
        "marlett",
        "segmdl2",
        "holomdl2",
    ]
    .iter()
    .any(|part| lower.contains(part))
    {
        return;
    }

    let Some(faces) = inspect_font_file(path).ok() else {
        add_face(path, stem, None, catalog, seen, count, application_font);
        return;
    };
    for face in faces {
        if *count >= MAX_SYSTEM_FONTS {
            return;
        }
        add_face(
            path,
            stem,
            Some(face),
            catalog,
            seen,
            count,
            application_font,
        );
    }
}

fn add_face(
    path: &Path,
    stem: &str,
    container: Option<super::FontContainerFace>,
    catalog: &mut FontCatalog,
    seen: &mut BTreeSet<String>,
    count: &mut usize,
    application_font: bool,
) {
    let id = match container.as_ref() {
        Some(face) => format!("{}#face={}", system_font_id(path), face.face_index),
        None => system_font_id(path),
    };
    if !seen.insert(id.clone()) {
        return;
    }
    let family_stem = container
        .as_ref()
        .and_then(|face| face.family.as_deref())
        .unwrap_or(stem);
    let (family, label, parsed_style, mut aliases) = classify_stem(family_stem);
    let style = container
        .as_ref()
        .and_then(|face| face.subfamily.clone())
        .unwrap_or(parsed_style);
    let localized_names = container
        .as_ref()
        .map(|face| face.family_names.clone())
        .unwrap_or_default();
    aliases.push(label.to_lowercase());
    aliases.push(family.clone());
    catalog.upsert_with_names(
        family,
        label,
        aliases,
        localized_names,
        FontFace {
            style,
            font_id: id,
            builtin: application_font,
        },
    );
    *count += 1;
}
