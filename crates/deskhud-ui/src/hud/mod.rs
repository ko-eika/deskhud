//! HUD 开关偏好（`[hud.config]` + `.enable` 键）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 用户对插件 / HUD 条目的启用状态。
///
/// 包全 ID：`hud.<组织>.<标识>`；条目短 id 再拼一段。
///
/// 落盘示例：
/// ```toml
/// [hud.config]
/// "hud.deskhud.demo.enable" = true
/// "hud.deskhud.demo.clock.enable" = true
/// "hud.deskhud.demo.tip.enable" = false
/// ```
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct HudPrefs {
    /// 统一配置表；开关键以 `.enable` 结尾。
    #[serde(default)]
    pub config: HashMap<String, bool>,
    /// 旧版 `enabled` 表；仅反序列化合并，不再写出。
    #[serde(default, skip_serializing)]
    enabled: HashMap<String, bool>,
    /// 旧版插件总开关表；仅反序列化合并，不再写出。
    #[serde(default, skip_serializing)]
    plugin_enabled: HashMap<String, bool>,
}

impl HudPrefs {
    /// 插件总开关键：`{plugin_id}.enable`。
    pub fn plugin_enable_key(plugin_id: &str) -> String {
        format!("{plugin_id}.enable")
    }

    /// 条目开关键：`{plugin_id}.{contribution_id}.enable`。
    pub fn contribution_enable_key(plugin_id: &str, contribution_id: &str) -> String {
        format!("{plugin_id}.{contribution_id}.enable")
    }

    fn get_bool(&self, key: &str) -> Option<bool> {
        self.config
            .get(key)
            .or_else(|| self.enabled.get(key))
            .copied()
    }

    /// 插件总开关；未配置时默认开启。
    pub fn is_plugin_enabled(&self, plugin_id: &str) -> bool {
        let key = Self::plugin_enable_key(plugin_id);
        if let Some(v) = self.get_bool(&key) {
            return v;
        }
        // 兼容：裸插件 id 当开关
        if let Some(v) = self.get_bool(plugin_id) {
            return v;
        }
        if let Some(v) = self.plugin_enabled.get(plugin_id).copied() {
            return v;
        }
        // 兼容旧 id
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
        self.config.insert(key, on);
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
        // 兼容：无 `.enable` 后缀
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
        self.get_bool(contribution_id)
            .unwrap_or(default_enabled)
    }

    /// 设置条目启用状态。
    pub fn set_enabled(&mut self, plugin_id: &str, contribution_id: &str, on: bool) {
        let key = Self::contribution_enable_key(plugin_id, contribution_id);
        self.config.insert(key, on);
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
            self.config
                .remove(&format!("{legacy}.{contribution_id}"));
            self.config
                .remove(&format!("{legacy}/{contribution_id}"));
            if legacy == "demo.hud" {
                self.config
                    .remove(&format!("demo.{contribution_id}"));
            }
        }
    }

    /// 插件开启 **且** 条目开启时才真正显示。
    pub fn is_active(
        &self,
        plugin_id: &str,
        contribution_id: &str,
        default_enabled: bool,
    ) -> bool {
        self.is_plugin_enabled(plugin_id)
            && self.is_enabled(plugin_id, contribution_id, default_enabled)
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
    fn enable_suffix_keys() {
        let mut hud = HudPrefs::default();
        hud.set_plugin_enabled("hud.deskhud.demo", false);
        hud.set_enabled("hud.deskhud.demo", "clock", true);
        assert!(!hud.is_plugin_enabled("hud.deskhud.demo"));
        assert!(!hud.is_active("hud.deskhud.demo", "clock", true));
        assert!(hud.is_enabled("hud.deskhud.demo", "clock", false));
        assert_eq!(
            hud.config.get("hud.deskhud.demo.enable").copied(),
            Some(false)
        );
        assert_eq!(
            hud.config
                .get("hud.deskhud.demo.clock.enable")
                .copied(),
            Some(true)
        );
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
        assert!(text.contains("[config]"));
        assert!(text.contains("hud.deskhud.demo.enable"));
        assert!(text.contains("hud.deskhud.demo.clock.enable"));
        assert!(!text.contains("plugin_enabled"));
        let back: HudPrefs = toml::from_str(&text).expect("de");
        assert!(back.is_plugin_enabled("hud.deskhud.demo"));
        assert!(!back.is_enabled("hud.deskhud.demo", "clock", true));
        assert!(back.is_enabled("hud.deskhud.demo", "tip", false));
    }
}
