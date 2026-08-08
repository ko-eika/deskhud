//! 宠物包偏好（`[pet.config]`）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

/// 宠物包级配置（键为全 ID 路径，如 `pet.deskhud.specs.follow_eyes`）。
#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct PetPrefs {
    /// 包自定义布尔配置。
    #[serde(default)]
    pub config: HashMap<String, bool>,
}

impl PetPrefs {
    /// 规范键：`{pet_id}.{option_key}`。
    pub fn option_key(pet_id: &str, option_key: &str) -> String {
        format!("{pet_id}.{option_key}")
    }

    /// 读取布尔配置；缺省 `default`。
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.config.get(key).copied().unwrap_or(default)
    }

    /// 读取宠选项。
    pub fn get_option(&self, pet_id: &str, option_key: &str, default: bool) -> bool {
        self.get_bool(&Self::option_key(pet_id, option_key), default)
    }

    /// 写入布尔配置。
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.config.insert(key.into(), value);
    }

    /// 写入宠选项。
    pub fn set_option(&mut self, pet_id: &str, option_key: &str, value: bool) {
        self.set_bool(Self::option_key(pet_id, option_key), value);
    }

    /// 解析当前宠的短键表（供 `PetConfigBag`）。
    pub fn short_map_for(
        &self,
        pet_id: &str,
        options: &[(&str, bool)],
    ) -> HashMap<String, bool> {
        let mut map = HashMap::new();
        for &(key, default) in options {
            map.insert(key.to_string(), self.get_option(pet_id, key, default));
        }
        map
    }
}
