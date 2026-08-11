//! UI 字体：内置（build 嵌入 assets/fonts）+ 系统扫描；同名家族样式互补合并。

mod classify;
mod scan;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use eframe::egui::{self, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Theme};

pub use classify::{normalize_style_name, style_sort_key};
pub use scan::system_font_families;

include!(concat!(env!("OUT_DIR"), "/builtin_fonts_gen.rs"));

/// 兼容旧 prefs / 默认家族键（无 `fam.` 前缀）。
pub const BUILTIN_NOTO_SANS_SC: &str = "notosanssc";
/// 默认 JetBrains Mono 家族键。
pub const BUILTIN_JETBRAINS_MONO: &str = "jetbrainsmono";

const LEGACY_NOTO: &str = "builtin.noto_sans_sc";
const LEGACY_JB: &str = "builtin.jetbrains_mono";
const DEFAULT_FACE_ID: &str = "JetBrainsMono-Regular";

/// 字重面。
#[derive(Debug, Clone)]
pub struct FontFace {
    /// 样式名：`Regular` / `Bold` / `Bold Italic` …
    pub style: String,
    /// 可加载 ID：内置为文件 stem；系统为字体文件路径（`/` 分隔）。
    pub font_id: String,
    /// 来自内置包（合并时同样式优先内置）。
    pub builtin: bool,
}

/// 字体系列（可含多种样式）。
#[derive(Debug, Clone)]
pub struct FontFamilyEntry {
    /// 稳定家族键（规范化小写码，无前缀）。
    pub family_key: String,
    /// 显示名（无来源后缀；合并后统一）。
    pub label: String,
    /// 搜索别名。
    pub search_terms: Vec<String>,
    pub faces: Vec<FontFace>,
}

impl FontFamilyEntry {
    pub fn face_for(&self, style: &str) -> Option<&FontFace> {
        let want = normalize_style_name(style);
        self.faces
            .iter()
            .find(|f| normalize_style_name(&f.style) == want)
            .or_else(|| {
                self.faces
                    .iter()
                    .find(|f| normalize_style_name(&f.style) == "Regular")
            })
            .or_else(|| self.faces.first())
    }

    pub fn style_names(&self) -> Vec<String> {
        let mut styles: Vec<String> = self.faces.iter().map(|f| f.style.clone()).collect();
        styles.sort_by(|a, b| style_sort_key(a).cmp(&style_sort_key(b)));
        styles.dedup();
        styles
    }
}

/// 内置 + 系统，同名家族样式互补合并；按显示名排序（不区分来源）。
pub fn list_font_families() -> Vec<FontFamilyEntry> {
    let mut by_key: BTreeMap<String, FontFamilyEntry> = BTreeMap::new();

    for (file, _bytes) in BUILTIN_FONT_FILES {
        let stem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        let (fam_code, label, style, aliases) = classify::classify_stem(stem);
        let family_key = fam_code;
        let font_id = stem.to_string();
        upsert_face(
            &mut by_key,
            family_key,
            label,
            aliases,
            FontFace {
                style,
                font_id,
                builtin: true,
            },
        );
    }

    for sys in system_font_families() {
        for face in sys.faces {
            upsert_face(
                &mut by_key,
                sys.family_key.clone(),
                sys.label.clone(),
                sys.search_terms.clone(),
                face,
            );
        }
    }

    let mut out: Vec<FontFamilyEntry> = by_key.into_values().collect();
    for fam in &mut out {
        fam.faces
            .sort_by(|a, b| style_sort_key(&a.style).cmp(&style_sort_key(&b.style)));
    }
    out.sort_by(|a, b| a.label.to_lowercase().cmp(&b.label.to_lowercase()));
    out
}

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

fn migrate_family_key(key: &str) -> String {
    let key = key.strip_prefix("fam.").unwrap_or(key);
    match key {
        "builtin.noto_sans_sc" | "noto_sans_sc" => BUILTIN_NOTO_SANS_SC.into(),
        "builtin.jetbrains_mono" | "jetbrains_mono" => BUILTIN_JETBRAINS_MONO.into(),
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
    BUILTIN_JETBRAINS_MONO.into()
}

/// 规范化历史 prefs 里的字体 id（去掉 builtin./system. 等前缀）。
pub fn migrate_legacy_font_id(id: &str) -> String {
    match id {
        LEGACY_NOTO | "noto_sans_sc" => "NotoSansSC-Regular".into(),
        LEGACY_JB | "jetbrains_mono" => DEFAULT_FACE_ID.into(),
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
            tracing::warn!(%id, "ui font missing; fallback to JetBrains Mono");
            "builtin_data_JetBrainsMono-Regular".into()
        }
    };

    let prop = fonts.families.entry(FontFamily::Proportional).or_default();
    prop.clear();
    prop.push(primary.clone());
    let noto_key = "builtin_data_NotoSansSC-Regular";
    if primary != noto_key && fonts.font_data.contains_key(noto_key) {
        prop.push(noto_key.into());
    }
    for (name, path) in scan::priority_system_cjk() {
        if fonts.font_data.contains_key(&name) {
            continue;
        }
        if let Ok(bytes) = std::fs::read(&path) {
            fonts
                .font_data
                .insert(name.clone(), Arc::new(FontData::from_owned(bytes)));
            prop.push(name);
        }
    }

    let mono = fonts.families.entry(FontFamily::Monospace).or_default();
    mono.clear();
    let jb = "builtin_data_JetBrainsMono-Regular";
    if fonts.font_data.contains_key(jb) {
        mono.push(jb.into());
    }
    if fonts.font_data.contains_key(noto_key) {
        mono.push(noto_key.into());
    }
    if primary != jb && primary != noto_key {
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
    for (file, bytes) in BUILTIN_FONT_FILES {
        let fstem = Path::new(file)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or(file);
        if fstem == id || *file == id {
            let key = format!("builtin_data_{fstem}");
            return Some((key, Arc::new(FontData::from_static(bytes))));
        }
    }
    // 2) 系统路径（或其它磁盘字体）
    if looks_like_font_path(&id) {
        let path = PathBuf::from(id.replace('/', std::path::MAIN_SEPARATOR_STR));
        let bytes = std::fs::read(&path).ok()?;
        let name = font_key_name(&path);
        return Some((name, Arc::new(FontData::from_owned(bytes))));
    }
    None
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
