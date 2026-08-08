//! 宿主注册表。

use std::sync::Arc;

use crate::pet::{BuiltinBlobPet, BuiltinSpecsPet, PetKind, PetKindInfo};
use crate::plugin::{DemoHudPlugin, HudContribution, Plugin, PluginInfo};

/// 运行时宿主。
pub struct HostRegistry {
    pets: Vec<Arc<dyn PetKind>>,
    plugins: Vec<Arc<dyn Plugin>>,
    active_pet_id: String,
}

impl Default for HostRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl HostRegistry {
    /// 默认：大眼球 + 蓝点；注册演示 HUD 插件。
    pub fn new() -> Self {
        let specs = Arc::new(BuiltinSpecsPet::default()) as Arc<dyn PetKind>;
        let blob = Arc::new(BuiltinBlobPet::default()) as Arc<dyn PetKind>;
        let demo = Arc::new(DemoHudPlugin) as Arc<dyn Plugin>;
        Self {
            active_pet_id: specs.info().id.to_string(),
            pets: vec![specs, blob],
            plugins: vec![demo],
        }
    }

    /// 注册宠物（同 ID 替换）。
    pub fn register_pet(&mut self, kind: Arc<dyn PetKind>) {
        let id = kind.info().id;
        self.pets.retain(|p| p.info().id != id);
        self.pets.push(kind);
    }

    /// 注册插件（同 ID 替换）。
    pub fn register_plugin(&mut self, plugin: Arc<dyn Plugin>) {
        let id = plugin.info().id;
        self.plugins.retain(|p| p.info().id != id);
        self.plugins.push(plugin);
    }

    /// 当前宠物 ID。
    pub fn active_pet_id(&self) -> &str {
        &self.active_pet_id
    }

    /// 切换宠物。
    pub fn set_active_pet(&mut self, id: &str) -> bool {
        if self.pets.iter().any(|p| p.info().id == id) {
            self.active_pet_id = id.to_string();
            true
        } else {
            false
        }
    }

    /// 当前宠物。
    pub fn active_pet(&self) -> Arc<dyn PetKind> {
        self.pets
            .iter()
            .find(|p| p.info().id == self.active_pet_id)
            .cloned()
            .unwrap_or_else(|| self.pets[0].clone())
    }

    /// 已注册宠物（含绘制逻辑，供设置预览等）。
    pub fn pets(&self) -> Vec<Arc<dyn PetKind>> {
        self.pets.clone()
    }

    /// 宠物元数据列表。
    pub fn pet_infos(&self) -> Vec<PetKindInfo> {
        self.pets.iter().map(|p| p.info()).collect()
    }

    /// 插件列表。
    pub fn plugin_infos(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|p| p.info()).collect()
    }

    /// `(plugin_id, contribution)` 汇总。
    pub fn all_hud_contributions(&self) -> Vec<(&'static str, HudContribution)> {
        let mut out = Vec::new();
        for plugin in &self.plugins {
            let pid = plugin.info().id;
            for c in plugin.hud_contributions() {
                out.push((pid, c.clone()));
            }
        }
        out
    }
}
