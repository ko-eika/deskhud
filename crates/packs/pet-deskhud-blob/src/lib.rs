//! 内置备选：简单蓝球（固定眼睛），用于「切换宠物」演示。

use std::sync::atomic::{AtomicBool, Ordering};

use deskhud_engine::{
    PetConfigBag, PetConfigOption, PetKind, PetKindInfo, PetPaint, PetPaintCtx, PetScene,
    SceneItem, SceneNode, Shape, Transform2D,
};

/// `pet.deskhud.blob`。
#[derive(Debug)]
pub struct BuiltinBlobPet {
    hover_pulse: AtomicBool,
    dock_tint: AtomicBool,
    drag_react: AtomicBool,
}

impl Default for BuiltinBlobPet {
    fn default() -> Self {
        Self {
            hover_pulse: AtomicBool::new(true),
            dock_tint: AtomicBool::new(true),
            drag_react: AtomicBool::new(true),
        }
    }
}

const BLOB_OPTIONS: &[PetConfigOption] = &[
    PetConfigOption {
        key: "hover_pulse",
        label: "悬停轻弹",
        description: "指针停在宠上时略微放大呼吸",
        default: true,
    },
    PetConfigOption {
        key: "dock_tint",
        label: "贴边变色",
        description: "吸附边缘时身体颜色变化",
        default: true,
    },
    PetConfigOption {
        key: "drag_react",
        label: "拖动反馈",
        description: "拖动时加强弹跳与提亮",
        default: true,
    },
];

impl PetKind for BuiltinBlobPet {
    fn info(&self) -> PetKindInfo {
        PetKindInfo {
            id: "pet.deskhud.blob",
            version: deskhud_engine::ENGINE_PRODUCT_VERSION,
            engine: deskhud_engine::ENGINE_COMPAT_FAMILY,
            display_name: "蓝点",
            description: "简洁圆润的蓝色小球，拖动和贴边时会轻微变形。",
            author: "DeskHud",
            homepage: Some("https://github.com/ko-eika/deskhud"),
            window_width: 96.0,
            window_height: 96.0,
            preview: Some(include_bytes!("../assets/preview.svg")),
        }
    }

    fn config_options(&self) -> &'static [PetConfigOption] {
        BLOB_OPTIONS
    }

    fn apply_config(&self, config: PetConfigBag<'_>) {
        self.hover_pulse
            .store(config.get("hover_pulse", true), Ordering::Relaxed);
        self.dock_tint
            .store(config.get("dock_tint", true), Ordering::Relaxed);
        self.drag_react
            .store(config.get("drag_react", true), Ordering::Relaxed);
    }

    fn paint(&self, ctx: PetPaintCtx<'_>) -> PetPaint {
        let _hover_pulse = ctx.config.get("hover_pulse", true);
        let dock_tint = ctx.config.get("dock_tint", true);
        let drag_react = ctx.config.get("drag_react", true);

        let dock = ctx.dock;
        let dragging = ctx.drag.is_dragging();
        let mut body = [0.25, 0.55, 0.95];
        if dock_tint {
            body = match (dock.left, dock.right, dock.top, dock.bottom) {
                (true, false, true, false) => [1.0, 0.78, 0.05],
                (false, true, true, false) => [1.0, 0.18, 0.52],
                (true, false, false, true) => [0.05, 0.88, 0.72],
                (false, true, false, true) => [0.68, 0.12, 1.0],
                (_, _, true, false) => [1.0, 0.48, 0.04],
                (_, _, false, true) => [0.05, 0.82, 0.32],
                (true, false, false, false) => [0.95, 0.08, 0.18],
                (false, true, false, false) => [0.82, 0.08, 0.72],
                _ => body,
            };
        }
        if dragging && drag_react {
            body = [0.45, 0.70, 1.0];
        }
        PetPaint {
            body_rgb: body,
            bubble_text: None,
            bubble_style: Default::default(),
        }
    }

    fn scene(&self, ctx: PetPaintCtx<'_>) -> PetScene {
        let paint = self.paint(ctx);
        let radius = if ctx.drag.active && self.drag_react.load(Ordering::Relaxed) {
            1.06
        } else {
            1.0
        };
        PetScene {
            items: vec![
                SceneItem {
                    transform: Transform2D {
                        scale: [radius, radius],
                        ..Transform2D::default()
                    },
                    z_index: 0,
                    node: SceneNode::Shape {
                        shape: Shape::Circle { radius: 1.0 },
                        color: [paint.body_rgb[0], paint.body_rgb[1], paint.body_rgb[2], 1.0],
                    },
                },
                SceneItem {
                    transform: Transform2D::default(),
                    z_index: -1,
                    node: SceneNode::HitRegion {
                        shape: Shape::Circle { radius: 1.0 },
                    },
                },
            ],
        }
    }
}
