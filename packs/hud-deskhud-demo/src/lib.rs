//! 演示插件：提供两条 HUD 贡献，方便验证「HUD 配置」菜单。

use deskhud_engine::{HudContribution, Plugin, PluginInfo};

/// `hud.deskhud.demo` — 演示用占位 HUD（时钟条 / 提示条画在宠窗内）。
#[derive(Debug, Default)]
pub struct DemoHudPlugin;

impl Plugin for DemoHudPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "hud.deskhud.demo",
            version: deskhud_engine::ENGINE_PRODUCT_VERSION,
            engine: deskhud_engine::ENGINE_COMPAT_FAMILY,
            display_name: "演示 HUD",
            description: "示例插件：开关后在宠窗底部显示演示条（非真实数据源）",
            author: "DeskHud",
            homepage: Some("https://github.com/ko-eika/deskhud"),
            icon: Some(include_bytes!("../assets/icon.svg")),
        }
    }

    fn hud_contributions(&self) -> &'static [HudContribution] {
        const ITEMS: &[HudContribution] = &[
            HudContribution {
                id: "clock",
                label: "演示时钟条",
                default_enabled: true,
                icon: Some(include_bytes!("../assets/icon_clock.svg")),
            },
            HudContribution {
                id: "tip",
                label: "演示提示条",
                default_enabled: false,
                icon: Some(include_bytes!("../assets/icon_tip.svg")),
            },
        ];
        ITEMS
    }
}
