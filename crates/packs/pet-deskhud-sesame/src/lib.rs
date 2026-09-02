//! 内置：芝麻豆；轻盈、好奇的吉祥物宠物。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

use deskhud_engine::{
    DockState, Path, PetBubbleStyle, PetConfigBag, PetConfigOption, PetEvent, PetKey, PetKind,
    PetKindInfo, PetModifiers, PetPaint, PetPaintCtx, PetScene, PetTheme, SceneItem, SceneNode,
    Shape, Transform2D,
};

/// 芝麻豆 `pet.deskhud.sesame`。
#[derive(Debug, Default)]
pub struct BuiltinSesamePet {
    follow_eyes: AtomicBool,
    custom_bubble: AtomicBool,
    hover_highlight: AtomicBool,
    dock_tint: AtomicBool,
    drag_tint: AtomicBool,
    key_tips: AtomicBool,
    mouse_tips: AtomicBool,
    bubble_ms: AtomicU32,
    dock_anim_ms: AtomicU32,
    bubble_text: Mutex<String>,
    last_pointer: Mutex<[i8; 2]>,
    idle_ms: AtomicU32,
    eye_motion: Mutex<EyeMotion>,
}

#[derive(Debug, Default)]
struct EyeMotion {
    current: [f32; 2],
    last_time_secs: f64,
}

const BOW_STROKE_WIDTH: f32 = 3.0 / 160.0;
const SESAME_BUBBLE_BACKGROUND: [f32; 4] = [0.72, 0.23, 0.42, 0.96];
const SESAME_BUBBLE_TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const SESAME_BUBBLE_CORNER_RADIUS: f32 = 12.0;

const OPTIONS: &[PetConfigOption] = &[
    PetConfigOption {
        key: "custom_bubble",
        label: "custom_bubble.label",
        description: "custom_bubble.description",
        default: true,
    },
    PetConfigOption {
        key: "follow_eyes",
        label: "follow_eyes.label",
        description: "follow_eyes.description",
        default: true,
    },
    PetConfigOption {
        key: "hover_highlight",
        label: "hover_highlight.label",
        description: "hover_highlight.description",
        default: true,
    },
    PetConfigOption {
        key: "drag_tint",
        label: "drag_tint.label",
        description: "drag_tint.description",
        default: true,
    },
    PetConfigOption {
        key: "dock_tint",
        label: "dock_tint.label",
        description: "dock_tint.description",
        default: true,
    },
    PetConfigOption {
        key: "key_tips",
        label: "key_tips.label",
        description: "key_tips.description",
        default: true,
    },
    PetConfigOption {
        key: "mouse_tips",
        label: "mouse_tips.label",
        description: "mouse_tips.description",
        default: true,
    },
];

impl PetKind for BuiltinSesamePet {
    fn info(&self) -> PetKindInfo {
        PetKindInfo {
            id: "pet.deskhud.sesame",
            version: deskhud_engine::ENGINE_PRODUCT_VERSION,
            engine: deskhud_engine::ENGINE_COMPAT_FAMILY,
            display_name: "display_name",
            description: "description",
            author: "DeskHud",
            homepage: Some("https://github.com/ko-eika/deskhud"),
            window_width: 192.0,
            window_height: 192.0,
            preview: Some(include_bytes!("../assets/preview.svg")),
        }
    }

    fn config_options(&self) -> &'static [PetConfigOption] {
        OPTIONS
    }

    fn apply_config(&self, config: PetConfigBag<'_>) {
        self.custom_bubble
            .store(config.get("custom_bubble", true), Ordering::Relaxed);
        self.follow_eyes
            .store(config.get("follow_eyes", true), Ordering::Relaxed);
        self.hover_highlight
            .store(config.get("hover_highlight", true), Ordering::Relaxed);
        self.dock_tint
            .store(config.get("dock_tint", true), Ordering::Relaxed);
        self.drag_tint
            .store(config.get("drag_tint", true), Ordering::Relaxed);
        self.key_tips
            .store(config.get("key_tips", true), Ordering::Relaxed);
        self.mouse_tips
            .store(config.get("mouse_tips", true), Ordering::Relaxed);
    }

    fn paint(&self, ctx: PetPaintCtx<'_>) -> PetPaint {
        let mut body = [0.27, 0.63, 0.96];
        if self.hover_highlight.load(Ordering::Relaxed) && ctx.mouse.hovering {
            body = [0.35, 0.69, 1.0];
        }
        if self.drag_tint.load(Ordering::Relaxed) && ctx.drag.is_dragging() {
            body = [0.46, 0.76, 0.98];
        }
        PetPaint {
            body_rgb: body,
            bubble_text: if self.bubble_ms.load(Ordering::Relaxed) > 0 {
                self.bubble_text
                    .lock()
                    .ok()
                    .filter(|text| !text.is_empty())
                    .map(|text| text.clone())
            } else {
                None
            },
            bubble_style: if self.custom_bubble.load(Ordering::Relaxed) {
                PetBubbleStyle::Custom {
                    background_rgba: SESAME_BUBBLE_BACKGROUND,
                    text_rgba: SESAME_BUBBLE_TEXT,
                    corner_radius: SESAME_BUBBLE_CORNER_RADIUS,
                }
            } else {
                PetBubbleStyle::FollowTheme
            },
        }
    }

    fn on_event(&self, event: PetEvent) {
        match event {
            PetEvent::DockChanged { .. } => self.dock_anim_ms.store(0, Ordering::Relaxed),
            PetEvent::GlobalMousePressed { button, .. } | PetEvent::MousePressed { button, .. }
                if self.mouse_tips.load(Ordering::Relaxed) =>
            {
                self.show_bubble(button.i18n_key(), 1000);
            }
            PetEvent::MouseWheel { delta, .. } | PetEvent::GlobalMouseWheel { delta, .. }
                if self.mouse_tips.load(Ordering::Relaxed) && delta != 0 =>
            {
                self.show_bubble(
                    if delta > 0 {
                        "InputKeyWheelUp"
                    } else {
                        "InputKeyWheelDown"
                    },
                    800,
                );
            }
            PetEvent::GlobalKeyPressed { key, modifiers }
            | PetEvent::KeyPressed { key, modifiers }
                if self.key_tips.load(Ordering::Relaxed) =>
            {
                self.show_bubble(format_shortcut(modifiers, key), 1000)
            }
            PetEvent::KeyCombinationPressed { key, modifiers }
                if self.key_tips.load(Ordering::Relaxed) =>
            {
                self.show_bubble(format_shortcut(modifiers, key), 1000)
            }
            _ => {}
        }
    }

    fn tick(&self, dt_secs: f32) {
        let elapsed = (dt_secs.max(0.0) * 1000.0) as u32;
        self.dock_anim_ms.fetch_add(elapsed, Ordering::Relaxed);
        let _ = self
            .bubble_ms
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_sub(elapsed))
            });
        let _ = self
            .idle_ms
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |value| {
                Some(value.saturating_add(elapsed).min(10_000))
            });
    }

    fn scene(&self, ctx: PetPaintCtx<'_>) -> PetScene {
        let paint = self.paint(ctx);
        let pointer_sample = [
            (ctx.pointer_dir[0].clamp(-1.0, 1.0) * 8.0) as i8,
            (ctx.pointer_dir[1].clamp(-1.0, 1.0) * 8.0) as i8,
        ];
        let pointer_changed = self
            .last_pointer
            .lock()
            .map(|mut last| {
                let changed = *last != pointer_sample;
                *last = pointer_sample;
                changed
            })
            .unwrap_or(false);
        if pointer_changed {
            self.idle_ms.store(0, Ordering::Relaxed);
        }
        let idle = self.idle_ms.load(Ordering::Relaxed) >= 900;
        let target = if self.follow_eyes.load(Ordering::Relaxed) && !idle {
            [
                ctx.pointer_dir[0],
                if ctx.dock.top {
                    -ctx.pointer_dir[1]
                } else {
                    ctx.pointer_dir[1]
                },
            ]
        } else {
            [0.0, 0.0]
        };
        let pointer = self.smooth_eye_target(target, ctx.time_secs);
        let blink = sesame_blink_open(ctx.time_secs);
        let dock_enabled = self.dock_tint.load(Ordering::Relaxed);
        let docked = ctx.dock.left || ctx.dock.right || ctx.dock.top || ctx.dock.bottom;
        let sway = if docked && !ctx.drag.is_dragging() {
            0.0
        } else {
            (ctx.time_secs as f32 * 2.2).sin() * 0.018
        };
        let tuck_progress = if docked && !ctx.drag.is_dragging() {
            let elapsed = self.dock_anim_ms.load(Ordering::Relaxed);
            (elapsed.saturating_sub(140) as f32 / 560.0).clamp(0.0, 1.0)
        } else {
            0.0
        };
        let (dock_x, dock_y) = if dock_enabled {
            dock_pose_offset(
                ctx.dock,
                ctx.time_secs,
                tuck_progress,
                ctx.mouse.hovering,
                ctx.drag.is_dragging(),
            )
        } else {
            (0.0, 0.0)
        };
        let (hit_x, hit_y) = if dock_enabled {
            dock_pose_offset(ctx.dock, ctx.time_secs, 1.0, false, ctx.drag.is_dragging())
        } else {
            (0.0, 0.0)
        };
        let drag_scale = drag_squash_scale(
            ctx.time_secs,
            ctx.drag.is_dragging(),
            self.drag_tint.load(Ordering::Relaxed),
        );
        let pet_y = dock_y + sway;
        let mut items = Vec::new();
        if ctx.shadows && (!docked || ctx.drag.is_dragging()) {
            for (scale, alpha) in [(1.22, 0.025), (1.08, 0.04), (0.92, 0.06)] {
                items.push(SceneItem {
                    transform: Transform2D {
                        translation: [0.0, svg_y(442.0)],
                        scale: [svg_scale(152.0 * scale), svg_scale(24.0 * scale)],
                        ..Transform2D::default()
                    },
                    z_index: -4,
                    node: SceneNode::Shape {
                        shape: Shape::Ellipse { radii: [1.0, 1.0] },
                        color: [0.0, 0.0, 0.0, alpha],
                    },
                });
            }
        }
        let pet_start = items.len();
        items.extend([
            sesame_bow_left(),
            sesame_bow_left_inner(),
            sesame_bow_left_fold(),
            sesame_bow_right(),
            sesame_bow_right_inner(),
            sesame_bow_right_fold(),
            sesame_bow_knot(),
        ]);
        for item in &mut items[pet_start..] {
            translate_scene_item(item, dock_x, pet_y);
        }
        items.extend([
            SceneItem {
                transform: Transform2D::default(),
                z_index: 0,
                node: SceneNode::GradientPath {
                    path: sesame_body_path(dock_x, pet_y, [1.0, 1.0]),
                    top_color: [1.0, 0.824, 0.878, 1.0],
                    bottom_color: [1.0, 0.941, 0.961, 1.0],
                },
            },
            SceneItem {
                transform: Transform2D {
                    translation: [dock_x, pet_y],
                    scale: [1.0, 1.0],
                    ..Transform2D::default()
                },
                z_index: -1,
                node: SceneNode::HitRegion {
                    shape: Shape::Ellipse {
                        radii: [0.92, 0.96],
                    },
                },
            },
        ]);
        let neutral_pupil = [9.0, 6.0];
        let pupil_offset = bounded_eye_offset(pointer, neutral_pupil, [23.0, 29.0]);
        let pupil_motion = [
            pupil_offset[0] - neutral_pupil[0],
            pupil_offset[1] - neutral_pupil[1],
        ];
        for x in [186.0, 326.0] {
            items.push(SceneItem {
                transform: Transform2D {
                    translation: [svg_x(x) + dock_x, svg_y(250.0) + pet_y],
                    scale: [svg_scale(49.0), svg_scale(55.0) * blink.max(0.08)],
                    ..Transform2D::default()
                },
                z_index: 1,
                node: SceneNode::Shape {
                    shape: Shape::Ellipse { radii: [1.0, 1.0] },
                    color: [1.0, 0.97, 0.95, 1.0],
                },
            });
            if blink > 0.08 {
                items.push(SceneItem {
                    transform: Transform2D {
                        translation: [
                            svg_x(x + pupil_offset[0]) + dock_x,
                            svg_y(250.0 + pupil_offset[1]) + pet_y,
                        ],
                        scale: [svg_scale(21.0), svg_scale(21.0) * blink],
                        ..Transform2D::default()
                    },
                    z_index: 2,
                    node: SceneNode::Shape {
                        shape: Shape::Circle { radius: 1.0 },
                        color: [0.72, 0.23, 0.42, 1.0],
                    },
                });
                items.push(SceneItem {
                    transform: Transform2D {
                        translation: [
                            svg_x(x + 1.0 + pupil_motion[0]) + dock_x,
                            svg_y(248.0 + pupil_motion[1]) + pet_y,
                        ],
                        scale: [svg_scale(6.5), svg_scale(6.5) * blink],
                        ..Transform2D::default()
                    },
                    z_index: 3,
                    node: SceneNode::Shape {
                        shape: Shape::Circle { radius: 1.0 },
                        color: [1.0, 1.0, 1.0, 0.8],
                    },
                });
            }
        }
        items.extend([
            SceneItem {
                transform: Transform2D {
                    translation: [svg_x(146.0) + dock_x, svg_y(314.0) + pet_y],
                    scale: [svg_scale(15.0), svg_scale(15.0)],
                    ..Transform2D::default()
                },
                z_index: 2,
                node: SceneNode::Shape {
                    shape: Shape::Circle { radius: 1.0 },
                    color: [1.0, 0.60, 0.63, 0.45],
                },
            },
            SceneItem {
                transform: Transform2D {
                    translation: [svg_x(366.0) + dock_x, svg_y(314.0) + pet_y],
                    scale: [svg_scale(15.0), svg_scale(15.0)],
                    ..Transform2D::default()
                },
                z_index: 2,
                node: SceneNode::Shape {
                    shape: Shape::Circle { radius: 1.0 },
                    color: [1.0, 0.60, 0.63, 0.45],
                },
            },
            SceneItem {
                transform: Transform2D::default(),
                z_index: 3,
                node: SceneNode::Path(Path {
                    points: (0..=12)
                        .map(|step| {
                            let t = step as f32 / 12.0;
                            let u = 1.0 - t;
                            let [x, y] = svg_point(
                                u * u * 241.0 + 2.0 * u * t * 256.0 + t * t * 271.0,
                                u * u * 329.0 + 2.0 * u * t * 347.0 + t * t * 329.0,
                            );
                            [x + dock_x, y + pet_y]
                        })
                        .collect(),
                    closed: false,
                    fill: None,
                    stroke: Some([0.72, 0.23, 0.42, 1.0]),
                    stroke_width: 7.0 / 160.0,
                }),
            },
        ]);
        for item in &mut items[pet_start..] {
            if ctx.dock.top
                && !ctx.drag.is_dragging()
                && !matches!(item.node, SceneNode::HitRegion { .. })
            {
                // A pet coming down from the top hangs head-first.
                scale_scene_item(item, [0.0, 0.0], [1.0, -1.0]);
                // The scene already contains the docking translation. Undo
                // the translation's flip so only the pet pose is inverted.
                translate_scene_item(item, 0.0, pet_y * 2.0);
            }
            if matches!(item.node, SceneNode::HitRegion { .. }) {
                item.transform.translation = [hit_x, hit_y + sway];
            } else {
                scale_scene_item(item, [dock_x, pet_y], drag_scale);
            }
            if !ctx.drag.is_dragging() && !matches!(item.node, SceneNode::HitRegion { .. }) {
                rotate_scene_item(item, dock_tilt(ctx.dock), [dock_x, pet_y]);
            }
        }
        if let Some(text) = paint.bubble_text {
            let (color, background, corner_radius) = match paint.bubble_style {
                PetBubbleStyle::FollowTheme => match ctx.theme {
                    PetTheme::Light => ([0.08, 0.08, 0.1, 1.0], [1.0, 1.0, 1.0, 0.94], 8.0),
                    PetTheme::Dark => ([0.91, 0.92, 0.95, 1.0], [0.12, 0.13, 0.16, 0.94], 8.0),
                },
                PetBubbleStyle::Custom {
                    background_rgba,
                    text_rgba,
                    corner_radius,
                } => (text_rgba, background_rgba, corner_radius),
            };
            items.push(SceneItem {
                transform: Transform2D {
                    translation: [0.0, -0.82],
                    ..Transform2D::default()
                },
                z_index: 10,
                node: SceneNode::Bubble {
                    text,
                    color,
                    background,
                    corner_radius,
                },
            });
        }
        PetScene { items }
    }
}

fn format_shortcut(modifiers: PetModifiers, key: PetKey) -> String {
    let mut parts = Vec::new();
    if modifiers.ctrl {
        parts.push(PetKey::Ctrl.i18n_key());
    }
    if modifiers.shift {
        parts.push(PetKey::Shift.i18n_key());
    }
    if modifiers.alt {
        parts.push(PetKey::Alt.i18n_key());
    }
    if modifiers.meta {
        parts.push(PetKey::Super.i18n_key());
    }
    if !is_modifier_key(key) || parts.is_empty() {
        parts.push(key.i18n_key());
    }
    parts.join(" + ")
}

fn is_modifier_key(key: PetKey) -> bool {
    matches!(
        key,
        PetKey::Shift | PetKey::Ctrl | PetKey::Alt | PetKey::Super
    )
}

impl BuiltinSesamePet {
    fn smooth_eye_target(&self, target: [f32; 2], time_secs: f64) -> [f32; 2] {
        let Ok(mut motion) = self.eye_motion.lock() else {
            return target;
        };
        let dt = (time_secs - motion.last_time_secs).clamp(0.0, 0.1) as f32;
        motion.last_time_secs = time_secs;
        let alpha = 1.0 - (-12.0 * dt).exp();
        for (current, target) in motion.current.iter_mut().zip(target) {
            *current += (target - *current) * alpha;
        }
        motion.current
    }
}

fn svg_x(x: f32) -> f32 {
    (x - 256.0) / 160.0
}
fn svg_y(y: f32) -> f32 {
    (y - 270.0) / 160.0
}
fn svg_scale(value: f32) -> f32 {
    value / 160.0
}
fn svg_point(x: f32, y: f32) -> [f32; 2] {
    [svg_x(x), svg_y(y)]
}

fn bounded_eye_offset(direction: [f32; 2], neutral: [f32; 2], limits: [f32; 2]) -> [f32; 2] {
    let axis_target = |direction: f32, neutral: f32, limit: f32| {
        let direction = direction.clamp(-1.0, 1.0);
        if direction >= 0.0 {
            neutral + direction * (limit - neutral)
        } else {
            neutral + direction * (limit + neutral)
        }
    };
    let mut offset = [
        axis_target(direction[0], neutral[0], limits[0]),
        axis_target(direction[1], neutral[1], limits[1]),
    ];
    let ellipse = (offset[0] / limits[0]).powi(2) + (offset[1] / limits[1]).powi(2);
    if ellipse > 1.0 {
        let scale = ellipse.sqrt().recip();
        offset[0] *= scale;
        offset[1] *= scale;
    }
    offset
}

fn sesame_blink_open(time_secs: f64) -> f32 {
    let phase = (time_secs as f32).rem_euclid(4.8);
    if phase < 4.52 {
        1.0
    } else if phase < 4.62 {
        1.0 - smooth_step((phase - 4.52) / 0.10)
    } else if phase < 4.70 {
        0.0
    } else {
        smooth_step((phase - 4.70) / 0.10)
    }
}

fn smooth_step(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

fn drag_squash_scale(time_secs: f64, dragging: bool, enabled: bool) -> [f32; 2] {
    if !dragging || !enabled {
        return [1.0, 1.0];
    }
    let pulse = (time_secs as f32 * 9.0).sin();
    [1.0 + pulse * 0.07, 1.0 - pulse * 0.07]
}

/// Gives each edge a distinct, restrained pose without changing the window.
/// Makes a docked pet peek toward and retreat from the corresponding edge.
fn dock_pose_offset(
    dock: DockState,
    time_secs: f64,
    tuck_progress: f32,
    hovering: bool,
    dragging: bool,
) -> (f32, f32) {
    if dragging {
        return (0.0, 0.0);
    }
    let motion = if dock.left || dock.right || dock.top || dock.bottom {
        0.0
    } else {
        (time_secs as f32 * 4.0).sin() * 0.014
    };
    let corner = (dock.left || dock.right) && (dock.top || dock.bottom);
    let horizontal_only = (dock.left || dock.right) && !corner;
    let settled_distance: f32 = if corner {
        1.05
    } else if horizontal_only {
        1.65
    } else {
        1.45
    };
    let distance = if hovering {
        // Hovering a docked pet reveals only a little more of it.
        (settled_distance - 0.25).max(0.75)
    } else {
        // Leave the window first, then settle into one rigid half-visible pose.
        2.7 - tuck_progress * (2.7 - settled_distance)
    };
    let x = if dock.left {
        -distance - motion
    } else if dock.right {
        distance + motion
    } else {
        0.0
    };
    let y = if dock.top {
        -distance - motion
    } else if dock.bottom {
        distance + motion
    } else {
        0.0
    };
    (x, y)
}

fn scale_scene_item(item: &mut SceneItem, pivot: [f32; 2], scale: [f32; 2]) {
    match &mut item.node {
        SceneNode::Path(path) | SceneNode::GradientPath { path, .. } => {
            for point in &mut path.points {
                point[0] = pivot[0] + (point[0] - pivot[0]) * scale[0];
                point[1] = pivot[1] + (point[1] - pivot[1]) * scale[1];
            }
        }
        _ => {
            item.transform.translation[0] =
                pivot[0] + (item.transform.translation[0] - pivot[0]) * scale[0];
            item.transform.translation[1] =
                pivot[1] + (item.transform.translation[1] - pivot[1]) * scale[1];
            item.transform.scale[0] *= scale[0];
            item.transform.scale[1] *= scale[1];
        }
    }
}

fn rotate_scene_item(item: &mut SceneItem, angle: f32, pivot: [f32; 2]) {
    if angle == 0.0 {
        return;
    }
    let (sin, cos) = angle.sin_cos();
    let rotate = |point: &mut [f32; 2]| {
        let [x, y] = [point[0] - pivot[0], point[1] - pivot[1]];
        *point = [pivot[0] + x * cos - y * sin, pivot[1] + x * sin + y * cos];
    };
    match &mut item.node {
        SceneNode::Path(path) | SceneNode::GradientPath { path, .. } => {
            for point in &mut path.points {
                rotate(point);
            }
        }
        _ => rotate(&mut item.transform.translation),
    }
}

fn dock_tilt(dock: DockState) -> f32 {
    let horizontal = if dock.left {
        1.0
    } else if dock.right {
        -1.0
    } else {
        0.0
    };
    let horizontal = if dock.top { -horizontal } else { horizontal };
    if horizontal == 0.0 {
        0.0
    } else if dock.top || dock.bottom {
        horizontal * 0.785
    } else {
        horizontal * 0.524
    }
}

fn translate_scene_item(item: &mut SceneItem, x: f32, y: f32) {
    match &mut item.node {
        SceneNode::Path(path) | SceneNode::GradientPath { path, .. } => {
            for point in &mut path.points {
                point[0] += x;
                point[1] += y;
            }
        }
        _ => {
            item.transform.translation[0] += x;
            item.transform.translation[1] += y;
        }
    }
}

fn sesame_body_path(dock_x: f32, dock_y: f32, drag_scale: [f32; 2]) -> Path {
    let mut points = Vec::with_capacity(128);
    let segments = [
        (
            [110.0, 308.0],
            [107.0, 235.0],
            [131.0, 143.0],
            [229.0, 125.0],
        ),
        (
            [229.0, 125.0],
            [241.0, 122.0],
            [250.0, 122.0],
            [256.0, 122.0],
        ),
        (
            [256.0, 122.0],
            [262.0, 122.0],
            [271.0, 122.0],
            [283.0, 125.0],
        ),
        (
            [283.0, 125.0],
            [381.0, 143.0],
            [405.0, 235.0],
            [402.0, 308.0],
        ),
        (
            [402.0, 308.0],
            [399.0, 338.0],
            [384.0, 363.0],
            [366.0, 372.0],
        ),
        (
            [366.0, 372.0],
            [357.0, 378.0],
            [347.0, 375.0],
            [338.0, 372.0],
        ),
        (
            [338.0, 372.0],
            [311.0, 424.0],
            [286.0, 421.0],
            [256.0, 381.0],
        ),
        (
            [256.0, 381.0],
            [226.0, 421.0],
            [201.0, 424.0],
            [174.0, 372.0],
        ),
        (
            [174.0, 372.0],
            [165.0, 378.0],
            [155.0, 378.0],
            [146.0, 375.0],
        ),
        (
            [146.0, 375.0],
            [129.0, 369.0],
            [113.0, 344.0],
            [110.0, 308.0],
        ),
    ];
    for (index, (a, b, c, d)) in segments.into_iter().enumerate() {
        if index == 0 {
            let [x, y] = svg_point(a[0], a[1]);
            points.push([x * drag_scale[0] + dock_x, y * drag_scale[1] + dock_y]);
        }
        for step in 1..=16 {
            let t = step as f32 / 16.0;
            let u = 1.0 - t;
            let x = u * u * u * a[0]
                + 3.0 * u * u * t * b[0]
                + 3.0 * u * t * t * c[0]
                + t * t * t * d[0];
            let y = u * u * u * a[1]
                + 3.0 * u * u * t * b[1]
                + 3.0 * u * t * t * c[1]
                + t * t * t * d[1];
            let [x, y] = svg_point(x, y);
            points.push([x * drag_scale[0] + dock_x, y * drag_scale[1] + dock_y]);
        }
    }
    Path {
        points,
        closed: true,
        fill: None,
        stroke: Some([0.95, 0.60, 0.71, 1.0]),
        stroke_width: 9.0 / 160.0,
    }
}

fn sesame_bow_left() -> SceneItem {
    SceneItem {
        z_index: -3,
        transform: Transform2D::default(),
        node: SceneNode::GradientPath {
            path: reference_bow_path(
                [244.135, 105.9575],
                &[
                    cubic([244.135, 105.9575], [215.1525, 80.4175], [182.4375, 77.835]),
                    cubic([182.4375, 77.835], [153.605, 71.965], [157.1175, 111.235]),
                    cubic([157.5, 115.51], [157.2175, 119.81], [156.3875, 124.02]),
                    cubic(
                        [154.73, 132.4225],
                        [153.4025, 147.6275],
                        [161.9675, 158.565],
                    ),
                    cubic(
                        [174.4025, 174.4425],
                        [224.1725, 163.9225],
                        [249.9825, 131.4],
                    ),
                    cubic([249.985, 131.4], [253.4125, 115.905], [244.135, 105.9575]),
                ],
                None,
                Some([0.722, 0.361, 0.471, 1.0]),
                BOW_STROKE_WIDTH,
            ),
            top_color: [1.0, 0.761, 0.839, 1.0],
            bottom_color: [0.84, 0.36, 0.55, 1.0],
        },
    }
}

fn sesame_bow_left_inner() -> SceneItem {
    SceneItem {
        z_index: -2,
        transform: Transform2D::default(),
        node: SceneNode::Path(reference_bow_path(
            [207.5, 116.0],
            &[
                cubic(
                    [199.1775, 120.7825],
                    [206.4475, 126.7125],
                    [216.205, 125.0875],
                ),
                cubic(
                    [225.9625, 123.4625],
                    [243.18, 125.0875],
                    [243.7525, 125.7575],
                ),
                cubic(
                    [244.3275, 126.4275],
                    [242.3175, 120.4975],
                    [242.3175, 120.4975],
                ),
                cubic([242.3175, 120.4975], [215.8225, 111.2175], [207.5, 116.0]),
            ],
            Some([0.835, 0.435, 0.612, 1.0]),
            None,
            0.0,
        )),
    }
}

fn sesame_bow_left_fold() -> SceneItem {
    SceneItem {
        // Lower ribbon sits behind the bow loops.
        z_index: -4,
        transform: Transform2D::default(),
        node: SceneNode::GradientPath {
            path: reference_bow_path(
                [250.735, 136.7575],
                &[
                    cubic([250.735, 136.7575], [234.53, 184.7875], [220.8675, 209.815]),
                    cubic([218.3025, 214.515], [211.86, 215.2525], [208.335, 211.22]),
                    cubic([205.89, 208.42], [203.445, 205.625], [201.0, 202.8275]),
                    cubic([197.0, 198.4], [192.0, 196.0], [186.57, 195.3775]),
                    cubic([183.18, 195.035], [179.7875, 194.69], [176.395, 194.3475]),
                    cubic(
                        [170.1275, 193.7125],
                        [167.2475, 186.2175],
                        [171.485, 181.555],
                    ),
                    cubic(
                        [180.845, 171.2575],
                        [193.6425, 157.035],
                        [196.595, 153.0175],
                    ),
                    cubic([201.3775, 146.515], [236.96, 113.8], [250.735, 136.7575]),
                ],
                None,
                Some([0.722, 0.361, 0.471, 1.0]),
                BOW_STROKE_WIDTH,
            ),
            top_color: [1.0, 0.761, 0.839, 1.0],
            bottom_color: [0.84, 0.36, 0.55, 1.0],
        },
    }
}

fn sesame_bow_right() -> SceneItem {
    SceneItem {
        z_index: -3,
        transform: Transform2D::default(),
        node: SceneNode::GradientPath {
            path: reference_bow_path(
                [267.5325, 104.905],
                &[
                    cubic([267.5325, 104.905], [296.515, 79.365], [329.23, 76.7825]),
                    cubic([329.23, 76.7825], [358.0625, 70.9125], [354.55, 110.1825]),
                    cubic([354.1675, 114.4575], [354.45, 118.7575], [355.28, 122.9675]),
                    cubic([356.9375, 131.37], [358.265, 146.575], [349.7, 157.5125]),
                    cubic([337.265, 173.39], [287.495, 162.87], [261.685, 130.3475]),
                    cubic(
                        [261.6825, 130.3475],
                        [258.2525, 114.8525],
                        [267.5325, 104.905],
                    ),
                ],
                None,
                Some([0.722, 0.361, 0.471, 1.0]),
                BOW_STROKE_WIDTH,
            ),
            top_color: [1.0, 0.761, 0.839, 1.0],
            bottom_color: [0.84, 0.36, 0.55, 1.0],
        },
    }
}

fn sesame_bow_right_inner() -> SceneItem {
    SceneItem {
        z_index: -2,
        transform: Transform2D::default(),
        node: SceneNode::Path(reference_bow_path(
            [304.1675, 114.9475],
            &[
                cubic([312.49, 119.73], [305.22, 125.66], [295.4625, 124.035]),
                cubic([285.705, 122.41], [268.4875, 124.035], [267.915, 124.705]),
                cubic([267.34, 125.375], [269.35, 119.445], [269.35, 119.445]),
                cubic([269.35, 119.445], [295.845, 110.165], [304.1675, 114.9475]),
            ],
            Some([0.835, 0.435, 0.612, 1.0]),
            None,
            0.0,
        )),
    }
}

fn sesame_bow_right_fold() -> SceneItem {
    SceneItem {
        // Lower ribbon sits behind the bow loops.
        z_index: -4,
        transform: Transform2D::default(),
        node: SceneNode::GradientPath {
            path: reference_bow_path(
                [260.9325, 135.705],
                &[
                    cubic([260.9325, 135.705], [277.1375, 183.735], [290.8, 208.7625]),
                    cubic([293.365, 213.4625], [299.8075, 214.2], [303.3325, 210.1675]),
                    cubic(
                        [305.7775, 207.37],
                        [308.2225, 204.5725],
                        [310.6675, 201.775],
                    ),
                    cubic([314.6675, 197.35], [319.6675, 194.95], [325.0975, 194.325]),
                    cubic([328.49, 193.9825], [331.88, 193.6375], [335.2725, 193.295]),
                    cubic([341.54, 192.66], [344.42, 185.165], [340.1825, 180.5025]),
                    cubic(
                        [330.8225, 170.205],
                        [318.025, 155.9825],
                        [315.0725, 151.965],
                    ),
                    cubic([310.29, 145.46], [274.705, 112.7475], [260.9325, 135.705]),
                ],
                None,
                Some([0.722, 0.361, 0.471, 1.0]),
                BOW_STROKE_WIDTH,
            ),
            top_color: [1.0, 0.761, 0.839, 1.0],
            bottom_color: [0.84, 0.36, 0.55, 1.0],
        },
    }
}

fn sesame_bow_knot() -> SceneItem {
    SceneItem {
        transform: Transform2D::default(),
        z_index: -1,
        node: SceneNode::GradientPath {
            path: reference_bow_path(
                [255.04, 100.7925],
                &[
                    cubic([255.04, 100.7925], [238.97, 99.3575], [238.97, 112.27]),
                    cubic([238.97, 125.1825], [237.8225, 142.975], [255.04, 142.975]),
                    cubic([272.2575, 142.975], [273.405, 132.3575], [273.405, 119.445]),
                    cubic([273.405, 106.5325], [273.405, 100.2175], [255.04, 100.7925]),
                ],
                None,
                Some([0.72, 0.36, 0.47, 1.0]),
                BOW_STROKE_WIDTH,
            ),
            top_color: [1.0, 0.878, 0.918, 1.0],
            bottom_color: [0.91, 0.47, 0.64, 1.0],
        },
    }
}

fn cubic(control_a: [f32; 2], control_b: [f32; 2], end: [f32; 2]) -> [[f32; 2]; 3] {
    [control_a, control_b, end]
}

/// Samples the exact cubic geometry authored in the Sesame preview SVG.
fn reference_bow_path(
    start: [f32; 2],
    segments: &[[[f32; 2]; 3]],
    fill: Option<[f32; 4]>,
    stroke: Option<[f32; 4]>,
    stroke_width: f32,
) -> Path {
    let mut sampled = Vec::with_capacity(segments.len() * 16 + 1);
    let mut current = start;
    sampled.push(svg_point(start[0], start[1]));
    for [control_a, control_b, next] in segments {
        for step in 1..=16 {
            let t = step as f32 / 16.0;
            let u = 1.0 - t;
            let x = u * u * u * current[0]
                + 3.0 * u * u * t * control_a[0]
                + 3.0 * u * t * t * control_b[0]
                + t * t * t * next[0];
            let y = u * u * u * current[1]
                + 3.0 * u * u * t * control_a[1]
                + 3.0 * u * t * t * control_b[1]
                + t * t * t * next[1];
            sampled.push(svg_point(x, y));
        }
        current = *next;
    }
    Path {
        points: sampled,
        closed: true,
        fill,
        stroke,
        stroke_width,
    }
}

impl BuiltinSesamePet {
    fn show_bubble(&self, text: impl Into<String>, ms: u32) {
        if let Ok(mut current) = self.bubble_text.lock() {
            *current = text.into();
        }
        self.bubble_ms.store(ms, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use deskhud_engine::{
        DockState, DragState, MouseState, PetBubbleStyle, PetConfigBag, PetKind, PetPaintCtx,
        PetScene, PetTheme, SceneNode,
    };

    use super::{BuiltinSesamePet, bounded_eye_offset};

    fn scene_at(
        pet: &BuiltinSesamePet,
        config: &HashMap<String, bool>,
        time_secs: f64,
        pointer_dir: [f32; 2],
    ) -> PetScene {
        scene_at_with_drag(pet, config, time_secs, pointer_dir, DragState::IDLE)
    }

    fn scene_at_with_drag(
        pet: &BuiltinSesamePet,
        config: &HashMap<String, bool>,
        time_secs: f64,
        pointer_dir: [f32; 2],
        drag: DragState,
    ) -> PetScene {
        pet.scene(PetPaintCtx {
            time_secs,
            pointer_dir,
            status_line: "",
            dock: DockState::FREE,
            drag,
            mouse: MouseState::IDLE,
            config: PetConfigBag::new(config),
            theme: PetTheme::Light,
            shadows: true,
        })
    }

    fn scene_at_docked(
        pet: &BuiltinSesamePet,
        config: &HashMap<String, bool>,
        time_secs: f64,
        dock: DockState,
    ) -> PetScene {
        pet.scene(PetPaintCtx {
            time_secs,
            pointer_dir: [0.0, 0.0],
            status_line: "",
            dock,
            drag: DragState::IDLE,
            mouse: MouseState::IDLE,
            config: PetConfigBag::new(config),
            theme: PetTheme::Light,
            shadows: true,
        })
    }

    #[test]
    fn scene_contains_sesame_bow_and_vector_artwork() {
        let config = HashMap::new();
        let scene = scene_at(&BuiltinSesamePet::default(), &config, 0.0, [0.0, 0.0]);
        assert!(scene.validate().is_ok());
        assert!(
            scene
                .items
                .iter()
                .any(|item| matches!(item.node, SceneNode::GradientPath { .. }))
        );
        assert!(scene.items.len() >= 7);
        assert!(
            scene
                .items
                .iter()
                .any(|item| matches!(item.node, SceneNode::Path(_)))
        );
    }

    #[test]
    fn custom_bubble_uses_sesame_palette() {
        let config = HashMap::from([("custom_bubble".to_owned(), true)]);
        let pet = BuiltinSesamePet::default();
        pet.apply_config(PetConfigBag::new(&config));
        let paint = pet.paint(PetPaintCtx {
            time_secs: 0.0,
            pointer_dir: [0.0, 0.0],
            status_line: "",
            dock: DockState::FREE,
            drag: DragState::IDLE,
            mouse: MouseState::IDLE,
            config: PetConfigBag::new(&config),
            theme: PetTheme::Light,
            shadows: true,
        });
        assert_eq!(
            paint.bubble_style,
            PetBubbleStyle::Custom {
                background_rgba: [0.72, 0.23, 0.42, 0.96],
                text_rgba: [1.0, 1.0, 1.0, 1.0],
                corner_radius: 12.0,
            }
        );
    }

    #[test]
    fn pointer_follow_moves_pupils_but_keeps_eye_whites_fixed() {
        let config = HashMap::from([("follow_eyes".to_owned(), true)]);
        let centered_pet = BuiltinSesamePet::default();
        centered_pet.apply_config(PetConfigBag::new(&config));
        let shifted_pet = BuiltinSesamePet::default();
        shifted_pet.apply_config(PetConfigBag::new(&config));
        let centered = scene_at(&centered_pet, &config, 0.1, [0.0, 0.0]);
        let shifted = scene_at(&shifted_pet, &config, 0.1, [1.0, -1.0]);
        let positions = |scene: &PetScene, color: [f32; 4]| {
            scene
                .items
                .iter()
                .filter_map(|item| match &item.node {
                    SceneNode::Shape {
                        color: item_color, ..
                    } if *item_color == color => Some(item.transform.translation),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        assert_eq!(
            positions(&centered, [1.0, 0.97, 0.95, 1.0]),
            positions(&shifted, [1.0, 0.97, 0.95, 1.0])
        );
        assert_ne!(
            positions(&centered, [0.72, 0.23, 0.42, 1.0]),
            positions(&shifted, [0.72, 0.23, 0.42, 1.0])
        );
        let pupil_centered = positions(&centered, [0.72, 0.23, 0.42, 1.0])[0];
        let pupil_shifted = positions(&shifted, [0.72, 0.23, 0.42, 1.0])[0];
        let highlight_centered = positions(&centered, [1.0, 1.0, 1.0, 0.8])[0];
        let highlight_shifted = positions(&shifted, [1.0, 1.0, 1.0, 0.8])[0];
        for axis in 0..2 {
            let pupil_delta = pupil_shifted[axis] - pupil_centered[axis];
            let highlight_delta = highlight_shifted[axis] - highlight_centered[axis];
            assert!((highlight_delta - pupil_delta).abs() < 0.000_1);
        }
    }

    #[test]
    fn closed_eyes_hide_pupils_and_highlights() {
        let config = HashMap::new();
        let scene = scene_at(&BuiltinSesamePet::default(), &config, 4.66, [0.0, 0.0]);
        let shape_colors = scene.items.iter().filter_map(|item| match &item.node {
            SceneNode::Shape { color, .. } => Some(*color),
            _ => None,
        });
        let colors = shape_colors.collect::<Vec<_>>();

        assert_eq!(
            colors
                .iter()
                .filter(|color| **color == [1.0, 0.97, 0.95, 1.0])
                .count(),
            2
        );
        assert!(!colors.contains(&[0.72, 0.23, 0.42, 1.0]));
        assert!(!colors.contains(&[1.0, 1.0, 1.0, 0.8]));
    }

    #[test]
    fn pupil_offsets_stay_inside_sesame_eye_socket() {
        let limits = [23.0, 29.0];
        for direction in [[-1.0, 0.0], [1.0, 0.0], [0.0, -1.0], [0.0, 1.0], [1.0, 1.0]] {
            let offset = bounded_eye_offset(direction, [9.0, 6.0], limits);
            let ellipse = (offset[0] / limits[0]).powi(2) + (offset[1] / limits[1]).powi(2);
            assert!(ellipse <= 1.000_1);
        }
        assert_eq!(
            bounded_eye_offset([0.0, 0.0], [9.0, 6.0], limits),
            [9.0, 6.0]
        );
    }

    #[test]
    fn idle_returns_sesame_pupils_toward_center() {
        let config = HashMap::from([
            ("follow_eyes".to_owned(), true),
            ("drag_tint".to_owned(), true),
        ]);
        let pet = BuiltinSesamePet::default();
        pet.apply_config(PetConfigBag::new(&config));
        let moving = scene_at(&pet, &config, 0.1, [1.0, 0.0]);
        pet.idle_ms.store(900, std::sync::atomic::Ordering::Relaxed);
        let idle = scene_at(&pet, &config, 0.2, [1.0, 0.0]);
        let pupil_x = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find(|item| item.z_index == 2 && matches!(item.node, SceneNode::Shape { .. }))
                .expect("sesame pupil")
                .transform
                .translation[0]
        };
        assert!(pupil_x(&idle) < pupil_x(&moving));
    }

    #[test]
    fn dragging_continuously_squashes_and_restores_sesame() {
        let config = HashMap::from([("drag_tint".to_owned(), true)]);
        let pet = BuiltinSesamePet::default();
        pet.apply_config(PetConfigBag::new(&config));
        let resting = scene_at_with_drag(&pet, &config, 0.0, [0.0, 0.0], DragState::ACTIVE);
        let squashed = scene_at_with_drag(
            &pet,
            &config,
            std::f64::consts::FRAC_PI_2 / 9.0,
            [0.0, 0.0],
            DragState::ACTIVE,
        );
        let body_width = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find_map(|item| match &item.node {
                    SceneNode::GradientPath { path, .. } if item.z_index == 0 => Some(
                        path.points
                            .iter()
                            .map(|point| point[0])
                            .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), x| {
                                (min.min(x), max.max(x))
                            }),
                    ),
                    _ => None,
                })
                .expect("sesame body path")
        };
        let (resting_min, resting_max) = body_width(&resting);
        let (squashed_min, squashed_max) = body_width(&squashed);
        assert!((squashed_max - squashed_min) > (resting_max - resting_min));
    }

    #[test]
    fn docked_sesame_scene_moves_over_time() {
        let config = HashMap::from([("dock_tint".to_owned(), true)]);
        let pet = BuiltinSesamePet::default();
        pet.apply_config(PetConfigBag::new(&config));
        pet.dock_anim_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let first = scene_at_docked(
            &pet,
            &config,
            0.0,
            DockState {
                top: true,
                ..DockState::FREE
            },
        );
        pet.dock_anim_ms
            .store(700, std::sync::atomic::Ordering::Relaxed);
        let second = scene_at_docked(
            &pet,
            &config,
            0.0,
            DockState {
                top: true,
                ..DockState::FREE
            },
        );
        let path_y = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find_map(|item| match &item.node {
                    SceneNode::GradientPath { path, .. } if item.z_index == 0 => {
                        path.points.first().map(|p| p[1])
                    }
                    _ => None,
                })
                .expect("sesame body path")
        };
        assert_ne!(path_y(&first), path_y(&second));
    }

    #[test]
    fn floating_moves_bow_and_body_together() {
        let config = HashMap::new();
        let pet = BuiltinSesamePet::default();
        let resting = scene_at(&pet, &config, 0.0, [0.0, 0.0]);
        let peak = scene_at(&pet, &config, std::f64::consts::FRAC_PI_2 / 2.2, [0.0, 0.0]);
        let path_y = |scene: &PetScene, z_index| {
            scene
                .items
                .iter()
                .find_map(|item| match &item.node {
                    SceneNode::GradientPath { path, .. } if item.z_index == z_index => {
                        path.points.first().map(|point| point[1])
                    }
                    _ => None,
                })
                .expect("expected gradient path")
        };
        let bow_delta = path_y(&peak, -3) - path_y(&resting, -3);
        let body_delta = path_y(&peak, 0) - path_y(&resting, 0);
        assert!((bow_delta - body_delta).abs() < 0.000_1);
        assert!((body_delta - 0.018).abs() < 0.000_1);
    }
}
