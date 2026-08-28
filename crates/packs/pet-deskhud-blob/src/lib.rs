//! Blue Dot community Guest implementation.

use std::sync::atomic::{AtomicBool, Ordering};

wit_bindgen::generate!({
    path: "../../core/deskhud-sdk/wit",
    world: "pet-guest",
});

use exports::deskhud::guest::pet_api::{self, Guest};

static HOVER_PULSE: AtomicBool = AtomicBool::new(true);
static DOCK_TINT: AtomicBool = AtomicBool::new(true);
static DRAG_REACT: AtomicBool = AtomicBool::new(true);

struct BlueDotGuest;

impl Guest for BlueDotGuest {
    fn info() -> pet_api::PetInfo {
        pet_api::PetInfo {
            id: "pet.deskhud.blob".into(),
            display_name: "Blue Dot".into(),
            description: "Simple blob; reacts to drag and docking".into(),
            author: "DeskHud".into(),
            homepage: Some("https://github.com/ko-eika/deskhud".into()),
            version: "0.7.0".into(),
            engine: "0.7".into(),
            window_width: 96.0,
            window_height: 96.0,
            config_options: vec![
                pet_api::ConfigOption {
                    key: "hover_pulse".into(),
                    label: "悬停轻弹".into(),
                    description: "指针停在宠物上时略微放大".into(),
                    default: true,
                },
                pet_api::ConfigOption {
                    key: "dock_tint".into(),
                    label: "贴边变色".into(),
                    description: "贴近屏幕边缘时改变颜色".into(),
                    default: true,
                },
                pet_api::ConfigOption {
                    key: "drag_react".into(),
                    label: "拖拽反馈".into(),
                    description: "拖拽宠物时改变颜色和大小".into(),
                    default: true,
                },
            ],
        }
    }

    fn apply_config(config: Vec<pet_api::ConfigEntry>) {
        for entry in config {
            match entry.key.as_str() {
                "hover_pulse" => HOVER_PULSE.store(entry.value, Ordering::Relaxed),
                "dock_tint" => DOCK_TINT.store(entry.value, Ordering::Relaxed),
                "drag_react" => DRAG_REACT.store(entry.value, Ordering::Relaxed),
                _ => {}
            }
        }
    }

    fn tick(_dt_secs: f32) {}

    fn on_event(_event: pet_api::Event) {}

    fn render(ctx: pet_api::PaintContext) -> pet_api::Scene {
        let mut color = pet_api::Color {
            r: 0.25,
            g: 0.55,
            b: 0.95,
            a: 1.0,
        };
        if DOCK_TINT.load(Ordering::Relaxed) {
            color = dock_color(ctx.dock);
        }
        let dragging = ctx.drag.active && DRAG_REACT.load(Ordering::Relaxed);
        if dragging {
            color = pet_api::Color {
                r: 0.45,
                g: 0.70,
                b: 1.0,
                a: 1.0,
            };
        }
        let radius = if dragging || (ctx.mouse.hovering && HOVER_PULSE.load(Ordering::Relaxed)) {
            1.06
        } else {
            1.0
        };
        pet_api::Scene {
            items: vec![
                pet_api::Item {
                    transform: pet_api::Transform {
                        translation: (0.0, 0.0),
                        rotation_radians: 0.0,
                        scale: (radius, radius),
                    },
                    z_index: 0,
                    node: pet_api::Node::Shape((pet_api::Shape::Circle(1.0), color)),
                },
                pet_api::Item {
                    transform: pet_api::Transform {
                        translation: (0.0, 0.0),
                        rotation_radians: 0.0,
                        scale: (1.0, 1.0),
                    },
                    z_index: -1,
                    node: pet_api::Node::HitRegion(pet_api::Shape::Circle(1.0)),
                },
            ],
        }
    }
}

fn dock_color(dock: pet_api::DockState) -> pet_api::Color {
    let (r, g, b) = match (dock.left, dock.right, dock.top, dock.bottom) {
        (true, false, true, false) => (1.0, 0.78, 0.05),
        (false, true, true, false) => (1.0, 0.18, 0.52),
        (true, false, false, true) => (0.05, 0.88, 0.72),
        (false, true, false, true) => (0.68, 0.12, 1.0),
        (_, _, true, false) => (1.0, 0.48, 0.04),
        (_, _, false, true) => (0.05, 0.82, 0.32),
        (true, false, false, false) => (0.95, 0.08, 0.18),
        (false, true, false, false) => (0.82, 0.08, 0.72),
        _ => (0.25, 0.55, 0.95),
    };
    pet_api::Color { r, g, b, a: 1.0 }
}

export!(BlueDotGuest);
