//! 偏好持久化（有序 TOML → 用户数据目录；兼容旧 `[shell]` / `[*.config]`）。

use std::fs;
use std::path::PathBuf;

use thiserror::Error;

use crate::UiPreferences;
use crate::hud::{HudConfigValue, HudPrefs, HudSlotLayout};
use crate::i18n::Locale;
use crate::pet::PetPrefs;
use crate::shell::{PetPickerMode, ShellPrefs, UiTheme};

/// 持久化错误。
#[derive(Debug, Error)]
pub enum PersistError {
    /// IO。
    #[error("prefs io: {0}")]
    Io(#[from] std::io::Error),
    /// TOML 解析。
    #[error("prefs parse: {0}")]
    Parse(#[from] toml::de::Error),
    /// TOML 序列化。
    #[error("prefs serialize: {0}")]
    Serialize(#[from] toml::ser::Error),
}

/// 用户数据根：`%APPDATA%/DeskHud` 或 `~/.local/share/DeskHud`。
pub fn user_data_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(appdata).join("DeskHud"))
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("DeskHud"),
        )
    }
}

/// 偏好文件路径。
pub fn prefs_path() -> Option<PathBuf> {
    Some(user_data_dir()?.join("config.toml"))
}

/// 从磁盘加载；文件不存在或损坏时返回 `Default`。
pub fn load_or_default() -> UiPreferences {
    match load() {
        Ok(p) => p,
        Err(_) => UiPreferences::default(),
    }
}

/// 从磁盘加载（含旧格式迁移）。
pub fn load() -> Result<UiPreferences, PersistError> {
    let path = prefs_path().ok_or_else(|| {
        PersistError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "user data dir unavailable",
        ))
    })?;
    if !path.exists() {
        return Ok(UiPreferences::default());
    }
    let text = fs::read_to_string(&path)?;
    let value: toml::Value = toml::from_str(&text)?;
    let mut prefs = prefs_from_value(value);
    prefs.normalize_ids();
    Ok(prefs)
}

/// 写入磁盘（分类有序 TOML）。
pub fn save(prefs: &UiPreferences) -> Result<(), PersistError> {
    save_ordered(prefs, &PrefsWriteOrder::default())
}

/// 写入磁盘，并按引擎注册顺序排列宠/插件配置键。
pub fn save_ordered(prefs: &UiPreferences, order: &PrefsWriteOrder) -> Result<(), PersistError> {
    let dir = user_data_dir().ok_or_else(|| {
        PersistError::Io(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "user data dir unavailable",
        ))
    })?;
    fs::create_dir_all(&dir)?;
    let path = dir.join("config.toml");
    let text = format_prefs_ordered(prefs, order);
    let tmp = dir.join("config.toml.tmp");
    fs::write(&tmp, text)?;
    fs::rename(&tmp, &path)?;
    Ok(())
}

/// 写出时的宠 / 插件配置顺序（来自引擎注册表；空则字典序兜底）。
#[derive(Debug, Clone, Default)]
pub struct PrefsWriteOrder {
    /// 宠物 ID 注册顺序。
    pub pet_ids: Vec<String>,
    /// `(pet_id, 选项短键定义序)`。
    pub pet_option_keys: Vec<(String, Vec<String>)>,
    /// 插件 ID 注册顺序。
    pub plugin_ids: Vec<String>,
    /// `(plugin_id, 贡献短键定义序)`。
    pub plugin_contrib_ids: Vec<(String, Vec<String>)>,
}

/// 生成分类、键序稳定的 config.toml 文本。
pub fn format_prefs(prefs: &UiPreferences) -> String {
    format_prefs_ordered(prefs, &PrefsWriteOrder::default())
}

/// 带注册顺序的 prefs 文本。
pub fn format_prefs_ordered(prefs: &UiPreferences, order: &PrefsWriteOrder) -> String {
    let mut out = String::new();
    out.push_str("# DeskHud configuration\n");

    out.push_str("\n[prefs]\n");
    if let (Some(width), Some(height)) = (prefs.shell.settings_width, prefs.shell.settings_height) {
        out.push_str(&format!("\"settings.size\" = [{width}, {height}]\n"));
    }
    if let (Some(x), Some(y)) = (prefs.shell.settings_pos_x, prefs.shell.settings_pos_y) {
        out.push_str(&format!("\"settings.position\" = [{x}, {y}]\n"));
    }

    out.push_str("\n[general]\n");
    out.push_str(&format!("topmost = {}\n", prefs.shell.topmost));

    out.push_str("\n[graphics]\n");
    out.push_str(&format!(
        "fps_limit = \"{}\"\n",
        match prefs.graphics.fps_limit {
            crate::FpsLimit::Auto => "auto",
            crate::FpsLimit::Fps30 => "30",
            crate::FpsLimit::Fps60 => "60",
            crate::FpsLimit::Fps120 => "120",
        }
    ));
    out.push_str(&format!(
        "animation_quality = \"{}\"\n",
        match prefs.graphics.animation_quality {
            crate::AnimationQuality::Low => "low",
            crate::AnimationQuality::Standard => "standard",
            crate::AnimationQuality::High => "high",
        }
    ));
    out.push_str(&format!("effects = {}\n", prefs.graphics.effects));
    out.push_str(&format!(
        "power_mode = \"{}\"\n",
        match prefs.graphics.power_mode {
            crate::PowerMode::Saving => "saving",
            crate::PowerMode::Balanced => "balanced",
            crate::PowerMode::Smooth => "smooth",
        }
    ));

    out.push_str("\n[theme]\n");
    out.push_str(&format!("mode = \"{}\"\n", theme_tag(prefs.shell.ui_theme)));
    out.push_str(&format!("locale = \"{}\"\n", locale_tag(prefs.locale)));

    out.push_str("\n[font]\n");
    out.push_str(&format!("id = \"{}\"\n", escape(&prefs.shell.ui_font_id)));
    out.push_str(&format!(
        "family = \"{}\"\n",
        escape(&prefs.shell.ui_font_family)
    ));
    out.push_str(&format!(
        "style = \"{}\"\n",
        escape(&prefs.shell.ui_font_style)
    ));
    out.push_str(&format!("size = {}\n", prefs.shell.ui_font_size));

    out.push_str("\n[pet]\n");
    // global 固定顺序靠前
    out.push_str(&format!(
        "\"{}\" = \"{}\"\n",
        PetPrefs::GLOBAL_KIND_KEY,
        escape(&prefs.pet.kind)
    ));
    out.push_str(&format!(
        "\"pet.global.size\" = [{}, {}]\n",
        prefs.pet.width, prefs.pet.height
    ));
    if let Some(pos) = prefs.pet.position() {
        out.push_str(&format!(
            "\"pet.global.position\" = [{}, {}]\n",
            pos.x, pos.y
        ));
    }
    out.push_str(&format!(
        "\"{}\" = \"{}\"\n",
        PetPrefs::GLOBAL_PICKER_MODE_KEY,
        picker_tag(prefs.pet.picker_mode)
    ));
    for (k, v) in ordered_pet_options(&prefs.pet.options, order) {
        out.push_str(&format!("\"{}\" = {}\n", escape(k), v));
    }

    out.push_str("\n[hud]\n");
    for (k, v) in ordered_hud_entries(&prefs.hud.config, order) {
        out.push_str(&format!("\"{}\" = {}\n", escape(k), format_hud_value(v)));
    }

    out
}

fn prefs_from_value(root: toml::Value) -> UiPreferences {
    let mut prefs = UiPreferences::default();
    let Some(table) = root.as_table() else {
        return prefs;
    };

    // 根级 locale（旧）或 [theme].locale
    if let Some(v) = table.get("locale").and_then(|v| v.as_str()) {
        prefs.locale = parse_locale(v);
    }
    if let Some(general) = table.get("general").and_then(|v| v.as_table()) {
        if let Some(topmost) = general.get("topmost").and_then(|v| v.as_bool()) {
            prefs.shell.topmost = topmost;
        }
    }
    if let Some(g) = table.get("graphics").and_then(|v| v.as_table()) {
        if let Some(v) = g.get("fps_limit").and_then(|v| v.as_str()) {
            prefs.graphics.fps_limit = match v {
                "auto" => crate::FpsLimit::Auto,
                "30" => crate::FpsLimit::Fps30,
                "120" => crate::FpsLimit::Fps120,
                _ => crate::FpsLimit::Auto,
            };
        }
        if let Some(v) = g.get("animation_quality").and_then(|v| v.as_str()) {
            prefs.graphics.animation_quality = match v {
                "low" => crate::AnimationQuality::Low,
                "high" => crate::AnimationQuality::High,
                _ => crate::AnimationQuality::Standard,
            };
        }
        if let Some(v) = g.get("effects").and_then(|v| v.as_bool()) {
            prefs.graphics.effects = v;
        }
        if let Some(v) = g.get("power_mode").and_then(|v| v.as_str()) {
            prefs.graphics.power_mode = match v {
                "saving" => crate::PowerMode::Saving,
                "smooth" => crate::PowerMode::Smooth,
                _ => crate::PowerMode::Balanced,
            };
        }
    }

    // [theme] / [prefs]
    if let Some(theme) = table.get("theme").and_then(|v| v.as_table()) {
        merge_theme_table(&mut prefs.shell, theme);
        if let Some(v) = theme.get("locale").and_then(|v| v.as_str()) {
            prefs.locale = parse_locale(v);
        }
    }
    if let Some(settings) = table.get("prefs").and_then(|v| v.as_table()) {
        if let Some(pair) = settings.get("settings.size").and_then(toml_pair) {
            prefs.shell.settings_width = Some(pair[0]);
            prefs.shell.settings_height = Some(pair[1]);
        }
        if let Some(pair) = settings.get("settings.position").and_then(toml_pair) {
            prefs.shell.settings_pos_x = Some(pair[0]);
            prefs.shell.settings_pos_y = Some(pair[1]);
        }
    }
    let settings_has_topmost = true;
    if let Some(ui) = table.get("ui").and_then(|v| v.as_table()) {
        merge_ui_table(&mut prefs.shell, ui);
        // 旧 shell 误写在 ui 里的宠字段 → pet
        merge_legacy_pet_fields(&mut prefs.pet, ui);
        if !settings_has_topmost {
            if let Some(tm) = legacy_topmost_from_pet_table(ui) {
                prefs.shell.topmost = tm;
            }
        }
        // 旧版字体仍可能写在 [ui]
        merge_font_table(&mut prefs.shell, ui);
    }
    if let Some(font) = table.get("font").and_then(|v| v.as_table()) {
        merge_font_table(&mut prefs.shell, font);
    }
    if let Some(shell) = table.get("shell").and_then(|v| v.as_table()) {
        merge_ui_table(&mut prefs.shell, shell);
        merge_legacy_pet_fields(&mut prefs.pet, shell);
        if !settings_has_topmost {
            if let Some(tm) = legacy_topmost_from_pet_table(shell) {
                prefs.shell.topmost = tm;
            }
        }
        merge_font_table(&mut prefs.shell, shell);
        // 旧字体键名
        if let Some(v) = shell.get("ui_font_id").and_then(|v| v.as_str()) {
            prefs.shell.ui_font_id = v.to_string();
        }
        if let Some(v) = shell.get("ui_font_family").and_then(|v| v.as_str()) {
            prefs.shell.ui_font_family = v.to_string();
        }
        if let Some(v) = shell.get("ui_font_style").and_then(|v| v.as_str()) {
            prefs.shell.ui_font_style = v.to_string();
        }
        if let Some(v) = shell
            .get("ui_font_size")
            .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        {
            prefs.shell.ui_font_size = v as f32;
        }
        if let Some(v) = shell.get("ui_theme").and_then(|v| v.as_str()) {
            prefs.shell.ui_theme = parse_theme(v);
        }
    }

    // [pet] 或旧 [pet.config]
    if let Some(pet) = table.get("pet").and_then(|v| v.as_table()) {
        // 若含嵌套 config，先合并外层再合并 config
        merge_pet_table(&mut prefs.pet, pet);
        if !settings_has_topmost {
            if let Some(tm) = legacy_topmost_from_pet_table(pet) {
                prefs.shell.topmost = tm;
            }
        }
        if let Some(cfg) = pet.get("config").and_then(|v| v.as_table()) {
            merge_pet_options(&mut prefs.pet, cfg);
        }
    }
    if let Some(cfg) = table
        .get("pet")
        .and_then(|v| v.get("config"))
        .and_then(|v| v.as_table())
    {
        merge_pet_options(&mut prefs.pet, cfg);
    }

    // [hud] 或旧 [hud.config] / layout
    if let Some(hud) = table.get("hud").and_then(|v| v.as_table()) {
        if let Some(cfg) = hud.get("config").and_then(|v| v.as_table()) {
            merge_hud_table(&mut prefs.hud, cfg);
        } else {
            merge_hud_table(&mut prefs.hud, hud);
        }
        if let Some(layout) = hud.get("layout").and_then(|v| v.as_table()) {
            merge_hud_layout_table(&mut prefs.hud, layout);
        }
    }

    prefs
}

fn merge_theme_table(ui: &mut ShellPrefs, t: &toml::map::Map<String, toml::Value>) {
    if let Some(v) = t
        .get("mode")
        .or_else(|| t.get("theme"))
        .or_else(|| t.get("ui_theme"))
        .and_then(|v| v.as_str())
    {
        ui.ui_theme = parse_theme(v);
    }
}

fn merge_settings_table(ui: &mut ShellPrefs, t: &toml::map::Map<String, toml::Value>) {
    ui.settings_width = t
        .get("width")
        .or_else(|| t.get("settings_width"))
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_width);
    ui.settings_height = t
        .get("height")
        .or_else(|| t.get("settings_height"))
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_height);
    ui.settings_pos_x = t
        .get("pos_x")
        .or_else(|| t.get("settings_pos_x"))
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_pos_x);
    ui.settings_pos_y = t
        .get("pos_y")
        .or_else(|| t.get("settings_pos_y"))
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_pos_y);
    if let Some(v) = t.get("topmost").and_then(|v| v.as_bool()) {
        ui.topmost = v;
    }
}

fn merge_ui_table(ui: &mut ShellPrefs, t: &toml::map::Map<String, toml::Value>) {
    merge_theme_table(ui, t);
    merge_settings_table(ui, t);
    // 旧 [ui] 里带 settings_ 前缀的键
    ui.settings_width = t
        .get("settings_width")
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_width);
    ui.settings_height = t
        .get("settings_height")
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_height);
    ui.settings_pos_x = t
        .get("settings_pos_x")
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_pos_x);
    ui.settings_pos_y = t
        .get("settings_pos_y")
        .and_then(toml_f64)
        .map(|v| v as f32)
        .or(ui.settings_pos_y);
}

/// `[font]` 或旧 `[ui]`/`[shell]` 内的字体键。
fn merge_font_table(ui: &mut ShellPrefs, t: &toml::map::Map<String, toml::Value>) {
    if let Some(v) = t
        .get("id")
        .or_else(|| t.get("font_id"))
        .or_else(|| t.get("ui_font_id"))
        .and_then(|v| v.as_str())
    {
        ui.ui_font_id = v.to_string();
    }
    if let Some(v) = t
        .get("family")
        .or_else(|| t.get("font_family"))
        .or_else(|| t.get("ui_font_family"))
        .and_then(|v| v.as_str())
    {
        ui.ui_font_family = v.to_string();
    }
    if let Some(v) = t
        .get("style")
        .or_else(|| t.get("font_style"))
        .or_else(|| t.get("ui_font_style"))
        .and_then(|v| v.as_str())
    {
        ui.ui_font_style = v.to_string();
    }
    if let Some(v) = t
        .get("size")
        .or_else(|| t.get("font_size"))
        .or_else(|| t.get("ui_font_size"))
        .and_then(toml_f64)
    {
        ui.ui_font_size = v as f32;
    }
}

fn merge_legacy_pet_fields(pet: &mut PetPrefs, t: &toml::map::Map<String, toml::Value>) {
    if let Some(v) = t
        .get(PetPrefs::GLOBAL_KIND_KEY)
        .or_else(|| t.get("kind"))
        .or_else(|| t.get("active_pet_kind_id"))
        .and_then(|v| v.as_str())
    {
        pet.kind = v.to_string();
    }
    if let Some(v) = t
        .get(PetPrefs::GLOBAL_WIDTH_KEY)
        .or_else(|| t.get("width"))
        .or_else(|| t.get("pet_width"))
        .and_then(toml_f64)
    {
        pet.width = v as f32;
    }
    if let Some(v) = t
        .get(PetPrefs::GLOBAL_HEIGHT_KEY)
        .or_else(|| t.get("height"))
        .or_else(|| t.get("pet_height"))
        .and_then(toml_f64)
    {
        pet.height = v as f32;
    }
    if let Some([w, h]) = t
        .get("pet.global.size")
        .or_else(|| t.get("size"))
        .and_then(toml_pair)
    {
        pet.width = w;
        pet.height = h;
    }
    if let Some(v) = t
        .get(PetPrefs::GLOBAL_POS_X_KEY)
        .or_else(|| t.get("pos_x"))
        .or_else(|| t.get("pet_pos_x"))
        .and_then(toml_f64)
    {
        pet.pos_x = Some(v as f32);
    }
    if let Some(v) = t
        .get(PetPrefs::GLOBAL_POS_Y_KEY)
        .or_else(|| t.get("pos_y"))
        .or_else(|| t.get("pet_pos_y"))
        .and_then(toml_f64)
    {
        pet.pos_y = Some(v as f32);
    }
    if let Some([x, y]) = t
        .get("pet.global.position")
        .or_else(|| t.get("position"))
        .and_then(toml_pair)
    {
        pet.pos_x = Some(x);
        pet.pos_y = Some(y);
    }
    // topmost 已迁到 [settings]；此处仅兼容旧键，由调用方写入 shell
    if let Some(v) = t
        .get(PetPrefs::GLOBAL_PICKER_MODE_KEY)
        .or_else(|| t.get("picker_mode"))
        .or_else(|| t.get("pet_picker_mode"))
        .and_then(|v| v.as_str())
    {
        pet.picker_mode = parse_picker(v);
    }
}

fn toml_pair(value: &toml::Value) -> Option<[f32; 2]> {
    let values = value.as_array()?;
    Some([
        values.first().and_then(toml_f64)? as f32,
        values.get(1).and_then(toml_f64)? as f32,
    ])
}

fn legacy_topmost_from_pet_table(t: &toml::map::Map<String, toml::Value>) -> Option<bool> {
    t.get(PetPrefs::LEGACY_GLOBAL_TOPMOST_KEY)
        .or_else(|| t.get("topmost"))
        .or_else(|| t.get("pet_topmost"))
        .and_then(|v| v.as_bool())
}

fn merge_pet_table(pet: &mut PetPrefs, t: &toml::map::Map<String, toml::Value>) {
    merge_legacy_pet_fields(pet, t);
    merge_pet_options(pet, t);
}

fn is_reserved_pet_key(k: &str) -> bool {
    matches!(
        k,
        "kind"
            | "active_pet_kind_id"
            | "kind_id"
            | "width"
            | "height"
            | "pet_width"
            | "pet_height"
            | "pos_x"
            | "pos_y"
            | "pet_pos_x"
            | "pet_pos_y"
            | "topmost"
            | "pet_topmost"
            | "picker_mode"
            | "pet_picker_mode"
            | "config"
            | "options"
    ) || k == PetPrefs::GLOBAL_KIND_KEY
        || k == PetPrefs::GLOBAL_WIDTH_KEY
        || k == PetPrefs::GLOBAL_HEIGHT_KEY
        || k == PetPrefs::GLOBAL_POS_X_KEY
        || k == PetPrefs::GLOBAL_POS_Y_KEY
        || k == PetPrefs::LEGACY_GLOBAL_TOPMOST_KEY
        || k == PetPrefs::GLOBAL_PICKER_MODE_KEY
}

fn merge_pet_options(pet: &mut PetPrefs, t: &toml::map::Map<String, toml::Value>) {
    for (k, v) in t {
        if is_reserved_pet_key(k) {
            continue;
        }
        if let Some(b) = v.as_bool() {
            pet.options.insert(k.clone(), b);
        }
    }
}

fn merge_hud_table(hud: &mut HudPrefs, t: &toml::map::Map<String, toml::Value>) {
    const SKIP: &[&str] = &["config", "layout", "enabled", "plugin_enabled"];
    for (k, v) in t {
        if SKIP.contains(&k.as_str()) {
            continue;
        }
        if let Some(val) = toml_to_hud_value(v) {
            hud.config.insert(k.clone(), val);
        }
    }
}

fn merge_hud_layout_table(hud: &mut HudPrefs, t: &toml::map::Map<String, toml::Value>) {
    for (key, v) in t {
        let Some(slot_t) = v.as_table() else {
            continue;
        };
        let mut slot = HudSlotLayout::default();
        if let Some(d) = slot_t.get("display").and_then(|v| v.as_str()) {
            slot.display = d.to_string();
        }
        if let Some(x) = slot_t.get("x").and_then(toml_f64) {
            slot.x = x as f32;
        }
        if let Some(y) = slot_t.get("y").and_then(toml_f64) {
            slot.y = y as f32;
        }
        if let Some(s) = slot_t.get("scale").and_then(toml_f64) {
            slot.scale = s as f32;
        }
        if let Some((plugin, contrib)) = key.rsplit_once('.') {
            hud.set_slot_layout(plugin, contrib, slot);
        }
    }
}

fn toml_to_hud_value(v: &toml::Value) -> Option<HudConfigValue> {
    if let Some(b) = v.as_bool() {
        return Some(HudConfigValue::Bool(b));
    }
    if let Some(s) = v.as_str() {
        return Some(HudConfigValue::String(s.to_string()));
    }
    if let Some(i) = v.as_integer() {
        return Some(HudConfigValue::Int(i));
    }
    if let Some(f) = v.as_float() {
        return Some(HudConfigValue::Float(f));
    }
    None
}

fn toml_f64(v: &toml::Value) -> Option<f64> {
    v.as_float().or_else(|| v.as_integer().map(|i| i as f64))
}

fn locale_tag(locale: Locale) -> &'static str {
    match locale {
        Locale::ZhCn => "zh-cn",
        Locale::En => "en",
    }
}

fn parse_locale(s: &str) -> Locale {
    match s {
        "en" | "en-US" | "en_US" => Locale::En,
        _ => Locale::ZhCn,
    }
}

fn theme_tag(theme: UiTheme) -> &'static str {
    match theme {
        UiTheme::System => "system",
        UiTheme::Light => "light",
        UiTheme::Dark => "dark",
    }
}

fn parse_theme(s: &str) -> UiTheme {
    match s {
        "light" => UiTheme::Light,
        "dark" => UiTheme::Dark,
        _ => UiTheme::System,
    }
}

fn picker_tag(mode: PetPickerMode) -> &'static str {
    match mode {
        PetPickerMode::Grid => "grid",
        PetPickerMode::List => "list",
    }
}

fn parse_picker(s: &str) -> PetPickerMode {
    match s {
        "list" => PetPickerMode::List,
        _ => PetPickerMode::Grid,
    }
}

fn escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

fn ordered_pet_options<'a>(
    map: &'a std::collections::HashMap<String, bool>,
    order: &PrefsWriteOrder,
) -> Vec<(&'a str, bool)> {
    let mut keys: Vec<&str> = map
        .keys()
        .map(|k| k.as_str())
        .filter(|k| !is_reserved_pet_key(k))
        .collect();
    keys.sort_by_cached_key(|k| pet_option_sort_key(k, order));
    keys.into_iter()
        .filter_map(|k| map.get(k).map(|v| (k, *v)))
        .collect()
}

fn ordered_hud_entries<'a>(
    map: &'a std::collections::HashMap<String, HudConfigValue>,
    order: &PrefsWriteOrder,
) -> Vec<(&'a str, &'a HudConfigValue)> {
    let mut keys: Vec<&str> = map.keys().map(|k| k.as_str()).collect();
    keys.sort_by_cached_key(|k| hud_key_sort_key(k, order));
    keys.into_iter()
        .filter_map(|k| map.get(k).map(|v| (k, v)))
        .collect()
}

fn pet_option_sort_key(key: &str, order: &PrefsWriteOrder) -> (u32, u32, String) {
    if key.contains(".global.") {
        return (0, attr_priority_from_key(key) as u32, key.to_string());
    }
    let pet_id = match_prefix(key, &order.pet_ids).unwrap_or_else(|| default_package_id(key));
    let pet_rank = index_or_tail(&order.pet_ids, &pet_id);
    let opt = key.strip_prefix(&format!("{pet_id}.")).unwrap_or(key);
    let opt_rank = order
        .pet_option_keys
        .iter()
        .find(|(id, _)| id == &pet_id)
        .map(|(_, opts)| index_or_tail(opts, opt))
        .unwrap_or_else(|| attr_priority_from_key(opt) as u32);
    (1 + pet_rank, opt_rank, key.to_string())
}

fn hud_key_sort_key(key: &str, order: &PrefsWriteOrder) -> (u32, u32, u32, u32, String) {
    if key.contains(".global.") {
        return (0, 0, 0, attr_priority_from_key(key) as u32, key.to_string());
    }
    let plugin_id = match_prefix(key, &order.plugin_ids).unwrap_or_else(|| default_package_id(key));
    let plugin_rank = index_or_tail(&order.plugin_ids, &plugin_id);
    let rest = key.strip_prefix(&format!("{plugin_id}.")).unwrap_or("");
    if rest.is_empty() || rest == "enable" || rest == "id" {
        // 插件级 id/enable
        return (
            1 + plugin_rank,
            0,
            0,
            attr_priority(rest) as u32,
            key.to_string(),
        );
    }
    let (contrib, attr) = match rest.rsplit_once('.') {
        Some((c, a)) => (c, a),
        None => (rest, ""),
    };
    // 形如 plugin.enable 已处理；clock.enable → contrib=clock attr=enable
    let contrib_rank = order
        .plugin_contrib_ids
        .iter()
        .find(|(id, _)| id == &plugin_id)
        .map(|(_, cs)| index_or_tail(cs, contrib))
        .unwrap_or(u32::MAX / 4);
    (
        1 + plugin_rank,
        1,
        contrib_rank,
        attr_priority(attr) as u32,
        key.to_string(),
    )
}

fn match_prefix(key: &str, ids: &[String]) -> Option<String> {
    ids.iter()
        .filter(|id| key == id.as_str() || key.starts_with(&format!("{id}.")))
        .max_by_key(|id| id.len())
        .cloned()
}

/// 约定全 ID 为 `kind.org.name`（三段）；其余回退整键。
fn default_package_id(key: &str) -> String {
    let parts: Vec<&str> = key.split('.').collect();
    if parts.len() >= 3 {
        format!("{}.{}.{}", parts[0], parts[1], parts[2])
    } else {
        key.to_string()
    }
}

fn index_or_tail(list: &[String], id: &str) -> u32 {
    list.iter()
        .position(|x| x == id)
        .map(|i| i as u32)
        .unwrap_or(u32::MAX / 2)
}

fn attr_priority_from_key(key: &str) -> u8 {
    attr_priority(key.rsplit('.').next().unwrap_or(key))
}

fn attr_priority(attr: &str) -> u8 {
    match attr {
        "id" | "kind" => 0,
        "enable" => 1,
        "display" => 10,
        "x" => 20,
        "y" => 21,
        "width" | "w" => 22,
        "height" | "h" => 23,
        "scale" => 24,
        "topmost" => 30,
        "picker_mode" => 31,
        _ => 40,
    }
}

fn format_hud_value(v: &HudConfigValue) -> String {
    match v {
        HudConfigValue::Bool(b) => b.to_string(),
        HudConfigValue::Int(i) => i.to_string(),
        HudConfigValue::Float(f) => {
            if f.fract() == 0.0 {
                format!("{f:.1}")
            } else {
                f.to_string()
            }
        }
        HudConfigValue::String(s) => format!("\"{}\"", escape(s)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::migrate_pet_id;

    #[test]
    fn format_and_reload_new_shape() {
        let mut prefs = UiPreferences::default();
        prefs.locale = Locale::En;
        prefs.pet.kind = "pet.deskhud.blob".into();
        prefs.pet.width = 96.0;
        prefs.pet.height = 96.0;
        prefs.pet.pos_x = Some(120.0);
        prefs.shell.topmost = false;
        prefs.pet.set_bool("pet.deskhud.specs.config1", true);
        prefs.hud.set_enabled("hud.deskhud.demo", "clock", false);
        prefs.hud.set_slot_layout("hud.deskhud.demo", "tip", {
            let mut s = HudSlotLayout::default();
            s.x = 0.5;
            s.y = 0.8;
            s.scale = 1.25;
            s
        });

        let text = format_prefs(&prefs);
        assert!(text.contains("[theme]\n"));
        assert!(text.contains("locale = \"en\""));
        assert!(!text.starts_with("locale ="));
        assert!(text.contains("[prefs]\n"));
        assert!(
            text.find("[prefs]").unwrap() < text.find("[theme]").unwrap(),
            "prefs section should precede theme"
        );
        assert!(text.contains("[font]\n"));
        assert!(text.contains("[pet]\n"));
        assert!(text.contains("[hud]\n"));
        assert!(!text.contains("[ui]\n"));
        assert!(!text.contains("[shell]"));
        assert!(!text.contains("pet.config"));
        assert!(!text.contains("hud.config"));
        assert!(text.contains("\"pet.global.kind\" = \"pet.deskhud.blob\""));
        assert!(text.contains("\"pet.global.size\""));
        assert!(text.contains("topmost = false"));
        assert!(!text.contains("pet.global.topmost"));
        assert!(!text.contains("\nkind = "));
        assert!(text.contains("id = \""));
        assert!(text.contains("\"hud.deskhud.demo.tip.x\""));
        // enable 应排在同条目 layout 属性前
        if let (Some(e), Some(x)) = (
            text.find("\"hud.deskhud.demo.clock.enable\""),
            text.find("\"hud.deskhud.demo.tip.x\""),
        ) {
            assert!(e < x);
        }

        let value: toml::Value = toml::from_str(&text).unwrap();
        let back = prefs_from_value(value);
        assert_eq!(back.locale, Locale::En);
        assert_eq!(back.pet.kind, "pet.deskhud.blob");
        assert!((back.pet.width - 96.0).abs() < 1e-3);
        assert!(!back.shell.topmost);
        assert!(back.pet.get_bool("pet.deskhud.specs.config1", false));
        assert!(!back.hud.is_enabled("hud.deskhud.demo", "clock", true));
        let tip = back.hud.slot_layout("hud.deskhud.demo", "tip", 0);
        assert!((tip.x - 0.5).abs() < 1e-3);
        assert!((tip.scale - 1.25).abs() < 1e-3);
    }

    #[test]
    fn migrate_old_shell_and_nested_config() {
        let text = r#"
locale = "zh-cn"

[shell]
active_pet_kind_id = "pet.deskhud.specs"
pet_width = 140.0
pet_height = 140.0
pet_topmost = true
ui_theme = "dark"
ui_font_size = 14.0

[pet.config]
"pet.deskhud.specs.follow_eyes" = false

[hud.config]
"hud.deskhud.demo.enable" = true
"hud.deskhud.demo.clock.enable" = false
"#;
        let value: toml::Value = toml::from_str(text).unwrap();
        let mut prefs = prefs_from_value(value);
        prefs.normalize_ids();
        assert_eq!(prefs.pet.kind, "pet.deskhud.specs");
        assert!((prefs.pet.width - 140.0).abs() < 1e-3);
        assert!(prefs.shell.topmost);
        assert_eq!(prefs.shell.ui_theme, UiTheme::Dark);
        assert!((prefs.shell.ui_font_size - 14.0).abs() < 1e-3);
        assert!(!prefs.pet.get_bool("pet.deskhud.specs.follow_eyes", true));
        assert!(prefs.hud.is_plugin_enabled("hud.deskhud.demo"));
        assert!(!prefs.hud.is_enabled("hud.deskhud.demo", "clock", true));
    }

    #[test]
    fn migrate_font_section_and_global_keys() {
        let text = r#"
locale = "en"

[ui]
theme = "light"
font_id = "builtin.NotoSansSC-Regular"
font_family = "fam.notosanssc"
font_style = "Regular"
font_size = 15.0

[pet]
kind = "pet.deskhud.blob"
width = 100.0
height = 100.0
topmost = false
picker_mode = "list"

[hud]
"master.enable" = false
"#;
        let value: toml::Value = toml::from_str(text).unwrap();
        let mut prefs = prefs_from_value(value);
        prefs.normalize_ids();
        assert_eq!(prefs.shell.ui_font_id, "NotoSansSC-Regular");
        assert_eq!(prefs.shell.ui_font_family, "notosanssc");
        assert!((prefs.shell.ui_font_size - 15.0).abs() < 1e-3);
        assert!(!prefs.shell.topmost);
        assert_eq!(prefs.pet.picker_mode, PetPickerMode::List);
        assert!(!prefs.hud.is_master_enabled());
        assert_eq!(
            prefs.hud.config.get("hud.global.enable"),
            Some(&HudConfigValue::Bool(false))
        );
        assert!(!prefs.hud.config.contains_key("master.enable"));

        let out = format_prefs(&prefs);
        assert!(out.contains("[theme]\n"));
        assert!(out.contains("mode = \"light\""));
        assert!(out.contains("locale = \"en\""));
        assert!(out.contains("[font]\n"));
        assert!(out.contains("id = \"NotoSansSC-Regular\""));
        assert!(out.contains("family = \"notosanssc\""));
        assert!(!out.contains("\nfont_id = "));
        assert!(!out.contains("builtin."));
        assert!(!out.contains("fam."));
        assert!(out.contains("\"pet.global.kind\" = \"pet.deskhud.blob\""));
        assert!(out.contains("topmost = false"));
        assert!(!out.contains("pet.global.topmost"));
        assert!(out.contains("\"hud.global.enable\" = false"));
        // global 开关应排在其它 hud 键前
        let g = out.find("\"hud.global.enable\"").unwrap();
        assert!(out[g..].contains("\"hud.global.enable\" = false"));
    }

    #[test]
    fn locale_in_theme_section() {
        let text = r#"
[theme]
mode = "dark"
locale = "en"
"#;
        let value: toml::Value = toml::from_str(text).unwrap();
        let prefs = prefs_from_value(value);
        assert_eq!(prefs.locale, Locale::En);
        assert_eq!(prefs.shell.ui_theme, UiTheme::Dark);
        let out = format_prefs(&prefs);
        let settings_at = out.find("[prefs]").unwrap();
        let theme_at = out.find("[theme]").unwrap();
        let locale_at = out.find("locale = \"en\"").unwrap();
        assert!(settings_at < theme_at);
        assert!(locale_at > theme_at);
    }

    #[test]
    fn hud_and_pet_keys_follow_registry_order() {
        let mut prefs = UiPreferences::default();
        prefs.pet.set_bool("pet.deskhud.specs.follow_eyes", true);
        prefs.pet.set_bool("pet.deskhud.specs.key_tips", false);
        prefs.pet.set_bool("pet.deskhud.blob.bounce", true);
        prefs.hud.set_plugin_enabled("hud.deskhud.demo", true);
        prefs.hud.set_enabled("hud.deskhud.demo", "tip", true);
        prefs.hud.set_enabled("hud.deskhud.demo", "clock", false);

        let order = PrefsWriteOrder {
            pet_ids: vec!["pet.deskhud.specs".into(), "pet.deskhud.blob".into()],
            pet_option_keys: vec![
                (
                    "pet.deskhud.specs".into(),
                    vec!["key_tips".into(), "follow_eyes".into()],
                ),
                ("pet.deskhud.blob".into(), vec!["bounce".into()]),
            ],
            plugin_ids: vec!["hud.deskhud.demo".into()],
            plugin_contrib_ids: vec![(
                "hud.deskhud.demo".into(),
                vec!["clock".into(), "tip".into()],
            )],
        };
        let out = format_prefs_ordered(&prefs, &order);
        let specs_tips = out.find("\"pet.deskhud.specs.key_tips\"").unwrap();
        let specs_eyes = out.find("\"pet.deskhud.specs.follow_eyes\"").unwrap();
        let blob = out.find("\"pet.deskhud.blob.bounce\"").unwrap();
        assert!(specs_tips < specs_eyes);
        assert!(specs_eyes < blob);

        let plugin_en = out.find("\"hud.deskhud.demo.enable\"").unwrap();
        let clock_en = out.find("\"hud.deskhud.demo.clock.enable\"").unwrap();
        let tip_en = out.find("\"hud.deskhud.demo.tip.enable\"").unwrap();
        assert!(plugin_en < clock_en);
        assert!(clock_en < tip_en);
    }

    #[test]
    fn migrate_old_pet_id() {
        assert_eq!(migrate_pet_id("builtin.specs"), "pet.deskhud.specs");
    }
}
