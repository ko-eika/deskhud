//! 引擎注册表。

use std::sync::Arc;

use crate::pet::{PetKind, PetKindInfo};
use crate::plugin::{
    HudConfigDynamicChoice, HudContribution, HudFrame, HudFrameCtx, Plugin, PluginInfo,
};

/// 引擎运行时注册表（宠物 + HUD 插件）。
pub struct EngineRegistry {
    pets: Vec<Arc<dyn PetKind>>,
    plugins: Vec<Arc<dyn Plugin>>,
    active_pet_id: String,
}

impl Default for EngineRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl EngineRegistry {
    /// 空注册表（不预装内置宠 / 插件；由 runtime 引导注册）。
    pub fn new() -> Self {
        Self::empty()
    }

    /// [`Self::new`] 的别名。
    pub fn empty() -> Self {
        Self {
            pets: Vec::new(),
            plugins: Vec::new(),
            active_pet_id: String::new(),
        }
    }

    /// 注册宠物（同 ID 替换）。若当前无激活宠，则设为该宠。
    pub fn register_pet(&mut self, kind: Arc<dyn PetKind>) {
        let id = kind.info().id;
        self.pets.retain(|p| p.info().id != id);
        self.pets.push(kind);
        if self.active_pet_id.is_empty()
            || !self.pets.iter().any(|p| p.info().id == self.active_pet_id)
        {
            self.active_pet_id = id.to_string();
        }
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
    ///
    /// # Panics
    /// 注册表中没有任何宠物时 panic（引导层须先 `register_pet`）。
    pub fn active_pet(&self) -> Arc<dyn PetKind> {
        self.pets
            .iter()
            .find(|p| p.info().id == self.active_pet_id)
            .cloned()
            .or_else(|| self.pets.first().cloned())
            .expect("EngineRegistry has no pets; bootstrap must register builtins first")
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

    /// Returns current choices for one plugin-owned dynamic HUD option.
    pub fn hud_config_choices(
        &self,
        plugin_id: &str,
        contribution_id: &str,
        option_key: &str,
    ) -> Vec<HudConfigDynamicChoice> {
        self.plugins
            .iter()
            .find(|plugin| plugin.info().id == plugin_id)
            .map(|plugin| plugin.hud_config_choices(contribution_id, option_key))
            .unwrap_or_default()
    }

    /// Produces frames for the contributions selected by the caller's prefs.
    pub fn hud_frame(&self, plugin_id: &str, contribution_id: &str, elapsed_secs: f32) -> HudFrame {
        self.plugins
            .iter()
            .find(|plugin| plugin.info().id == plugin_id)
            .map(|plugin| plugin.hud_frame(contribution_id, elapsed_secs))
            .unwrap_or_else(HudFrame::empty)
    }

    /// Produces a frame for one stable host-owned HUD instance.
    pub fn hud_frame_for_instance(&self, ctx: &HudFrameCtx<'_>) -> HudFrame {
        self.plugins
            .iter()
            .find(|plugin| plugin.info().id == ctx.source.plugin_id)
            .map(|plugin| plugin.hud_frame_for_instance(ctx))
            .unwrap_or_else(HudFrame::empty)
    }
}

#[cfg(test)]
mod tests {
    use super::EngineRegistry;
    use crate::{HudFrame, HudFrameCtx, HudInstanceId, HudSourceId, HudVisual, Plugin, PluginInfo};
    use std::sync::Arc;

    struct TestPlugin;
    impl Plugin for TestPlugin {
        fn info(&self) -> PluginInfo {
            PluginInfo {
                id: "hud.test.frame",
                display_name: "test",
                description: "test",
                author: "test",
                homepage: None,
                version: "0.1.0",
                engine: "0.5",
                icon: None,
            }
        }
        fn hud_frame(&self, id: &str, _elapsed_secs: f32) -> HudFrame {
            if id == "clock" {
                HudFrame {
                    visuals: vec![HudVisual::Text {
                        text: "ok".into(),
                        font_size: 12.0,
                        color: [255; 4],
                    }],
                }
            } else {
                HudFrame::empty()
            }
        }
    }

    #[test]
    fn forwards_hud_frame_to_registered_plugin() {
        let mut registry = EngineRegistry::empty();
        registry.register_plugin(Arc::new(TestPlugin));
        assert!(
            !registry
                .hud_frame("hud.test.frame", "clock", 1.0)
                .is_empty()
        );
        assert!(
            registry
                .hud_frame("hud.test.frame", "missing", 1.0)
                .is_empty()
        );
        let instance_id = HudInstanceId::new("instance:1");
        let source = HudSourceId::new("hud.test.frame", "clock");
        assert!(
            !registry
                .hud_frame_for_instance(&HudFrameCtx {
                    instance_id: &instance_id,
                    source: &source,
                    config: &std::collections::HashMap::new(),
                    locale: "en-US",
                    elapsed_secs: 1.0,
                })
                .is_empty()
        );
    }
}
