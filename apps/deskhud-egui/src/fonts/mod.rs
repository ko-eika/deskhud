//! UI 字体：内置（build 嵌入根目录全局资源）+ 系统扫描；同名家族样式互补合并。

mod scan;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use deskhud_ui::font::{FontCatalog, inspect_font_bytes};
use egui::{self, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Theme};

pub use deskhud_ui::font::{FontFace, FontFamilyEntry};
pub use deskhud_ui::font::{classify_stem, normalize_style_name};
pub use scan::system_font_families;

include!(concat!(env!("OUT_DIR"), "/builtin_fonts_gen.rs"));

/// 兼容旧 prefs / 默认家族键（无 `fam.` 前缀）。
pub const BUILTIN_NOTO_SANS_SC: &str = "notosanssc";
/// 默认 Inter 家族键。
pub const BUILTIN_INTER: &str = "inter";

const LEGACY_NOTO: &str = "builtin.noto_sans_sc";
const LEGACY_JB: &str = "builtin.jetbrains_mono";
const LEGACY_JB_FACE: &str = "JetBrainsMono-Regular";
const DEFAULT_FACE_ID: &str = "Inter";

/// 内置 + 系统，同名家族样式互补合并；按显示名排序（不区分来源）。
pub fn list_font_families() -> Vec<FontFamilyEntry> {
    static FAMILIES: std::sync::OnceLock<Vec<FontFamilyEntry>> = std::sync::OnceLock::new();
    FAMILIES.get_or_init(list_font_families_uncached).clone()
}

fn list_font_families_uncached() -> Vec<FontFamilyEntry> {
    let mut catalog = FontCatalog::default();

    for (file, _bytes) in BUILTIN_FONT_FILES {
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        if let Ok(faces) = inspect_font_bytes(_bytes) {
            for face in faces {
                let family_name = face.family.as_deref().unwrap_or(stem);
                let (family_key, fallback_label, fallback_style, aliases) =
                    classify_stem(family_name);
                let style = face.subfamily.unwrap_or(fallback_style);
                let label = face.family.unwrap_or(fallback_label);
                let face_id = if face.face_index == 0 {
                    stem.to_string()
                } else {
                    format!("{stem}#face={}", face.face_index)
                };
                catalog.upsert(
                    family_key,
                    label,
                    aliases,
                    FontFace {
                        style,
                        font_id: face_id,
                        builtin: true,
                    },
                );
            }
        } else {
            let (fam_code, label, style, aliases) = classify_stem(stem);
            catalog.upsert(
                fam_code,
                label,
                aliases,
                FontFace {
                    style,
                    font_id: stem.to_string(),
                    builtin: true,
                },
            );
        }
    }

    for sys in system_font_families() {
        for face in sys.faces {
            catalog.upsert(
                sys.family_key.clone(),
                sys.label.clone(),
                sys.search_terms.clone(),
                face,
            );
        }
    }

    catalog.into_entries()
}

#[cfg(any())]
fn upsert_face(
    map: &mut BTreeMap<String, FontFamilyEntry>,
    family_key: String,
    label: String,
    aliases: Vec<String>,
    face: FontFace,
) {
    let entry = map
        .entry(family_key.clone())
        .or_insert_with(|| FontFamilyEntry {
            family_key: family_key.clone(),
            label: label.clone(),
            search_terms: aliases.clone(),
            faces: Vec::new(),
        });
    if entry.label.is_empty() {
        entry.label = label;
    }
    for a in aliases {
        if !entry.search_terms.iter().any(|t| t == &a) {
            entry.search_terms.push(a);
        }
    }
    let style_n = normalize_style_name(&face.style);
    if let Some(existing) = entry
        .faces
        .iter_mut()
        .find(|f| normalize_style_name(&f.style) == style_n)
    {
        // 互补：同样式优先保留内置
        if face.builtin && !existing.builtin {
            *existing = face;
        }
        return;
    }
    entry.faces.push(face);
}

/// 由家族 + 样式解析可加载 font_id。
pub fn resolve_font_id(families: &[FontFamilyEntry], family_key: &str, style: &str) -> String {
    let key = migrate_family_key(family_key);
    if let Some(fam) = families.iter().find(|f| f.family_key == key) {
        if let Some(face) = fam.face_for(style) {
            return face.font_id.clone();
        }
    }
    // 旧版误把 face id 存进 family 时直接回退
    let as_face = migrate_legacy_font_id(family_key);
    if families
        .iter()
        .any(|f| f.faces.iter().any(|face| face.font_id == as_face))
    {
        return as_face;
    }
    DEFAULT_FACE_ID.into()
}

/// Checks the actual selected face instead of relying on the OS fallback.
pub fn font_id_supports_text(id: &str, text: &str) -> bool {
    let Some((_, data)) = resolve_font_data(id) else {
        return false;
    };
    deskhud_ui::font::face_supports_text(data.font.as_ref(), data.index, text)
}

pub fn family_supports_locale(family: &FontFamilyEntry, english: bool) -> bool {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<HashMap<(String, bool), bool>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let cache_key = (family.family_key.clone(), english);
    if let Some(value) = cache
        .lock()
        .expect("font support cache poisoned")
        .get(&cache_key)
    {
        return *value;
    }
    let sample = if english {
        "Settings Aa 123"
    } else {
        "设置中文界面"
    };
    let supported = family
        .faces
        .iter()
        .any(|face| font_id_supports_text(&face.font_id, sample));
    cache
        .lock()
        .expect("font support cache poisoned")
        .insert(cache_key, supported);
    supported
}

fn migrate_family_key(key: &str) -> String {
    let key = key.strip_prefix("fam.").unwrap_or(key);
    match key {
        "builtin.noto_sans_sc" | "noto_sans_sc" => BUILTIN_NOTO_SANS_SC.into(),
        "builtin.jetbrains_mono" | "jetbrains_mono" => BUILTIN_INTER.into(),
        other => other.to_string(),
    }
}

/// 从已存 font_id 反推家族键。
pub fn family_key_for_font_id(families: &[FontFamilyEntry], font_id: &str) -> String {
    let id = migrate_legacy_font_id(font_id);
    for fam in families {
        if fam.faces.iter().any(|f| f.font_id == id) || fam.family_key == id {
            return fam.family_key.clone();
        }
    }
    // 系统路径：尽量用路径本身在列表里已登记的家族
    if looks_like_font_path(&id) {
        return id;
    }
    BUILTIN_INTER.into()
}

/// 规范化历史 prefs 里的字体 id（去掉 builtin./system. 等前缀）。
pub fn migrate_legacy_font_id(id: &str) -> String {
    match id {
        LEGACY_NOTO | "noto_sans_sc" => "NotoSansSC-Regular".into(),
        LEGACY_JB | LEGACY_JB_FACE | "jetbrains_mono" => DEFAULT_FACE_ID.into(),
        other => {
            if let Some(rest) = other.strip_prefix("builtin.") {
                return rest.to_string();
            }
            if let Some(rest) = other.strip_prefix("system.") {
                return rest.to_string();
            }
            other.to_string()
        }
    }
}

/// 家族显示名。
pub fn label_for_family(families: &[FontFamilyEntry], family_key: &str) -> String {
    let key = migrate_family_key(family_key);
    families
        .iter()
        .find(|f| f.family_key == key)
        .map(|f| f.label.clone())
        .unwrap_or_else(|| family_key.to_string())
}

/// 可选字号。
pub const FONT_SIZE_OPTIONS: &[f32] = &[11.0, 12.0, 13.0, 14.0, 15.0, 16.0, 18.0];

/// 配置字体数据 + 全局字号（双主题）。
pub fn configure_typography(ctx: &egui::Context, ui_font_id: &str, ui_font_size: f32) {
    configure_fonts(ctx, ui_font_id);
    apply_text_size(ctx, ui_font_size);
    // Most of the settings surface uses explicit FontIds for compact cards and
    // previews. Scale the egui coordinate system as well so those controls
    // follow the user-facing font-size preference instead of only text styles.
    let base_ppp = ctx
        .data(|data| data.get_temp::<f32>(egui::Id::new("deskhud.base_ppp")))
        .unwrap_or_else(|| {
            let current = ctx.pixels_per_point();
            ctx.data_mut(|data| {
                data.insert_temp(egui::Id::new("deskhud.base_ppp"), current);
            });
            current
        });
    ctx.set_pixels_per_point(base_ppp * (ui_font_size.clamp(10.0, 22.0) / 13.0));
}

/// 按 prefs 中的字体 ID 配置 egui。
pub fn configure_fonts(ctx: &egui::Context, ui_font_id: &str) {
    let id = migrate_legacy_font_id(if ui_font_id.trim().is_empty() {
        DEFAULT_FACE_ID
    } else {
        ui_font_id
    });

    let mut fonts = FontDefinitions::default();

    // 注册全部内置面（切换样式无需再读盘）
    for (file, bytes) in BUILTIN_FONT_FILES {
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        let key = format!("builtin_data_{stem}");
        fonts
            .font_data
            .insert(key, Arc::new(FontData::from_static(bytes)));
    }

    let primary = match resolve_font_data(&id) {
        Some((name, data)) => {
            if !fonts.font_data.contains_key(&name) {
                fonts.font_data.insert(name.clone(), data);
            }
            name
        }
        None => {
            tracing::warn!(%id, "ui font missing; fallback to Inter");
            "builtin_data_Inter".into()
        }
    };

    let selected_style = list_font_families()
        .iter()
        .flat_map(|family| family.faces.iter())
        .find(|face| face.font_id == id)
        .map(|face| face.style.to_ascii_lowercase())
        .unwrap_or_default();
    let wants_bold_cjk = ["bold", "semibold", "extrabold", "black"]
        .iter()
        .any(|weight| selected_style.contains(weight));

    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.clear();
    prop.push(primary.clone());
    let noto_key = "builtin_data_NotoSansSC-Regular";
    let mut cjk_fallbacks = scan::priority_system_cjk();
    cjk_fallbacks.sort_by_key(|(name, _)| {
        let is_bold = name.to_ascii_lowercase().contains("bd");
        if wants_bold_cjk == is_bold { 0 } else { 1 }
    });
    for (name, path) in cjk_fallbacks {
        if fonts.font_data.contains_key(&name) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&path) {
            fonts
                .font_data
                .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
            prop.push(name);
        }
        // Keep the bundled font as the final safety net. System CJK faces are
        // preferred so a bold selection can use a bold CJK face when available.
        if primary != noto_key && fonts.font_data.contains_key(noto_key) {
            prop.push(noto_key.into());
        }
    }

    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.clear();
    if fonts.font_data.contains_key(noto_key) {
        mono.push(noto_key.into());
    }
    if primary != noto_key {
        mono.push(primary);
    }

    ctx.set_fonts(fonts);
}

fn apply_text_size(ctx: &egui::Context, size: f32) {
    let size = size.clamp(10.0, 22.0);
    for theme in [Theme::Light, Theme::Dark] {
        let mut style = (*ctx.style_of(theme)).clone();
        style.text_styles.insert(
            TextStyle::Small,
            FontId::proportional((size * 0.85).round()),
        );
        style
            .text_styles
            .insert(TextStyle::Body, FontId::proportional(size));
        style
            .text_styles
            .insert(TextStyle::Button, FontId::proportional(size));
        style.text_styles.insert(
            TextStyle::Heading,
            FontId::proportional((size * 1.35).round()),
        );
        style
            .text_styles
            .insert(TextStyle::Monospace, FontId::monospace(size));
        style.visuals.panel_fill = egui::Color32::TRANSPARENT;
        style.visuals.window_fill = egui::Color32::TRANSPARENT;
        style.visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
        style.visuals.popup_shadow = egui::Shadow::NONE;
        style.visuals.window_stroke = egui::Stroke::NONE;
        ctx.set_style_of(theme, style);
    }
}

fn resolve_font_data(id: &str) -> Option<(String, Arc<FontData>)> {
    let id = migrate_legacy_font_id(id);
    // 1) 内置：按文件 stem 匹配
    let (source_id, face_index) = split_face_id(&id);
    for (file, bytes) in BUILTIN_FONT_FILES {
        let fstem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        if fstem == source_id || *file == source_id {
            let key = if face_index == 0 {
                format!("builtin_data_{fstem}")
            } else {
                format!("builtin_data_{fstem}_face_{face_index}")
            };
            let mut data = FontData::from_static(bytes);
            data.index = face_index;
            return Some((key, Arc::new(data)));
        }
    }
    // 2) 系统路径（或其它磁盘字体）
    if looks_like_font_path(&id) {
        let (source, index) = split_face_id(&id);
        let path = PathBuf::from(source.replace('/', std::path::MAIN_SEPARATOR_STR));
        let bytes = std::fs::read(&path).ok()?;
        let name = if index == 0 {
            font_key_name(&path)
        } else {
            format!("{}_face_{index}", font_key_name(&path))
        };
        let mut data = FontData::from_owned(bytes);
        data.index = index;
        return Some((name, Arc::new(data)));
    }
    None
}

fn split_face_id(id: &str) -> (String, u32) {
    let Some((source, suffix)) = id.rsplit_once("#face=") else {
        return (id.to_string(), 0);
    };
    (source.to_string(), suffix.parse().unwrap_or(0))
}

fn looks_like_font_path(id: &str) -> bool {
    id.contains('/')
        || id.contains('\\')
        || id.ends_with(".ttf")
        || id.ends_with(".otf")
        || id.ends_with(".ttc")
        || id.ends_with(".TTF")
        || id.ends_with(".OTF")
        || id.ends_with(".TTC")
}

pub(crate) fn system_font_id(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn font_key_name(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("system_font")
        .to_string()
}

/// 样式展示名（含 Italic 组合，如 Light Italic → 细斜体）。
pub fn style_label_zh(style: &str) -> String {
    let s = normalize_style_name(style);
    let lower = s.to_ascii_lowercase();
    let known = match lower.as_str() {
        "regular" => Some("常规"),
        "italic" => Some("斜体"),
        "bold" => Some("粗体"),
        "negreta" | "negrita" | "gras" | "fett" | "grassetto" => Some("粗体"),
        "bold italic" => Some("粗体斜体"),
        "light" => Some("细体"),
        "light italic" => Some("细体斜体"),
        "thin" => Some("纤细"),
        "thin italic" => Some("纤细斜体"),
        "medium" => Some("中等"),
        "medium italic" => Some("中等斜体"),
        "demilight" => Some("微细"),
        "demilight italic" => Some("微细斜体"),
        "demibold" => Some("半粗"),
        "demibold italic" => Some("半粗斜体"),
        "semibold" => Some("半粗"),
        "semibold italic" => Some("半粗斜体"),
        "extrabold" => Some("特粗"),
        "extrabold italic" => Some("特粗斜体"),
        "extralight" => Some("特细"),
        "extralight italic" => Some("特细斜体"),
        "black" => Some("特黑"),
        "black italic" => Some("特黑斜体"),
        "normal" => Some("正常"),
        "narrow" => Some("窄体"),
        "narrow bold" => Some("窄体粗体"),
        "narrow italic" => Some("窄体斜体"),
        "narrow bold italic" => Some("窄体粗体斜体"),
        "condensed" => Some("窄体"),
        "condensed italic" => Some("窄体斜体"),
        "condensed bold" => Some("窄体粗体"),
        "condensed bold italic" => Some("窄体粗体斜体"),
        "expanded" => Some("宽体"),
        "expanded italic" => Some("宽体斜体"),
        _ => None,
    };
    if let Some(label) = known {
        return label.into();
    }
    let italic = lower.contains("italic");
    let base = lower
        .replace(" italic", "")
        .replace("italic", "")
        .replace(' ', "");
    let base_zh = match base.as_str() {
        "" | "regular" => "常规",
        "bold" => "粗体",
        "light" => "细体",
        "demilight" => "微细",
        "thin" => "纤细",
        "medium" | "book" => "中等",
        "semibold" | "demibold" => "半粗",
        "extrabold" => "特粗",
        "black" | "heavy" => "特黑",
        "extralight" => "特细",
        _ => return s,
    };
    if !italic {
        return base_zh.into();
    }
    if matches!(base.as_str(), "" | "regular") {
        return "斜体".into();
    }
    if let Some(stem) = base_zh.strip_suffix('体') {
        format!("{stem}斜体")
    } else {
        format!("{base_zh}斜体")
    }
}

pub fn style_label_en(style: &str) -> String {
    normalize_style_name(style)
}
