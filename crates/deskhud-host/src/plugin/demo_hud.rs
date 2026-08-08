//! 演示插件：提供两条 HUD 贡献，方便验证「HUD 配置」菜单。

use super::{HudContribution, Plugin, PluginInfo};

/// `hud.deskhud.demo` — 演示用占位 HUD（时钟条 / 提示条画在宠窗内）。
#[derive(Debug, Default)]
pub struct DemoHudPlugin;

impl Plugin for DemoHudPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "hud.deskhud.demo",
            display_name: "演示 HUD",
            description: "示例插件：开关后在宠窗底部显示演示条（非真实数据源）",
            author: "DeskHud",
            homepage: Some("https://github.com/deskhud/deskhud"),
            icon_png: Some(include_bytes!("../../assets/icon_hud_demo.png")),
        }
    }

    fn hud_contributions(&self) -> &'static [HudContribution] {
        const ITEMS: &[HudContribution] = &[
            HudContribution {
                id: "clock",
                label: "演示时钟条",
                default_enabled: true,
                icon_png: Some(include_bytes!("../../assets/icon_hud_demo_clock.png")),
            },
            HudContribution {
                id: "tip",
                label: "演示提示条",
                default_enabled: false,
                icon_png: Some(include_bytes!("../../assets/icon_hud_demo_tip.png")),
            },
        ];
        ITEMS
    }
}
