//! System font discovery, independent from a UI toolkit.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

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
        ingest(&path, &mut catalog, &mut seen, &mut face_count);
    }
    for directory in font_dirs() {
        collect(&directory, &mut catalog, &mut seen, &mut face_count);
        if face_count >= MAX_SYSTEM_FONTS {
            break;
        }
    }
    catalog.into_entries()
}

#[cfg(windows)]
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    } else {
        dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join(r"Microsoft\Windows\Fonts"));
    }
    dirs
}

#[cfg(target_os = "macos")]
fn font_dirs() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts",
        "/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

#[cfg(all(unix, not(target_os = "macos")))]
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/usr/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
    ];
    if let Ok(home) = std::env::var("HOME") {
        dirs.push(PathBuf::from(&home).join(".local/share/fonts"));
        dirs.push(PathBuf::from(home).join(".fonts"));
    }
    dirs
}

#[cfg(not(any(windows, unix, target_os = "macos")))]
fn font_dirs() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(windows)]
fn priority_system_cjk() -> Vec<PathBuf> {
    [
        "msyh.ttc",
        "msyh.ttf",
        "msyhbd.ttc",
        "simhei.ttf",
        "simsun.ttc",
        "msyhl.ttc",
    ]
    .into_iter()
    .filter_map(|name| {
        font_dirs()
            .into_iter()
            .map(|dir| dir.join(name))
            .find(|path| path.is_file())
    })
    .collect()
}

#[cfg(not(windows))]
fn priority_system_cjk() -> Vec<PathBuf> {
    [
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
    ]
    .into_iter()
    .map(PathBuf::from)
    .filter(|path| path.is_file())
    .collect()
}

fn collect(path: &Path, catalog: &mut FontCatalog, seen: &mut BTreeSet<String>, count: &mut usize) {
    let Ok(entries) = std::fs::read_dir(path) else {
        return;
    };
    for entry in entries.flatten() {
        if *count >= MAX_SYSTEM_FONTS {
            return;
        }
        let path = entry.path();
        if path.is_dir() {
            collect(&path, catalog, seen, count);
        } else if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext.to_ascii_lowercase().as_str(), "ttf" | "otf" | "ttc"))
        {
            ingest(&path, catalog, seen, count);
        }
    }
}

fn ingest(path: &Path, catalog: &mut FontCatalog, seen: &mut BTreeSet<String>, count: &mut usize) {
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
        add_face(path, stem, None, catalog, seen, count);
        return;
    };
    for face in faces {
        if *count >= MAX_SYSTEM_FONTS {
            return;
        }
        add_face(path, stem, Some(face), catalog, seen, count);
    }
}

fn add_face(
    path: &Path,
    stem: &str,
    container: Option<super::FontContainerFace>,
    catalog: &mut FontCatalog,
    seen: &mut BTreeSet<String>,
    count: &mut usize,
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
        .and_then(|face| face.subfamily)
        .unwrap_or(parsed_style);
    aliases.push(label.to_lowercase());
    aliases.push(family.clone());
    catalog.upsert(
        family,
        label,
        aliases,
        FontFace {
            style,
            font_id: id,
            builtin: false,
        },
    );
    *count += 1;
}
