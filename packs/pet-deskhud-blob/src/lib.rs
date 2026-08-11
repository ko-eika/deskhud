//! 内置备选：简单蓝球（固定眼睛），用于「切换宠物」演示。

use std::sync::atomic::{AtomicBool, Ordering};

use deskhud_engine::{PetConfigBag, PetConfigOption, PetKind, PetKindInfo, PetPaint, PetPaintCtx};

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
            description: "简洁圆点；拖动/贴边略变形态",
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
        let hover_pulse = ctx.config.get("hover_pulse", true);
        let dock_tint = ctx.config.get("dock_tint", true);
        let drag_react = ctx.config.get("drag_react", true);

        let dock = ctx.dock;
        let dragging = ctx.drag.is_dragging();
        let bounce = if dragging && drag_react {
            1.06 + (ctx.time_secs * 4.0).sin() as f32 * 0.035
        } else if hover_pulse && ctx.mouse.hovering {
            1.03 + (ctx.time_secs * 2.2).sin() as f32 * 0.02
        } else if dock.is_free() {
            1.0 + (ctx.time_secs * 1.6).sin() as f32 * 0.02
        } else {
            0.96 + (ctx.time_secs * 1.2).sin() as f32 * 0.012
        };
        let mut body = [0.25, 0.55, 0.95];
        if dock_tint {
            if dock.bottom {
                body = [0.20, 0.48, 0.88];
            } else if !dock.is_free() {
                body = [0.30, 0.62, 0.92];
            }
        }
        if dragging && drag_react {
            body = [0.45, 0.70, 1.0];
        }
        PetPaint {
            body_rgb: body,
            eye_rgb: [0.98, 0.98, 0.98],
            bounce,
            pupil_offset: [0.0, 0.0],
            draw_eyes: true,
            eye_open: 1.0,
            bubble_text: None,
            bubble_style: Default::default(),
        }
    }
}
