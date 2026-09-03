//! 外置 WASM 宠物：小汤圆（Little Dumpling）。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

wit_bindgen::generate!({
    path: "../../core/deskhud-sdk/wit",
    world: "pet-guest",
});

use exports::deskhud::guest::pet_api::{self, Guest};

struct BigEyesGuest;

static FOLLOW_EYES: AtomicBool = AtomicBool::new(true);
static CUSTOM_BUBBLE: AtomicBool = AtomicBool::new(false);
static KEY_TIPS: AtomicBool = AtomicBool::new(true);
static MOUSE_TIPS: AtomicBool = AtomicBool::new(true);
static HOVER_HIGHLIGHT: AtomicBool = AtomicBool::new(true);
static DOCK_TINT: AtomicBool = AtomicBool::new(true);
static DRAG_TINT: AtomicBool = AtomicBool::new(true);
static BUBBLE_MS: AtomicU32 = AtomicU32::new(0);
static CLICK_MS: AtomicU32 = AtomicU32::new(0);
static IDLE_MS: AtomicU32 = AtomicU32::new(0);
static BUBBLE_TEXT: Mutex<String> = Mutex::new(String::new());
static LAST_POINTER: Mutex<(i8, i8)> = Mutex::new((0, 0));
static EYE_MOTION: Mutex<EyeMotion> = Mutex::new(EyeMotion {
    current: (0.0, 0.0),
    last_time_secs: 0.0,
});

struct EyeMotion {
    current: (f32, f32),
    last_time_secs: f32,
}

impl Guest for BigEyesGuest {
    fn info() -> pet_api::PetInfo {
        pet_api::PetInfo {
            id: "pet.deskhud.dumpling".into(),
            display_name: "display_name".into(),
            description: "description".into(),
            author: "DeskHud".into(),
            homepage: Some("https://github.com/ko-eika/deskhud".into()),
            version: "0.9.4".into(),
            engine: "0.9".into(),
            window_width: 192.0,
            window_height: 192.0,
            config_options: [
                ("custom_bubble", false),
                ("follow_eyes", true),
                ("hover_highlight", true),
                ("drag_tint", true),
                ("dock_tint", true),
                ("key_tips", true),
                ("mouse_tips", true),
            ]
            .into_iter()
            .map(|(key, default)| pet_api::ConfigOption {
                key: key.into(),
                label: format!("{key}.label"),
                description: format!("{key}.description"),
                default,
            })
            .collect(),
        }
    }

    fn apply_config(config: Vec<pet_api::ConfigEntry>) {
        for entry in config {
            match entry.key.as_str() {
                "follow_eyes" => FOLLOW_EYES.store(entry.value, Ordering::Relaxed),
                "custom_bubble" => CUSTOM_BUBBLE.store(entry.value, Ordering::Relaxed),
                "key_tips" => KEY_TIPS.store(entry.value, Ordering::Relaxed),
                "mouse_tips" => MOUSE_TIPS.store(entry.value, Ordering::Relaxed),
                "hover_highlight" => HOVER_HIGHLIGHT.store(entry.value, Ordering::Relaxed),
                "dock_tint" => DOCK_TINT.store(entry.value, Ordering::Relaxed),
                "drag_tint" => DRAG_TINT.store(entry.value, Ordering::Relaxed),
                _ => {}
            }
        }
    }

    fn tick(dt_secs: f32) {
        let elapsed = (dt_secs.max(0.0) * 1000.0) as u32;
        BUBBLE_MS.fetch_sub(
            elapsed.min(BUBBLE_MS.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
        CLICK_MS.fetch_sub(
            elapsed.min(CLICK_MS.load(Ordering::Relaxed)),
            Ordering::Relaxed,
        );
        let _ = IDLE_MS.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
            Some(value.saturating_add(elapsed).min(10_000))
        });
    }

    fn on_event(event: pet_api::Event) {
        use pet_api::{Event, MouseButton};
        match event {
            Event::MouseClicked((MouseButton::Primary, _))
            | Event::MousePressed((MouseButton::Primary, _))
            | Event::GlobalMousePressed((MouseButton::Primary, _)) => {
                CLICK_MS.store(420, Ordering::Relaxed);
                IDLE_MS.store(0, Ordering::Relaxed);
                if MOUSE_TIPS.load(Ordering::Relaxed) {
                    bubble("InputKeyPrimary", 1000);
                }
            }
            Event::MousePressed((button, _)) | Event::GlobalMousePressed((button, _))
                if MOUSE_TIPS.load(Ordering::Relaxed) =>
            {
                bubble(
                    match button {
                        MouseButton::Secondary => "InputKeySecondary",
                        MouseButton::Middle => "InputKeyMiddle",
                        MouseButton::Primary => "InputKeyPrimary",
                    },
                    1000,
                );
            }
            Event::MouseWheel((delta, _)) | Event::GlobalMouseWheel((delta, _))
                if MOUSE_TIPS.load(Ordering::Relaxed) =>
            {
                bubble(
                    if delta > 0 {
                        "InputKeyWheelUp"
                    } else {
                        "InputKeyWheelDown"
                    },
                    800,
                );
            }
            Event::KeyPressed((key, modifiers)) | Event::GlobalKeyPressed((key, modifiers))
                if KEY_TIPS.load(Ordering::Relaxed) =>
            {
                bubble(format_shortcut(key, modifiers), 1000);
            }
            Event::KeyCombinationPressed((key, modifiers)) if KEY_TIPS.load(Ordering::Relaxed) => {
                bubble(format_shortcut(key, modifiers), 1000);
            }
            _ => {}
        }
    }

    fn render(ctx: pet_api::PaintContext) -> pet_api::Scene {
        let click = CLICK_MS.load(Ordering::Relaxed) as f32 / 420.0;
        let pointer_sample = (
            (ctx.pointer_dir.0.clamp(-1.0, 1.0) * 8.0) as i8,
            (ctx.pointer_dir.1.clamp(-1.0, 1.0) * 8.0) as i8,
        );
        if let Ok(mut last) = LAST_POINTER.lock()
            && *last != pointer_sample
        {
            *last = pointer_sample;
            IDLE_MS.store(0, Ordering::Relaxed);
        }
        let idle = IDLE_MS.load(Ordering::Relaxed) >= 900;
        let eye_target = if FOLLOW_EYES.load(Ordering::Relaxed) && (!idle || click > 0.0) {
            (
                ctx.pointer_dir.0.clamp(-1.0, 1.0) * (0.06 + click * 0.025),
                ctx.pointer_dir.1.clamp(-1.0, 1.0) * (0.05 + click * 0.02),
            )
        } else {
            (0.0, 0.0)
        };
        let (px, py) = smooth_eye_motion(eye_target, ctx.time_secs as f32);
        let blink = if (ctx.time_secs % 4.7) < 0.16 {
            0.12
        } else {
            1.0
        };
        let mut body = dock_color(ctx.dock);
        if !DOCK_TINT.load(Ordering::Relaxed) {
            body = pet_api::Color {
                r: 1.0,
                g: 0.97,
                b: 0.91,
                a: 1.0,
            };
        }
        if HOVER_HIGHLIGHT.load(Ordering::Relaxed) && ctx.mouse.hovering && !ctx.drag.active {
            body = pet_api::Color {
                r: (body.r + 0.08).min(1.0),
                g: (body.g + 0.06).min(1.0),
                b: (body.b + 0.04).min(1.0),
                a: 1.0,
            };
        }
        if DRAG_TINT.load(Ordering::Relaxed) && ctx.drag.active {
            body = pet_api::Color {
                r: 1.0,
                g: 0.72,
                b: 0.52,
                a: 1.0,
            };
        }
        let mut items = Vec::new();
        if ctx.shadows {
            // Layered transparent black creates a soft falloff with the
            // current vector scene primitives, without a hard-edged shadow.
            for (scale, alpha) in [(1.28, 0.025), (1.14, 0.040), (0.98, 0.060)] {
                items.push(scene_item(
                    0.0,
                    1.18,
                    scale,
                    0.12,
                    -1,
                    pet_api::Node::Shape((
                        pet_api::Shape::Ellipse((1.0, 1.0)),
                        pet_api::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: alpha,
                        },
                    )),
                ));
            }
        }
        items.push(item(
            0,
            pet_api::Node::GradientPath((
                pet_api::Path {
                    points: body_points(),
                    closed: true,
                    fill: None,
                    stroke: Some(pet_api::Color {
                        r: 0.357,
                        g: 0.251,
                        b: 0.212,
                        a: 1.0,
                    }),
                    stroke_width: 8.96 / 160.0,
                },
                body,
                pet_api::Color {
                    r: 0.949,
                    g: 0.757,
                    b: 0.553,
                    a: 1.0,
                },
            )),
        ));
        items.push(item(
            -1,
            pet_api::Node::HitRegion(pet_api::Shape::Circle(1.0)),
        ));
        for x in [-0.25, 0.25] {
            let eye_x = x * 1.86;
            let pupil_center = (eye_x + px, -0.093 + py);
            items.push(scene_item(
                eye_x,
                -0.093,
                0.205,
                0.205 * blink,
                3,
                pet_api::Node::Shape((
                    pet_api::Shape::Circle(1.0),
                    pet_api::Color {
                        r: 0.439,
                        g: 0.267,
                        b: 0.231,
                        a: 1.0,
                    },
                )),
            ));
            items.push(scene_item(
                pupil_center.0,
                pupil_center.1,
                0.102,
                0.102 * blink,
                4,
                pet_api::Node::Shape((
                    pet_api::Shape::Circle(1.0),
                    pet_api::Color {
                        r: 0.075,
                        g: 0.040,
                        b: 0.038,
                        a: 1.0,
                    },
                )),
            ));
            // Keep the catchlight anchored to the eye rather than gluing it
            // to the moving pupil. A small amount of parallax keeps it inside
            // the eye while avoiding a sticker-like rigid lock.
            items.push(scene_item(
                eye_x - 0.034 + px * 0.45,
                -0.145 + py * 0.45,
                0.041,
                0.041 * blink,
                5,
                pet_api::Node::Shape((
                    pet_api::Shape::Circle(1.0),
                    pet_api::Color {
                        r: 1.0,
                        g: 1.0,
                        b: 1.0,
                        a: 0.67,
                    },
                )),
            ));
        }
        for x in [-0.38, 0.38] {
            items.push(scene_item(
                x * 1.96,
                0.279,
                0.186,
                0.112,
                3,
                pet_api::Node::Shape((
                    pet_api::Shape::Ellipse((1.0, 1.0)),
                    pet_api::Color {
                        r: 1.0,
                        g: 0.718,
                        b: 0.698,
                        a: 0.80,
                    },
                )),
            ));
        }
        items.push(scene_item(
            0.0,
            0.0,
            1.0,
            1.0,
            6,
            pet_api::Node::Path(pet_api::Path {
                points: mouth_points(),
                closed: false,
                fill: None,
                stroke: Some(pet_api::Color {
                    r: 0.357,
                    g: 0.251,
                    b: 0.212,
                    a: 1.0,
                }),
                stroke_width: 7.68 / 160.0,
            }),
        ));
        if BUBBLE_MS.load(Ordering::Relaxed) > 0 {
            let text = BUBBLE_TEXT
                .lock()
                .map(|text| text.clone())
                .unwrap_or_default();
            if !text.is_empty() {
                items.push(scene_item(
                    0.0,
                    -0.82,
                    1.0,
                    1.0,
                    10,
                    pet_api::Node::Bubble((
                        text,
                        if CUSTOM_BUBBLE.load(Ordering::Relaxed) {
                            pet_api::Color {
                                r: 0.357,
                                g: 0.251,
                                b: 0.212,
                                a: 1.0,
                            }
                        } else if ctx.theme_dark {
                            pet_api::Color {
                                r: 0.91,
                                g: 0.92,
                                b: 0.95,
                                a: 1.0,
                            }
                        } else {
                            pet_api::Color {
                                r: 0.08,
                                g: 0.08,
                                b: 0.1,
                                a: 1.0,
                            }
                        },
                        if CUSTOM_BUBBLE.load(Ordering::Relaxed) {
                            pet_api::Color {
                                r: 1.0,
                                g: 0.969,
                                b: 0.91,
                                a: 0.96,
                            }
                        } else if ctx.theme_dark {
                            pet_api::Color {
                                r: 0.12,
                                g: 0.13,
                                b: 0.16,
                                a: 0.94,
                            }
                        } else {
                            pet_api::Color {
                                r: 1.0,
                                g: 1.0,
                                b: 1.0,
                                a: 0.94,
                            }
                        },
                        if CUSTOM_BUBBLE.load(Ordering::Relaxed) {
                            12.0
                        } else {
                            8.0
                        },
                    )),
                ));
            }
        }
        pet_api::Scene { items }
    }
}

fn bubble(text: impl Into<String>, duration: u32) {
    if let Ok(mut current) = BUBBLE_TEXT.lock() {
        *current = text.into();
    }
    BUBBLE_MS.store(duration, Ordering::Relaxed);
}

fn key_i18n_key(key: pet_api::KeyValue) -> String {
    match key {
        pet_api::KeyValue::Named(name) => format!("InputKey.{name:?}"),
        pet_api::KeyValue::Function(number) => format!("InputKey.F{number}"),
        pet_api::KeyValue::Letter(value) | pet_api::KeyValue::Digit(value) => value.to_string(),
        pet_api::KeyValue::Punct(value) => value.to_string(),
        pet_api::KeyValue::NumpadDigit(value) => format!("Num {value}"),
    }
}

fn format_shortcut(key: pet_api::KeyValue, modifiers: pet_api::Modifiers) -> String {
    let mut parts = Vec::new();
    if modifiers.ctrl {
        parts.push("InputKey.Ctrl".to_owned());
    }
    if modifiers.shift {
        parts.push("InputKey.Shift".to_owned());
    }
    if modifiers.alt {
        parts.push("InputKey.Alt".to_owned());
    }
    if modifiers.meta {
        parts.push("InputKey.Super".to_owned());
    }
    let key_label = key_i18n_key(key);
    let modifier_key = matches!(
        key_label.as_str(),
        "InputKey.Ctrl" | "InputKey.Shift" | "InputKey.Alt" | "InputKey.Super"
    );
    if !modifier_key || parts.is_empty() {
        parts.push(key_label);
    }
    parts.join(" + ")
}

fn dock_color(dock: pet_api::DockState) -> pet_api::Color {
    let (r, g, b) = if dock.left {
        (1.0, 0.91, 0.84)
    } else if dock.right {
        (0.98, 0.88, 0.91)
    } else if dock.top {
        (1.0, 0.91, 0.78)
    } else if dock.bottom {
        (0.88, 0.96, 0.84)
    } else {
        (1.0, 0.97, 0.91)
    };
    pet_api::Color { r, g, b, a: 1.0 }
}

/// Samples the preview SVG's cubic outline into a smooth, platform-neutral
/// polygon. The scene ABI carries points rather than Bézier commands, so a
/// dense sample keeps the runtime edge rounded at the pet's small display size.
fn body_points() -> Vec<(f32, f32)> {
    const SCALE: f32 = 53.76;
    let p0 = (0.0, -60.0);
    let segments = [
        (p0, (45.0, -60.0), (70.0, -30.0), (70.0, 10.0)),
        ((70.0, 10.0), (70.0, 45.0), (50.0, 60.0), (0.0, 60.0)),
        ((0.0, 60.0), (-50.0, 60.0), (-70.0, 45.0), (-70.0, 10.0)),
        ((-70.0, 10.0), (-70.0, -30.0), (-45.0, -60.0), p0),
    ];
    let mut points = Vec::with_capacity(65);
    for (segment_index, (a, b, c, d)) in segments.into_iter().enumerate() {
        for step in 0..16 {
            let t = step as f32 / 16.0;
            let u = 1.0 - t;
            points.push((
                (u * u * u * a.0 + 3.0 * u * u * t * b.0 + 3.0 * u * t * t * c.0 + t * t * t * d.0)
                    / SCALE,
                (u * u * u * a.1 + 3.0 * u * u * t * b.1 + 3.0 * u * t * t * c.1 + t * t * t * d.1)
                    / SCALE,
            ));
        }
        if segment_index == segments.len() - 1 {
            points.push((d.0 / SCALE, d.1 / SCALE));
        }
    }
    points
}

/// Samples the preview SVG's quadratic smile so the mouth keeps a soft arc
/// instead of becoming a sharp V made from three line vertices.
fn mouth_points() -> Vec<(f32, f32)> {
    let start = (-0.223, 0.372);
    let control = (0.0, 0.614);
    let end = (0.223, 0.372);
    (0..=16)
        .map(|step| {
            let t = step as f32 / 16.0;
            let u = 1.0 - t;
            (
                u * u * start.0 + 2.0 * u * t * control.0 + t * t * end.0,
                u * u * start.1 + 2.0 * u * t * control.1 + t * t * end.1,
            )
        })
        .collect()
}

/// Smooths the pointer target so the pupil eases into position instead of
/// jumping once per frame. The highlight intentionally stays out of this
/// motion and is anchored to the eye itself in `render`.
fn smooth_eye_motion(target: (f32, f32), time_secs: f32) -> (f32, f32) {
    let Ok(mut motion) = EYE_MOTION.lock() else {
        return target;
    };
    let dt = (time_secs - motion.last_time_secs).clamp(0.0, 0.1);
    motion.last_time_secs = time_secs;
    let blend = 1.0 - (-dt * 14.0).exp();
    motion.current.0 += (target.0 - motion.current.0) * blend;
    motion.current.1 += (target.1 - motion.current.1) * blend;
    motion.current
}

type SceneItem = pet_api::Item;

fn item(z: i32, node: pet_api::Node) -> SceneItem {
    pet_api::Item {
        transform: pet_api::Transform {
            translation: (0.0, 0.0),
            rotation_radians: 0.0,
            scale: (1.0, 1.0),
        },
        z_index: z,
        node,
    }
}

fn scene_item(x: f32, y: f32, sx: f32, sy: f32, z: i32, node: pet_api::Node) -> SceneItem {
    pet_api::Item {
        transform: pet_api::Transform {
            translation: (x, y),
            rotation_radians: 0.0,
            scale: (sx, sy),
        },
        z_index: z,
        node,
    }
}

export!(BigEyesGuest);
