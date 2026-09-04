//! HUD 开关与布局偏好（落盘 `[hud]` 扁平键，便于扩展）。

mod layout;
mod model;

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::shell::LayerPreference;

pub use layout::{HUD_SIZE_FACTOR_MAX, HUD_SIZE_FACTOR_MIN, HudSlotLayout};
pub use model::{
    HudGroup, HudGroupMemberLayout, HudInstance, HudInstanceConfig, HudRecoveryReport,
};

/// `[hud]` 里单个键的值：bool / 数字 / 字符串。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum HudConfigValue {
    /// 开关类（`.enable`）。
    Bool(bool),
    /// 整数（反序列化兜底，读布局时当 float）。
    Int(i64),
    /// 浮点参数。
    Float(f64),
    /// 位置元组（`.position = [x, y]`）。
    Position([f64; 2]),
    /// 尺寸元组（`.size = [width, height]`）。
    Size([f64; 2]),
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
            Self::Bool(_) | Self::String(_) | Self::Position(_) | Self::Size(_) => None,
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
/// "hud.deskhud.demo.tip.size" = [1.0, 1.0]
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HudPrefs {
    /// HUD 桌面覆盖层级。
    #[serde(default, skip_serializing)]
    pub layer: LayerPreference,
    /// Stable HUD instances. Entries remain when their plugin is temporarily unavailable.
    #[serde(default)]
    pub instances: Vec<HudInstance>,
    /// User-defined groups in stable display order.
    #[serde(default)]
    pub groups: Vec<HudGroup>,
    /// Sources whose deterministic default instance was explicitly deleted.
    #[serde(default)]
    pub suppressed_default_sources: Vec<deskhud_engine::HudSourceId>,
    /// 统一扁平配置表（直接落在 `[hud]` 下）。
    #[serde(default, flatten)]
    pub config: HashMap<String, HudConfigValue>,
}

impl HudPrefs {
    /// Persisted representation revision for readable instance/group array tables.
    pub const MODEL_FORMAT_VERSION: i64 = 2;
    /// Internal key used to rewrite older equivalent TOML representations once.
    pub const MODEL_FORMAT_KEY: &'static str = "hud.global.model_format";
    /// HUD 层级键：`hud.global.layer`。
    pub const GLOBAL_LAYER_KEY: &'static str = "hud.global.layer";
    /// 全局 HUD 总开关键：`hud.global.enable`。
    pub const MASTER_ENABLE_KEY: &'static str = "hud.global.enable";

    /// Returns whether the persisted HUD model already uses the current representation.
    pub fn is_model_format_current(&self) -> bool {
        matches!(
            self.config.get(Self::MODEL_FORMAT_KEY),
            Some(HudConfigValue::Int(Self::MODEL_FORMAT_VERSION))
        )
    }

    /// Marks the in-memory preferences for the current HUD model representation.
    pub fn mark_model_format_current(&mut self) {
        self.config.insert(
            Self::MODEL_FORMAT_KEY.into(),
            HudConfigValue::Int(Self::MODEL_FORMAT_VERSION),
        );
    }

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

    fn get_size(&self, key: &str) -> Option<[f32; 2]> {
        match self.config.get(key) {
            Some(HudConfigValue::Size(v) | HudConfigValue::Position(v)) => {
                Some([v[0] as f32, v[1] as f32])
            }
            _ => None,
        }
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.config.get(key).and_then(HudConfigValue::as_bool)
    }

    fn get_f32(&self, key: &str) -> Option<f32> {
        self.config.get(key).and_then(HudConfigValue::as_f32)
    }

    fn get_str(&self, key: &str) -> Option<&str> {
        self.config.get(key).and_then(HudConfigValue::as_str)
    }

    /// 全局 HUD 总开关；未配置时默认开启。关闭后不渲染任何 HUD。
    pub fn is_master_enabled(&self) -> bool {
        self.get_bool(Self::MASTER_ENABLE_KEY).unwrap_or(true)
    }

    /// 设置全局 HUD 总开关。
    pub fn set_master_enabled(&mut self, on: bool) {
        self.config
            .insert(Self::MASTER_ENABLE_KEY.into(), HudConfigValue::Bool(on));
    }

    /// 插件总开关；未配置时默认开启。
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let key = Self::plugin_enable_key(plugin_id);
        self.get_bool(&key).unwrap_or(true)
    }

    /// 设置插件总开关。
    pub fn set_plugin_enabled(&mut self, plugin_id: impl Into<String>, on: bool) {
        let id = plugin_id.into();
        let key = Self::plugin_enable_key(&id);
        self.config.insert(key, HudConfigValue::Bool(on));
    }

    /// 条目开关（不含插件总开关）；未配置时回落到 `default_enabled`。
    pub fn is_enabled(
        &self,
        plugin_id: &str,
        contribution_id: &str,
        default_enabled: bool,
    ) -> bool {
        let source = deskhud_engine::HudSourceId::new(plugin_id, contribution_id);
        let default_id = Self::default_instance_id(&source);
        if let Some(instance) = self
            .instances
            .iter()
            .find(|instance| instance.id == default_id && instance.source == source)
        {
            return instance.enabled;
        }
        let key = Self::contribution_enable_key(plugin_id, contribution_id);
        self.get_bool(&key).unwrap_or(default_enabled)
    }

    /// 设置条目启用状态。
    pub fn set_enabled(&mut self, plugin_id: &str, contribution_id: &str, on: bool) {
        let source = deskhud_engine::HudSourceId::new(plugin_id, contribution_id);
        let default_id = Self::default_instance_id(&source);
        let key = Self::contribution_enable_key(plugin_id, contribution_id);
        if let Some(instance) = self
            .instances
            .iter_mut()
            .find(|instance| instance.id == default_id && instance.source == source)
        {
            instance.enabled = on;
            self.config.remove(&key);
            return;
        }
        self.config.insert(key, HudConfigValue::Bool(on));
        self.config
            .remove(&format!("{plugin_id}.{contribution_id}"));
        self.config
            .remove(&format!("{plugin_id}/{contribution_id}"));
        self.config.remove(contribution_id);
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

    /// 读取当前扁平布局键，缺失时返回默认槽。
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
            || self.get_f32(&format!("{base}.width")).is_some()
            || self.get_f32(&format!("{base}.height")).is_some()
            || self.config.contains_key(&format!("{base}.size"));

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
            if let Some([w, h]) = self.get_size(&format!("{base}.size")) {
                slot.width = w;
                slot.height = h;
            }
            if let Some(w) = self.get_f32(&format!("{base}.width")) {
                slot.width = w;
            }
            if let Some(h) = self.get_f32(&format!("{base}.height")) {
                slot.height = h;
            }
            return slot.clamp01();
        }
        HudSlotLayout::default_for_index(index)
    }

    /// Writes layout to the stable instance when it exists; otherwise keeps
    /// the flattened form only for one-time legacy migration.
    pub fn set_slot_layout(
        &mut self,
        plugin_id: &str,
        contribution_id: &str,
        layout: HudSlotLayout,
    ) {
        let layout = layout.clamp01();
        let updated_instance = if let Some(instance) = self.instances.iter_mut().find(|instance| {
            instance.source.plugin_id == plugin_id
                && instance.source.contribution_id == contribution_id
        }) {
            instance.layout = layout.clone();
            true
        } else {
            false
        };
        if updated_instance {
            self.clear_legacy_layout_keys(plugin_id, contribution_id);
            return;
        }
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
            format!("{base}.size"),
            HudConfigValue::Size([layout.width as f64, layout.height as f64]),
        );
        // Remove keys written by older versions once the new shape is saved.
        self.config.remove(&format!("{base}.width"));
        self.config.remove(&format!("{base}.height"));
    }

    fn clear_legacy_layout_keys(&mut self, plugin_id: &str, contribution_id: &str) {
        let base = Self::layout_key(plugin_id, contribution_id);
        for suffix in [
            "enable", "display", "position", "size", "x", "y", "width", "height", "scale",
        ] {
            self.config.remove(&format!("{base}.{suffix}"));
        }
    }

    /// Returns a visual tuning value for a HUD item. Visual values deliberately
    /// live beside the layout keys so a layout is one portable preset.
    pub fn visual_value(
        &self,
        plugin_id: &str,
        contribution_id: &str,
        name: &str,
        default: f32,
    ) -> f32 {
        let key = format!("{}.{}.{name}", plugin_id, contribution_id);
        self.get_f32(&key).unwrap_or(default).clamp(0.0, 1.0)
    }

    /// Stores a visual tuning value immediately usable by the HUD renderer.
    pub fn set_visual_value(
        &mut self,
        plugin_id: &str,
        contribution_id: &str,
        name: &str,
        value: f32,
    ) {
        let key = format!("{}.{}.{name}", plugin_id, contribution_id);
        self.config
            .insert(key, HudConfigValue::Float(value.clamp(0.0, 1.0) as f64));
    }

    /// Reads an instance-owned visual value, falling back to its source default.
    pub fn instance_visual_value(
        &self,
        instance_id: &deskhud_engine::HudInstanceId,
        name: &str,
        default: f32,
    ) -> f32 {
        let Some(instance) = self
            .instances
            .iter()
            .find(|instance| &instance.id == instance_id)
        else {
            return default.clamp(0.0, 1.0);
        };
        instance
            .config
            .get(name)
            .and_then(HudConfigValue::as_f32)
            .unwrap_or_else(|| {
                self.visual_value(
                    &instance.source.plugin_id,
                    &instance.source.contribution_id,
                    name,
                    default,
                )
            })
            .clamp(0.0, 1.0)
    }

    /// Stores a visual override owned by one stable HUD instance.
    pub fn set_instance_visual_value(
        &mut self,
        instance_id: &deskhud_engine::HudInstanceId,
        name: impl Into<String>,
        value: f32,
    ) -> bool {
        let Some(instance) = self
            .instances
            .iter_mut()
            .find(|instance| &instance.id == instance_id)
        else {
            return false;
        };
        instance.config.insert(
            name.into(),
            HudConfigValue::Float(value.clamp(0.0, 1.0) as f64),
        );
        true
    }

    /// 将 `other` 中的布局扁平键同步到 `self`（不影响 enable 等开关）。
    pub fn copy_layout_keys_from(&mut self, other: &Self) {
        const SUFFIXES: &[&str] = &[".display", ".position", ".size", ".width", ".height"];
        for (k, v) in &other.config {
            if SUFFIXES.iter().any(|s| k.ends_with(s)) {
                self.config.insert(k.clone(), v.clone());
            }
        }
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

    #[cfg(any())]
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
        let slot = HudSlotLayout {
            display: "primary".into(),
            x: 0.12,
            y: 0.34,
            width: 1.5,
            height: 1.5,
        };
        let mut hud = HudPrefs::default();
        hud.set_slot_layout("hud.deskhud.demo", "tip", slot);
        let text = toml::to_string_pretty(&hud).expect("ser");
        assert!(
            text.contains("hud.deskhud.demo.tip.position"),
            "position tuple missing in:\n{text}"
        );
        assert!(
            text.contains("hud.deskhud.demo.tip.size"),
            "size tuple missing in:\n{text}"
        );
        assert!(
            !text.contains("[layout"),
            "must not write nested layout table:\n{text}"
        );
        let back: HudPrefs = toml::from_str(&text).expect("de");
        let got = back.slot_layout("hud.deskhud.demo", "tip", 0);
        assert!((got.x - 0.12).abs() < 1e-4);
        assert!((got.y - 0.34).abs() < 1e-4);
        assert!((got.width - 1.5).abs() < 1e-4);
        assert!((got.height - 1.5).abs() < 1e-4);
        assert_eq!(got.display, "primary");
    }

    #[test]
    fn visual_values_roundtrip_and_clamp() {
        let mut hud = HudPrefs::default();
        hud.set_visual_value("hud.acme.demo", "clock", "background_opacity", 0.35);
        hud.set_visual_value("hud.acme.demo", "clock", "content_opacity", 2.0);
        assert!(
            (hud.visual_value("hud.acme.demo", "clock", "background_opacity", 1.0) - 0.35).abs()
                < 1e-5
        );
        assert_eq!(
            hud.visual_value("hud.acme.demo", "clock", "content_opacity", 1.0),
            1.0
        );
        let text = toml::to_string_pretty(&hud).expect("ser");
        let back: HudPrefs = toml::from_str(&text).expect("de");
        assert!(
            (back.visual_value("hud.acme.demo", "clock", "background_opacity", 1.0) - 0.35).abs()
                < 1e-5
        );
    }

    #[test]
    fn legacy_contribution_switch_controls_its_default_instance() {
        let mut hud = HudPrefs::default();
        let source = deskhud_engine::HudSourceId::new("hud.deskhud.demo", "clock");
        hud.ensure_default_instances([(source.clone(), true)]);
        hud.set_enabled(&source.plugin_id, &source.contribution_id, false);
        assert!(!hud.is_enabled(&source.plugin_id, &source.contribution_id, true));
        assert!(!hud.instances[0].enabled);
    }

    #[cfg(any())]
    #[test]
    fn migrate_nested_layout_table() {
        let text = r#"
[layout."hud.deskhud.demo.clock"]
display = "primary"
x = 0.2
y = 0.3
"#;
        let hud: HudPrefs = toml::from_str(text).expect("de");
        let got = hud.slot_layout("hud.deskhud.demo", "clock", 0);
        assert!((got.x - 0.2).abs() < 1e-4);
    }
}
