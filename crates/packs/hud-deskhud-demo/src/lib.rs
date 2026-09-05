//! 演示插件：提供两条 HUD 贡献，方便验证「HUD 配置」菜单。

use deskhud_engine::{HudContribution, HudFrame, HudVisual, Plugin, PluginInfo};

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
                config: &[],
            },
            HudContribution {
                id: "tip",
                label: "演示提示条",
                default_enabled: false,
                icon: Some(include_bytes!("../assets/icon_tip.svg")),
                config: &[],
            },
        ];
        ITEMS
    }

    fn hud_frame(&self, contribution_id: &str, elapsed_secs: f32) -> HudFrame {
        let mut frame = HudFrame::empty();
        frame.visuals.push(HudVisual::Panel {
            width: 260.0,
            height: 56.0,
            radius: 12.0,
            color: [28, 32, 40, 232],
        });
        let text = match contribution_id {
            "clock" => format!(
                "DeskHud  ·  {:02}:{:02}:{:02}",
                (elapsed_secs as u64 / 3600) % 24,
                (elapsed_secs as u64 / 60) % 60,
                elapsed_secs as u64 % 60
            ),
            "tip" => "HUD 已启用".to_owned(),
            _ => return HudFrame::empty(),
        };
        frame.visuals.push(HudVisual::Text {
            text,
            font_size: 18.0,
            color: [248, 248, 252, 255],
        });
        frame
    }
}
