//! HUD 开关与布局偏好（落盘 `[hud]` 扁平键，便于扩展）。

mod layout;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

pub use layout::HudSlotLayout;

/// `[hud]` 里单个键的值：bool / 数字 / 字符串。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HudConfigValue {
    /// 开关类（`.enable`）。
    Bool(bool),
    /// 整数（反序列化兜底，读布局时当 float）。
    Int(i64),
    /// 浮点（`.x` / `.y` / `.scale`）。
    Float(f64),
    /// 位置元组（`.position = [x, y]`）。
    Position([f64; 2]),
    /// 字符串（`.display` 等）。
    String(String),
}

impl HudConfigValue {
    fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(v) => Some(*v),
            _ => None,
        }
    }

    fn as_f32(&self) -> Option<f32> {
        match self {
            Self::Float(v) => Some(*v as f32),
            Self::Int(v) => Some(*v as f32),
            Self::Bool(_) | Self::String(_) | Self::Position(_) => None,
        }
    }

    fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(s) => Some(s.as_str()),
            _ => None,
        }
    }
}

/// 用户对插件 / HUD 条目的启用状态与屏幕布局。
///
/// ```toml
/// [hud]
/// "hud.global.enable" = true
/// "hud.deskhud.demo.enable" = true
/// "hud.deskhud.demo.tip.enable" = true
/// "hud.deskhud.demo.tip.display" = "primary"
/// "hud.deskhud.demo.tip.x" = 0.54
/// "hud.deskhud.demo.tip.y" = 0.82
/// "hud.deskhud.demo.tip.scale" = 1.0
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HudPrefs {
    /// 统一扁平配置表（直接落在 `[hud]` 下）。
    #[serde(default, flatten)]
    pub config: HashMap<String, HudConfigValue>,
    /// 旧版嵌套 `layout` 表；仅读入迁移，不再写出。
    #[serde(default, skip_serializing)]
    layout: HashMap<String, HudSlotLayout>,
    /// 旧版 `enabled` 表；仅反序列化合并，不再写出。
    #[serde(default, skip_serializing)]
    enabled: HashMap<String, bool>,
    /// 旧版插件总开关表；仅反序列化合并，不再写出。
    #[serde(default, skip_serializing)]
    plugin_enabled: HashMap<String, bool>,
}

impl HudPrefs {
    /// 全局 HUD 总开关键：`hud.global.enable`。
    pub const MASTER_ENABLE_KEY: &'static str = "hud.global.enable";
    /// 旧全局开关键（读入迁移）。
    pub const LEGACY_MASTER_ENABLE_KEY: &'static str = "master.enable";

    /// 插件总开关键：`{plugin_id}.enable`。
    pub fn plugin_enable_key(plugin_id: &str) -> String {
        format!("{plugin_id}.enable")
    }

    /// 条目开关键：`{plugin_id}.{contribution_id}.enable`。
    pub fn contribution_enable_key(plugin_id: &str, contribution_id: &str) -> String {
        format!("{plugin_id}.{contribution_id}.enable")
    }

    /// 布局属性键：`{plugin_id}.{contribution_id}.{attr}`。
    pub fn layout_attr_key(plugin_id: &str, contribution_id: &str, attr: &str) -> String {
        format!("{plugin_id}.{contribution_id}.{attr}")
    }

    fn get_position(&self, key: &str) -> Option<[f32; 2]> {
        match self.config.get(key) {
            Some(HudConfigValue::Position(v)) => Some([v[0] as f32, v[1] as f32]),
            _ => None,
        }
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.config
            .get(key)
            .and_then(HudConfigValue::as_bool)
            .or_else(|| self.enabled.get(key).copied())
    }

    fn get_f32(&self, key: &str) -> Option<f32> {
        self.config.get(key).and_then(HudConfigValue::as_f32)
    }

    fn get_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(HudConfigValue::as_str)
    }

    /// 全局 HUD 总开关；未配置时默认开启。关闭后不渲染任何 HUD。
    pub fn is_master_enabled(&self) -> bool {
        self.get_bool(Self::MASTER_ENABLE_KEY)
            .or_else(|| self.get_bool(Self::LEGACY_MASTER_ENABLE_KEY))
            .unwrap_or(true)
    }

    /// 设置全局 HUD 总开关。
    pub fn set_master_enabled(&mut self, on: bool) {
        self.config
            .insert(Self::MASTER_ENABLE_KEY.into(), HudConfigValue::Bool(on));
        self.config.remove(Self::LEGACY_MASTER_ENABLE_KEY);
    }

    /// 将旧键 `master.enable` 迁到 `hud.global.enable`。
    pub fn migrate_global_keys(&mut self) {
        if self.config.contains_key(Self::MASTER_ENABLE_KEY) {
            self.config.remove(Self::LEGACY_MASTER_ENABLE_KEY);
            return;
        }
        if let Some(v) = self.config.remove(Self::LEGACY_MASTER_ENABLE_KEY) {
            self.config.insert(Self::MASTER_ENABLE_KEY.into(), v);
        }
    }

    /// 插件总开关；未配置时默认开启。
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let key = Self::plugin_enable_key(plugin_id);
        if let Some(v) = self.get_bool(&key) {
            return v;
        }
        if let Some(v) = self.get_bool(plugin_id) {
            return v;
        }
        if let Some(v) = self.plugin_enabled.get(plugin_id).copied() {
            return v;
        }
        for &legacy in legacy_plugin_ids(plugin_id) {
            if let Some(v) = self.get_bool(&Self::plugin_enable_key(legacy)) {
                return v;
            }
            if let Some(v) = self.get_bool(legacy) {
                return v;
            }
            if let Some(v) = self.plugin_enabled.get(legacy).copied() {
                return v;
            }
        }
        true
    }

    /// 设置插件总开关。
    pub fn set_plugin_enabled(&mut self, plugin_id: impl Into<String>, on: bool) {
        let id = plugin_id.into();
        let key = Self::plugin_enable_key(&id);
        self.config.insert(key, HudConfigValue::Bool(on));
        self.config.remove(&id);
        self.enabled.remove(&id);
        self.plugin_enabled.remove(&id);
        for &legacy in legacy_plugin_ids(&id) {
            self.config.remove(&Self::plugin_enable_key(legacy));
            self.config.remove(legacy);
            self.enabled.remove(legacy);
            self.plugin_enabled.remove(legacy);
        }
    }

    /// 条目开关（不含插件总开关）；未配置时回落到 `default_enabled`。
    pub fn is_enabled(
        &self,
        plugin_id: &str,
        contribution_id: &str,
        default_enabled: bool,
    ) -> bool {
        let key = Self::contribution_enable_key(plugin_id, contribution_id);
        if let Some(v) = self.get_bool(&key) {
            return v;
        }
        let dotted = format!("{plugin_id}.{contribution_id}");
        if let Some(v) = self.get_bool(&dotted) {
            return v;
        }
        let slash = format!("{plugin_id}/{contribution_id}");
        if let Some(v) = self.get_bool(&slash) {
            return v;
        }
        for &legacy in legacy_plugin_ids(plugin_id) {
            let k = Self::contribution_enable_key(legacy, contribution_id);
            if let Some(v) = self.get_bool(&k) {
                return v;
            }
            let d = format!("{legacy}.{contribution_id}");
            if let Some(v) = self.get_bool(&d) {
                return v;
            }
            let s = format!("{legacy}/{contribution_id}");
            if let Some(v) = self.get_bool(&s) {
                return v;
            }
            if legacy == "demo.hud" {
                let very_old = format!("demo.{contribution_id}");
                if let Some(v) = self.get_bool(&very_old) {
                    return v;
                }
            }
        }
        self.get_bool(contribution_id).unwrap_or(default_enabled)
    }

    /// 设置条目启用状态。
    pub fn set_enabled(&mut self, plugin_id: &str, contribution_id: &str, on: bool) {
        let key = Self::contribution_enable_key(plugin_id, contribution_id);
        self.config.insert(key, HudConfigValue::Bool(on));
        self.config
            .remove(&format!("{plugin_id}.{contribution_id}"));
        self.config
            .remove(&format!("{plugin_id}/{contribution_id}"));
        self.config.remove(contribution_id);
        self.enabled
            .remove(&format!("{plugin_id}.{contribution_id}"));
        self.enabled
            .remove(&format!("{plugin_id}/{contribution_id}"));
        self.enabled.remove(contribution_id);
        for &legacy in legacy_plugin_ids(plugin_id) {
            self.config
                .remove(&Self::contribution_enable_key(legacy, contribution_id));
            self.config.remove(&format!("{legacy}.{contribution_id}"));
            self.config.remove(&format!("{legacy}/{contribution_id}"));
            if legacy == "demo.hud" {
                self.config.remove(&format!("demo.{contribution_id}"));
            }
        }
    }

    /// 全局总开关、插件开启 **且** 条目开启时才真正显示。
    pub fn is_active(&self, plugin_id: &str, contribution_id: &str, default_enabled: bool) -> bool {
        self.is_master_enabled()
            && self.is_plugin_enabled(plugin_id)
            && self.is_enabled(plugin_id, contribution_id, default_enabled)
    }

    /// 布局键前缀：`{plugin_id}.{contribution_id}`。
    pub fn layout_key(plugin_id: &str, contribution_id: &str) -> String {
        format!("{plugin_id}.{contribution_id}")
    }

    /// 读取布局；优先扁平键，其次旧 `layout` 表，再默认槽。
    pub fn slot_layout(
        &self,
        plugin_id: &str,
        contribution_id: &str,
        index: usize,
    ) -> HudSlotLayout {
        let base = Self::layout_key(plugin_id, contribution_id);
        let has_flat = self.get_str(&format!("{base}.display")).is_some()
            || self.get_position(&format!("{base}.position")).is_some()
            || self.get_f32(&format!("{base}.x")).is_some()
            || self.get_f32(&format!("{base}.y")).is_some()
            || self.get_f32(&format!("{base}.scale")).is_some()
            || self.get_f32(&format!("{base}.w")).is_some()
            || self.get_f32(&format!("{base}.h")).is_some();

        if has_flat {
            let mut slot = HudSlotLayout::default_for_index(index);
            if let Some(d) = self.get_str(&format!("{base}.display")) {
                slot.display = d.to_string();
            }
            if let Some(x) = self.get_f32(&format!("{base}.x")) {
                slot.x = x;
            }
            if let Some(y) = self.get_f32(&format!("{base}.y")) {
                slot.y = y;
            }
            if let Some([x, y]) = self.get_position(&format!("{base}.position")) {
                slot.x = x;
                slot.y = y;
            }
            if let Some(s) = self.get_f32(&format!("{base}.scale")) {
                slot.scale = s;
            }
            if let Some(w) = self.get_f32(&format!("{base}.w")) {
                slot.set_legacy_w(w);
            }
            if let Some(h) = self.get_f32(&format!("{base}.h")) {
                slot.set_legacy_h(h);
            }
            return slot.compact_legacy();
        }

        if let Some(legacy) = self.layout.get(&base) {
            return legacy.clone().compact_legacy();
        }

        HudSlotLayout::default_for_index(index).compact_legacy()
    }

    /// 写入布局到扁平 `[hud.config]` 键（会 clamp）。
    pub fn set_slot_layout(
        &mut self,
        plugin_id: &str,
        contribution_id: &str,
        layout: HudSlotLayout,
    ) {
        let layout = layout.clamp01();
        let base = Self::layout_key(plugin_id, contribution_id);
        self.config.insert(
            format!("{base}.display"),
            HudConfigValue::String(layout.display.clone()),
        );
        self.config.insert(
            format!("{base}.position"),
            HudConfigValue::Position([layout.x as f64, layout.y as f64]),
        );
        self.config.insert(
            format!("{base}.scale"),
            HudConfigValue::Float(layout.scale as f64),
        );
        // 清旧嵌套表与遗留 w/h 扁平键
        self.layout.remove(&base);
        self.config.remove(&format!("{base}.x"));
        self.config.remove(&format!("{base}.y"));
        self.config.remove(&format!("{base}.w"));
        self.config.remove(&format!("{base}.h"));
    }

    /// 将 `other` 中的布局扁平键同步到 `self`（不影响 enable 等开关）。
    pub fn copy_layout_keys_from(&mut self, other: &Self) {
        const SUFFIXES: &[&str] = &[".display", ".x", ".y", ".scale", ".w", ".h"];
        for (k, v) in &other.config {
            if SUFFIXES.iter().any(|s| k.ends_with(s)) {
                self.config.insert(k.clone(), v.clone());
            }
        }
        // 旧嵌套表也并入读路径后清空写入侧
        for (key, layout) in &other.layout {
            if let Some((plugin, contrib)) = key.rsplit_once('.') {
                self.set_slot_layout(plugin, contrib, layout.clone());
            }
        }
    }
}

fn legacy_plugin_ids(plugin_id: &str) -> &'static [&'static str] {
    match plugin_id {
        "hud.deskhud.demo" => &["demo.hud"],
        _ => &[],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn master_enable_gates_all() {
        let mut hud = HudPrefs::default();
        hud.set_plugin_enabled("hud.deskhud.demo", true);
        hud.set_enabled("hud.deskhud.demo", "clock", true);
        assert!(hud.is_active("hud.deskhud.demo", "clock", true));
        hud.set_master_enabled(false);
        assert!(!hud.is_master_enabled());
        assert!(!hud.is_active("hud.deskhud.demo", "clock", true));
        assert!(hud.is_plugin_enabled("hud.deskhud.demo"));
        hud.set_master_enabled(true);
        assert!(hud.is_active("hud.deskhud.demo", "clock", true));
    }

    #[test]
    fn enable_suffix_keys() {
        let mut hud = HudPrefs::default();
        hud.set_plugin_enabled("hud.deskhud.demo", false);
        hud.set_enabled("hud.deskhud.demo", "clock", true);
        assert!(!hud.is_plugin_enabled("hud.deskhud.demo"));
        assert!(!hud.is_active("hud.deskhud.demo", "clock", true));
        assert!(hud.is_enabled("hud.deskhud.demo", "clock", false));
        assert_eq!(hud.get_bool("hud.deskhud.demo.enable"), Some(false));
        assert_eq!(hud.get_bool("hud.deskhud.demo.clock.enable"), Some(true));
    }

    #[test]
    fn cross_org_no_clash() {
        let mut hud = HudPrefs::default();
        hud.set_enabled("hud.acme.demo", "clock", true);
        hud.set_enabled("hud.deskhud.demo", "clock", false);
        assert!(hud.is_enabled("hud.acme.demo", "clock", false));
        assert!(!hud.is_enabled("hud.deskhud.demo", "clock", true));
    }

    #[test]
    fn legacy_keys_still_read() {
        let mut hud = HudPrefs::default();
        hud.plugin_enabled.insert("demo.hud".into(), false);
        hud.enabled.insert("demo.hud/clock".into(), false);
        assert!(!hud.is_plugin_enabled("hud.deskhud.demo"));
        assert!(!hud.is_enabled("hud.deskhud.demo", "clock", true));
    }

    #[test]
    fn toml_roundtrip_config_table() {
        let mut hud = HudPrefs::default();
        hud.set_plugin_enabled("hud.deskhud.demo", true);
        hud.set_enabled("hud.deskhud.demo", "clock", false);
        hud.set_enabled("hud.deskhud.demo", "tip", true);
        let text = toml::to_string_pretty(&hud).expect("ser");
        assert!(text.contains("hud.deskhud.demo.enable"));
        assert!(text.contains("hud.deskhud.demo.clock.enable"));
        assert!(!text.contains("plugin_enabled"));
        let back: HudPrefs = toml::from_str(&text).expect("de");
        assert!(back.is_plugin_enabled("hud.deskhud.demo"));
        assert!(!back.is_enabled("hud.deskhud.demo", "clock", true));
        assert!(back.is_enabled("hud.deskhud.demo", "tip", false));
    }

    #[test]
    fn layout_flat_keys_roundtrip() {
        let mut slot = HudSlotLayout::default();
        slot.display = "primary".into();
        slot.x = 0.12;
        slot.y = 0.34;
        slot.scale = 1.5;
        let mut hud = HudPrefs::default();
        hud.set_slot_layout("hud.deskhud.demo", "tip", slot);
        let text = toml::to_string_pretty(&hud).expect("ser");
        assert!(
            text.contains("hud.deskhud.demo.tip.position"),
            "position tuple missing in:\n{text}"
        );
        assert!(
            text.contains("hud.deskhud.demo.tip.scale"),
            "flat scale missing in:\n{text}"
        );
        assert!(
            !text.contains("[layout"),
            "must not write nested layout table:\n{text}"
        );
        let back: HudPrefs = toml::from_str(&text).expect("de");
        let got = back.slot_layout("hud.deskhud.demo", "tip", 0);
        assert!((got.x - 0.12).abs() < 1e-4);
        assert!((got.y - 0.34).abs() < 1e-4);
        assert!((got.scale - 1.5).abs() < 1e-4);
        assert_eq!(got.display, "primary");
    }

    #[test]
    fn migrate_nested_layout_table() {
        let text = r#"
[layout."hud.deskhud.demo.clock"]
display = "primary"
x = 0.2
y = 0.3
scale = 1.25
"#;
        let hud: HudPrefs = toml::from_str(text).expect("de");
        let got = hud.slot_layout("hud.deskhud.demo", "clock", 0);
        assert!((got.x - 0.2).abs() < 1e-4);
        assert!((got.scale - 1.25).abs() < 1e-4);
    }
}
