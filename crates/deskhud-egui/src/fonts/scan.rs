//! 扫描系统字体目录。

use std::path::{Path, PathBuf};

use super::{FontFace, FontFamilyEntry, classify_stem, system_font_id};
use deskhud_ui::font::FontCatalog;
use deskhud_ui::font::inspect_font_file;

const MAX_SYSTEM_FONTS: usize = 480;

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
    vec![
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts/Supplemental"),
    ]
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

#[cfg(windows)]
pub(super) fn priority_system_cjk() -> Vec<(String, PathBuf)> {
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
        for dir in font_dirs() {
            let p = dir.join(name);
            if p.is_file() {
                return Some((
                    p.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or(name)
                        .to_string(),
                    p,
                ));
            }
        }
        None
    })
    .collect()
}

#[cfg(not(windows))]
pub(super) fn priority_system_cjk() -> Vec<(String, PathBuf)> {
    const CANDIDATES: &[&str] = &[
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
    ];
    CANDIDATES
        .iter()
        .filter_map(|p| {
            let path = PathBuf::from(p);
            path.is_file().then(|| {
                (
                    path.file_stem()
                        .and_then(|s| s.to_str())
                        .unwrap_or("cjk")
                        .to_string(),
                    path,
                )
            })
        })
        .collect()
}

/// 扫描系统字体并按家族聚合（键为规范化家族码，无前缀）。
pub fn system_font_families() -> Vec<FontFamilyEntry> {
    let mut catalog = FontCatalog::default();
    let mut seen_paths = std::collections::BTreeSet::new();
    let mut file_count = 0usize;

    for (_, path) in priority_system_cjk() {
        ingest_font_path(&path, &mut catalog, &mut seen_paths, &mut file_count);
    }

    for dir in font_dirs() {
        collect_fonts_dir(&dir, &mut catalog, &mut seen_paths, &mut file_count);
        if file_count >= MAX_SYSTEM_FONTS {
            break;
        }
    }

    catalog.into_entries()
}

fn collect_fonts_dir(
    dir: &Path,
    catalog: &mut FontCatalog,
    seen_paths: &mut std::collections::BTreeSet<String>,
    file_count: &mut usize,
) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        if *file_count >= MAX_SYSTEM_FONTS {
            break;
        }
        let path = entry.path();
        if path.is_dir() {
            collect_fonts_dir(&path, catalog, seen_paths, file_count);
            continue;
        }
        let Some(ext) = path.extension().and_then(|e| e.to_str()) else {
            continue;
        };
        let ext = ext.to_ascii_lowercase();
        if !matches!(ext.as_str(), "ttf" | "otf" | "ttc") {
            continue;
        }
        ingest_font_path(&path, catalog, seen_paths, file_count);
    }
}

fn ingest_font_path(
    path: &Path,
    catalog: &mut FontCatalog,
    seen_paths: &mut std::collections::BTreeSet<String>,
    file_count: &mut usize,
) {
    if !path.is_file() {
        return;
    }
    let Some(stem) = path.file_stem().and_then(|s| s.to_str()) else {
        return;
    };
    let lower = stem.to_ascii_lowercase();
    if lower.contains("emoji")
        || lower.contains("symbol")
        || lower.contains("webdings")
        || lower.contains("wingdings")
        || lower.contains("marlett")
        || lower.contains("segmdl2")
        || lower.contains("holomdl2")
    {
        return;
    }
    let container_faces = inspect_font_file(path).ok();
    let has_container = container_faces.is_some();
    for container in container_faces.into_iter().flatten() {
        let id = format!("{}#face={}", system_font_id(path), container.face_index);
        if !seen_paths.insert(id.clone()) {
            continue;
        }
        *file_count += 1;
        let family_stem = container.family.as_deref().unwrap_or(stem);
        let (fam_code, display, parsed_style, aliases) = classify_stem(family_stem);
        let style = container.subfamily.clone().unwrap_or(parsed_style);
        let mut aliases = aliases;
        aliases.push(display.to_lowercase());
        aliases.push(fam_code.clone());
        catalog.upsert(
            fam_code,
            display,
            aliases,
            FontFace {
                style,
                font_id: id,
                builtin: false,
            },
        );
    }
    if has_container {
        return;
    }

    let id = system_font_id(path);
    if !seen_paths.insert(id.clone()) {
        return;
    }
    *file_count += 1;
    let (fam_code, display, style, aliases) = classify_stem(stem);
    let mut aliases = aliases;
    aliases.push(display.to_lowercase());
    aliases.push(fam_code.clone());
    catalog.upsert(
        fam_code,
        display,
        aliases,
        FontFace {
            style,
            font_id: id,
            builtin: false,
        },
    );
}
