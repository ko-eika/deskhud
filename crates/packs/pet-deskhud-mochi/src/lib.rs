//! 内置：糯米团；沉稳的吉祥物宠物，带鼠标跟随、姿态和反馈动画。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use deskhud_engine::{
    DockState, Path, PetBubbleStyle, PetConfigBag, PetConfigOption, PetEvent, PetKey, PetKind,
    PetKindInfo, PetModifiers, PetMouseButton, PetPaint, PetPaintCtx, PetScene, PetTheme,
    SceneItem, SceneNode, Shape, Transform2D,
};

/// 糯米团 `pet.deskhud.mochi`。
#[derive(Debug)]
pub struct BuiltinMochiPet {
    last_dock_bits: AtomicU8,
    last_dragging: AtomicBool,
    bubble_ms: AtomicU32,
    bubble_text: Mutex<String>,
    custom_bubble: AtomicBool,
    follow_eyes: AtomicBool,
    key_tips: AtomicBool,
    mouse_tips: AtomicBool,
    hover_highlight: AtomicBool,
    dock_tint: AtomicBool,
    click_ms: AtomicU32,
    dock_anim_ms: AtomicU32,
    last_pointer: Mutex<[i8; 2]>,
    idle_ms: AtomicU32,
    eye_motion: Mutex<EyeMotion>,
    blink: Mutex<BlinkState>,
}

#[derive(Debug)]
struct EyeMotion {
    current: [f32; 2],
    last_time_secs: f64,
}

impl Default for EyeMotion {
    fn default() -> Self {
        Self {
            current: [0.0, 0.0],
            last_time_secs: 0.0,
        }
    }
}

/// Short, asymmetric eye-close cycle with locally generated timing jitter.
#[derive(Debug)]
struct BlinkState {
    next_blink_secs: f32,
    elapsed_secs: Option<f32>,
    random: u32,
}

#[allow(dead_code)]
impl BlinkState {
    const TOTAL_SECS: f32 = 0.18;
    const CLOSE_SECS: f32 = 0.05;
    const HOLD_SECS: f32 = 0.025;

    fn new() -> Self {
        let seed = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|elapsed| elapsed.subsec_nanos() ^ elapsed.as_secs() as u32)
            .unwrap_or(0x4d59_5df4);
        let mut state = Self {
            next_blink_secs: 0.0,
            elapsed_secs: None,
            random: seed,
        };
        state.next_blink_secs = 2.2 + state.next_random_unit() * 3.2;
        state
    }

    fn tick(&mut self, dt_secs: f32) {
        let dt_secs = dt_secs.clamp(0.0, 0.25);
        if let Some(elapsed_secs) = &mut self.elapsed_secs {
            *elapsed_secs += dt_secs;
            if *elapsed_secs >= Self::TOTAL_SECS {
                self.elapsed_secs = None;
                self.next_blink_secs = self.next_delay_secs();
            }
            return;
        }

        self.next_blink_secs -= dt_secs;
        if self.next_blink_secs <= 0.0 {
            self.elapsed_secs = Some(0.0);
        }
    }

    fn eye_open(&self) -> f32 {
        let Some(elapsed_secs) = self.elapsed_secs else {
            return 1.0;
        };
        if elapsed_secs < Self::CLOSE_SECS {
            return 1.0 - smooth_step(elapsed_secs / Self::CLOSE_SECS);
        }
        if elapsed_secs < Self::CLOSE_SECS + Self::HOLD_SECS {
            return 0.0;
        }
        let reopen_secs = Self::TOTAL_SECS - Self::CLOSE_SECS - Self::HOLD_SECS;
        smooth_step((elapsed_secs - Self::CLOSE_SECS - Self::HOLD_SECS) / reopen_secs)
    }

    fn next_delay_secs(&mut self) -> f32 {
        let random = self.next_random_unit();
        // Most blinks are 2.2–5.4 s apart; a small chance produces a quick double blink.
        if random < 0.16 {
            0.22 + self.next_random_unit() * 0.22
        } else {
            2.2 + self.next_random_unit() * 3.2
        }
    }

    fn next_random_unit(&mut self) -> f32 {
        let mut value = self.random;
        value ^= value << 13;
        value ^= value >> 17;
        value ^= value << 5;
        self.random = value;
        value as f32 / u32::MAX as f32
    }
}

#[allow(dead_code)]
fn smooth_step(value: f32) -> f32 {
    let value = value.clamp(0.0, 1.0);
    value * value * (3.0 - 2.0 * value)
}

impl Default for BuiltinMochiPet {
    fn default() -> Self {
        Self {
            last_dock_bits: AtomicU8::new(0),
            last_dragging: AtomicBool::new(false),
            bubble_ms: AtomicU32::new(0),
            bubble_text: Mutex::new(String::new()),
            custom_bubble: AtomicBool::new(false),
            follow_eyes: AtomicBool::new(true),
            key_tips: AtomicBool::new(true),
            mouse_tips: AtomicBool::new(true),
            hover_highlight: AtomicBool::new(true),
            dock_tint: AtomicBool::new(true),
            click_ms: AtomicU32::new(0),
            dock_anim_ms: AtomicU32::new(700),
            last_pointer: Mutex::new([0, 0]),
            idle_ms: AtomicU32::new(0),
            eye_motion: Mutex::new(EyeMotion::default()),
            blink: Mutex::new(BlinkState::new()),
        }
    }
}

#[allow(dead_code)]
const SPECS_OPTIONS: &[PetConfigOption] = &[
    PetConfigOption {
        key: "follow_eyes",
        label: "眼睛跟随指针",
        description: "瞳孔跟随桌面光标方向转动",
        default: true,
    },
    PetConfigOption {
        key: "hover_highlight",
        label: "按键提示",
        description: "键盘按下时显示短气泡（如 Ctrl+C）",
        default: true,
    },
    PetConfigOption {
        key: "drag_tint",
        label: "鼠标提示",
        description: "全局鼠标按键 / 滚轮时显示短气泡",
        default: true,
    },
    PetConfigOption {
        key: "dock_tint",
        label: "悬停高亮",
        description: "指针停在宠上时身体略提亮",
        default: true,
    },
    PetConfigOption {
        key: "key_tips",
        label: "贴边变色",
        description: "吸附屏幕边缘时改变身体颜色",
        default: true,
    },
    PetConfigOption {
        key: "mouse_tips",
        label: "点击瞪眼",
        description: "点击宠物时短暂瞪大眼睛",
        default: true,
    },
    PetConfigOption {
        key: "drag_tint",
        label: "空闲回正",
        description: "鼠标停止移动一段时间后恢复正视前方",
        default: true,
    },
    PetConfigOption {
        key: "mouse_tips",
        label: "拖拽变色",
        description: "拖拽宠物时改变身体颜色",
        default: true,
    },
];

const SPECS_OPTIONS_ORDERED: &[PetConfigOption] = &[
    PetConfigOption {
        key: "custom_bubble",
        label: "个性气泡",
        description: "使用宠物包定义的气泡颜色和圆角",
        default: false,
    },
    PetConfigOption {
        key: "follow_eyes",
        label: "眼睛效果",
        description: "鼠标移动时跟随，空闲后回正，点击时短暂朝鼠标方向瞪眼",
        default: true,
    },
    PetConfigOption {
        key: "hover_highlight",
        label: "悬停高亮",
        description: "指针停在宠物上时身体略微提亮",
        default: true,
    },
    PetConfigOption {
        key: "drag_tint",
        label: "拖拽效果",
        description: "拖拽宠物时提供视觉反馈",
        default: true,
    },
    PetConfigOption {
        key: "dock_tint",
        label: "贴边效果",
        description: "吸附屏幕边缘时提供视觉反馈",
        default: true,
    },
    PetConfigOption {
        key: "key_tips",
        label: "按键提示",
        description: "键盘按下时显示短气泡",
        default: true,
    },
    PetConfigOption {
        key: "mouse_tips",
        label: "鼠标提示",
        description: "鼠标按键或滚轮时显示短气泡",
        default: true,
    },
];

const MOCHI_BUBBLE_BACKGROUND: [f32; 4] = [0.12, 0.28, 0.58, 0.96];
const MOCHI_BUBBLE_TEXT: [f32; 4] = [1.0, 1.0, 1.0, 1.0];
const MOCHI_BUBBLE_CORNER_RADIUS: f32 = 12.0;

fn dock_bits(d: DockState) -> u8 {
    (u8::from(d.left))
        | (u8::from(d.right) << 1)
        | (u8::from(d.top) << 2)
        | (u8::from(d.bottom) << 3)
}

fn is_modifier_key(key: PetKey) -> bool {
    matches!(
        key,
        PetKey::Shift | PetKey::Ctrl | PetKey::Alt | PetKey::Super
    )
}

fn key_label(key: PetKey) -> String {
    match key {
        PetKey::Space => "空格".into(),
        PetKey::Escape => "Esc".into(),
        PetKey::Tab => "Tab".into(),
        PetKey::Enter => mac_key_label("Return", "Enter"),
        PetKey::Backspace => "Backspace".into(),
        PetKey::Delete => "Del".into(),
        PetKey::Insert => "Insert".into(),
        PetKey::Clear => "Clear".into(),
        PetKey::ArrowUp => "↑".into(),
        PetKey::ArrowDown => "↓".into(),
        PetKey::ArrowLeft => "←".into(),
        PetKey::ArrowRight => "→".into(),
        PetKey::Home => "Home".into(),
        PetKey::End => "End".into(),
        PetKey::PageUp => "PgUp".into(),
        PetKey::PageDown => "PgDn".into(),
        PetKey::Shift => mac_key_label("Shift", "Shift"),
        PetKey::Ctrl => mac_key_label("Control", "Ctrl"),
        PetKey::Alt => mac_key_label("Option", "Alt"),
        PetKey::Super => mac_key_label("Command", "Win"),
        PetKey::CapsLock => "Caps".into(),
        PetKey::NumLock => "NumLock".into(),
        PetKey::NumpadEnter => "Num Enter".into(),
        PetKey::NumpadDigit(n) => format!("Num {n}"),
        PetKey::NumpadAdd => "Num +".into(),
        PetKey::NumpadSubtract => "Num -".into(),
        PetKey::NumpadMultiply => "Num ×".into(),
        PetKey::NumpadDivide => "Num ÷".into(),
        PetKey::NumpadDecimal => "Num .".into(),
        PetKey::NumpadSeparator => "Num ,".into(),
        PetKey::Function(n) => format!("F{n}"),
        PetKey::Letter(c) | PetKey::Digit(c) | PetKey::Punct(c) => c.to_string(),
    }
}

fn format_shortcut(mods: PetModifiers, key: PetKey) -> String {
    #[cfg(target_os = "macos")]
    {
        let mut parts: Vec<String> = Vec::new();
        if mods.ctrl {
            parts.push("Control".into());
        }
        if mods.alt {
            parts.push("Option".into());
        }
        if mods.shift {
            parts.push("Shift".into());
        }
        if mods.meta {
            parts.push("Command".into());
        }
        parts.push(key_label(key));
        return parts.join("+");
    }
    #[cfg(not(target_os = "macos"))]
    {
        let mut parts: Vec<String> = Vec::new();
        if mods.ctrl {
            parts.push("Ctrl".into());
        }
        if mods.shift {
            parts.push("Shift".into());
        }
        if mods.alt {
            parts.push("Alt".into());
        }
        if mods.meta {
            parts.push("Win".into());
        }
        parts.push(key_label(key));
        parts.join("+")
    }
}

#[cfg(target_os = "macos")]
fn mac_key_label(mac: &str, _other: &str) -> String {
    mac.into()
}

#[cfg(not(target_os = "macos"))]
fn mac_key_label(_mac: &str, other: &str) -> String {
    other.into()
}

impl BuiltinMochiPet {
    fn show_bubble(&self, text: impl Into<String>, ms: u32) {
        if let Ok(mut g) = self.bubble_text.lock() {
            *g = text.into();
        }
        self.bubble_ms.store(ms, Ordering::Relaxed);
    }

    fn smooth_eye_target(&self, target: [f32; 2], time_secs: f64) -> [f32; 2] {
        let Ok(mut motion) = self.eye_motion.lock() else {
            return target;
        };
        let dt = (time_secs - motion.last_time_secs).clamp(0.0, 0.1) as f32;
        motion.last_time_secs = time_secs;
        // Exponential smoothing keeps the response frame-rate independent and
        // prevents a large cursor sample jump from teleporting the pupils.
        let alpha = 1.0 - (-12.0 * dt).exp();
        for (current, target) in motion.current.iter_mut().zip(target) {
            *current += (target - *current) * alpha;
        }
        motion.current
    }
}

impl PetKind for BuiltinMochiPet {
    fn info(&self) -> PetKindInfo {
        PetKindInfo {
            id: "pet.deskhud.mochi",
            version: deskhud_engine::ENGINE_PRODUCT_VERSION,
            engine: deskhud_engine::ENGINE_COMPAT_FAMILY,
            display_name: "糯米团",
            description: "糯米团性格沉稳可靠，遇事总想站出来照看大家；熟悉之后也会露出顽皮的一面，是让人安心的陪伴者。",
            author: "DeskHud",
            homepage: Some("https://github.com/ko-eika/deskhud"),
            window_width: 192.0,
            window_height: 192.0,
            preview: Some(include_bytes!("../assets/preview.svg")),
        }
    }

    fn config_options(&self) -> &'static [PetConfigOption] {
        SPECS_OPTIONS_ORDERED
    }

    fn apply_config(&self, config: PetConfigBag<'_>) {
        self.custom_bubble
            .store(config.get("custom_bubble", false), Ordering::Relaxed);
        self.follow_eyes
            .store(config.get("follow_eyes", true), Ordering::Relaxed);
        self.key_tips
            .store(config.get("key_tips", true), Ordering::Relaxed);
        self.mouse_tips
            .store(config.get("mouse_tips", true), Ordering::Relaxed);
        self.hover_highlight
            .store(config.get("hover_highlight", true), Ordering::Relaxed);
        self.dock_tint
            .store(config.get("dock_tint", true), Ordering::Relaxed);
    }

    fn tick(&self, dt_secs: f32) {
        let dec = (dt_secs * 1000.0).max(0.0) as u32;
        self.dock_anim_ms.fetch_add(dec, Ordering::Relaxed);
        let _ = self
            .bubble_ms
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(dec))
            });
        let _ = self
            .click_ms
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_sub(dec))
            });
        let _ = self
            .idle_ms
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |v| {
                Some(v.saturating_add(dec).min(10_000))
            });
        if let Ok(mut blink) = self.blink.lock() {
            blink.tick(dt_secs);
        }
    }

    fn on_event(&self, event: PetEvent) {
        match event {
            PetEvent::DragStarted => {
                self.last_dragging.store(true, Ordering::Relaxed);
            }
            PetEvent::DragEnded { .. } => {
                self.last_dragging.store(false, Ordering::Relaxed);
            }
            PetEvent::DockChanged { to, .. } => {
                self.last_dock_bits.store(dock_bits(to), Ordering::Relaxed);
                self.dock_anim_ms.store(0, Ordering::Relaxed);
            }
            PetEvent::GlobalMousePressed { button, .. } => {
                if button == PetMouseButton::Primary {
                    self.click_ms.store(420, Ordering::Relaxed);
                    self.idle_ms.store(0, Ordering::Relaxed);
                }
                if !self.mouse_tips.load(Ordering::Relaxed) {
                    return;
                }
                let text = match button {
                    PetMouseButton::Primary => "左键",
                    PetMouseButton::Secondary => "右键",
                    PetMouseButton::Middle => "中键",
                };
                self.show_bubble(text, 1000);
            }
            PetEvent::MouseClicked {
                button: PetMouseButton::Primary,
                ..
            } => {
                self.click_ms.store(420, Ordering::Relaxed);
                self.idle_ms.store(0, Ordering::Relaxed);
            }
            PetEvent::MousePressed { button, .. } => {
                if button == PetMouseButton::Primary {
                    self.click_ms.store(420, Ordering::Relaxed);
                    self.idle_ms.store(0, Ordering::Relaxed);
                }
                if !self.mouse_tips.load(Ordering::Relaxed) {
                    return;
                }
                let text = match button {
                    PetMouseButton::Primary => "左键",
                    PetMouseButton::Secondary => "右键",
                    PetMouseButton::Middle => "中键",
                };
                self.show_bubble(text, 1000);
            }
            PetEvent::MouseWheel { delta, .. } | PetEvent::GlobalMouseWheel { delta, .. } => {
                if !self.mouse_tips.load(Ordering::Relaxed) {
                    return;
                }
                if delta > 0 {
                    self.show_bubble("滚轮↑", 800);
                } else if delta < 0 {
                    self.show_bubble("滚轮↓", 800);
                }
            }
            PetEvent::GlobalKeyPressed { key, modifiers }
            | PetEvent::KeyPressed { key, modifiers } => {
                if !self.key_tips.load(Ordering::Relaxed) {
                    return;
                }
                if is_modifier_key(key) {
                    self.show_bubble(key_label(key), 900);
                    return;
                }
                self.show_bubble(format_shortcut(modifiers, key), 1400);
            }
            PetEvent::GlobalMouseReleased { .. }
            | PetEvent::GlobalKeyReleased { .. }
            | PetEvent::MouseHover { .. }
            | PetEvent::MouseReleased { .. }
            | PetEvent::MouseClicked { .. }
            | PetEvent::MouseDoubleClicked { .. }
            | PetEvent::KeyReleased { .. } => {}
        }
    }

    fn paint(&self, ctx: PetPaintCtx<'_>) -> PetPaint {
        let hover_hl = ctx.config.get("hover_highlight", true);
        let dock_tint = ctx.config.get("dock_tint", true);
        let drag_tint = ctx.config.get("drag_tint", true);
        let tips_on = ctx.config.get("key_tips", true) || ctx.config.get("mouse_tips", true);

        let _ = (
            self.last_dock_bits.load(Ordering::Relaxed),
            self.last_dragging.load(Ordering::Relaxed),
        );
        let dock = ctx.dock;
        let dragging = ctx.drag.is_dragging();
        let hovering = ctx.mouse.hovering;
        let bubble_left = self.bubble_ms.load(Ordering::Relaxed);
        let bubble_text = if tips_on && bubble_left > 0 {
            self.bubble_text
                .lock()
                .ok()
                .filter(|s| !s.is_empty())
                .map(|s| s.clone())
        } else {
            None
        };
        let pointer = [
            (ctx.pointer_dir[0].clamp(-1.0, 1.0) * 8.0) as i8,
            (ctx.pointer_dir[1].clamp(-1.0, 1.0) * 8.0) as i8,
        ];
        let pointer_changed = self
            .last_pointer
            .lock()
            .map(|mut last| {
                let changed = *last != pointer;
                *last = pointer;
                changed
            })
            .unwrap_or(false);
        if pointer_changed {
            self.idle_ms.store(0, Ordering::Relaxed);
        }

        let mut body: [f32; 3] = [0.20, 0.58, 0.96];
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
        if hover_hl && hovering && !dragging {
            body = [
                (body[0] + 0.08).min(1.0),
                (body[1] + 0.06).min(1.0),
                (body[2] + 0.04).min(1.0),
            ];
        }
        if drag_tint && dragging {
            body = [
                (body[0] * 0.55 + 0.45).min(1.0),
                (body[1] * 0.75 + 0.20).min(1.0),
                (body[2] * 0.65 + 0.15).min(1.0),
            ];
        }

        PetPaint {
            body_rgb: body,
            bubble_text,
            bubble_style: if self.custom_bubble.load(Ordering::Relaxed) {
                PetBubbleStyle::Custom {
                    background_rgba: MOCHI_BUBBLE_BACKGROUND,
                    text_rgba: MOCHI_BUBBLE_TEXT,
                    corner_radius: MOCHI_BUBBLE_CORNER_RADIUS,
                }
            } else {
                PetBubbleStyle::default()
            },
        }
    }

    fn scene(&self, ctx: PetPaintCtx<'_>) -> PetScene {
        let paint = self.paint(ctx);
        let blink = self
            .blink
            .lock()
            .map(|state| state.eye_open())
            .unwrap_or(1.0);
        let click = (self.click_ms.load(Ordering::Relaxed) as f32 / 420.0).clamp(0.0, 1.0);
        let drag_scale = drag_squash_scale(
            ctx.time_secs,
            ctx.drag.is_dragging(),
            ctx.config.get("drag_tint", true),
        );
        let idle = self.idle_ms.load(Ordering::Relaxed) >= 900;
        let target = if self.follow_eyes.load(Ordering::Relaxed) && (!idle || click > 0.0) {
            [
                ctx.pointer_dir[0].clamp(-1.0, 1.0),
                if ctx.dock.top {
                    -ctx.pointer_dir[1].clamp(-1.0, 1.0)
                } else {
                    ctx.pointer_dir[1].clamp(-1.0, 1.0)
                },
            ]
        } else {
            [0.0, 0.0]
        };
        let pointer = self.smooth_eye_target(target, ctx.time_secs);
        let pupil_radius = 24.0;
        let neutral_pupil = [6.0, 6.0];
        let pupil_offset = bounded_eye_offset(
            pointer,
            neutral_pupil,
            [55.0 - pupil_radius - 5.0, 61.0 - pupil_radius - 5.0],
        );
        let pupil_motion = [
            pupil_offset[0] - neutral_pupil[0],
            pupil_offset[1] - neutral_pupil[1],
        ];
        let mut items = Vec::new();
        if ctx.shadows
            && (!(ctx.dock.left || ctx.dock.right || ctx.dock.top || ctx.dock.bottom)
                || ctx.drag.is_dragging())
        {
            for (scale, alpha) in [(1.22, 0.025), (1.08, 0.04), (0.92, 0.06)] {
                items.push(SceneItem {
                    transform: Transform2D {
                        translation: [0.0, svg_y(442.0)],
                        scale: [svg_scale(152.0 * scale), svg_scale(24.0 * scale)],
                        ..Transform2D::default()
                    },
                    z_index: -3,
                    node: SceneNode::Shape {
                        shape: Shape::Ellipse { radii: [1.0, 1.0] },
                        color: [0.0, 0.08, 0.18, alpha],
                    },
                });
            }
        }
        let pet_start = items.len();
        items.extend([
            SceneItem {
                transform: Transform2D::default(),
                z_index: 0,
                node: SceneNode::GradientPath {
                    path: mochi_body_path(),
                    top_color: [0.659, 0.847, 1.0, 1.0],
                    bottom_color: [0.902, 0.957, 1.0, 1.0],
                },
            },
            SceneItem {
                transform: Transform2D::default(),
                z_index: -1,
                node: SceneNode::HitRegion {
                    shape: Shape::Ellipse { radii: [1.08, 1.0] },
                },
            },
        ]);
        for (x, y) in [(174.0, 250.0), (338.0, 250.0)] {
            items.push(SceneItem {
                transform: Transform2D {
                    translation: [svg_x(x), svg_y(y)],
                    scale: [svg_scale(55.0), svg_scale(61.0) * blink.max(0.08)],
                    ..Transform2D::default()
                },
                z_index: 1,
                node: SceneNode::Shape {
                    shape: Shape::Ellipse { radii: [1.0, 1.0] },
                    color: [1.0, 1.0, 1.0, 0.98],
                },
            });
            let pupil_center = [svg_x(x + pupil_offset[0]), svg_y(y + pupil_offset[1])];
            if blink > 0.08 {
                // Approximate the preview SVG's off-center radial gradient
                // with enough nested neutral circles to stay smooth at audit
                // sizes. Closed eyes omit pupil layers entirely.
                for layer in 0..=20 {
                    let t = layer as f32 / 20.0;
                    let radius = pupil_radius * (1.0 - t * 0.94);
                    let focal_shift = 6.0 * t / 160.0;
                    let color = std::array::from_fn(|index| {
                        const OUTER: [f32; 4] = [0.102, 0.231, 0.427, 1.0];
                        const INNER: [f32; 4] = [0.290, 0.435, 0.647, 1.0];
                        OUTER[index] + (INNER[index] - OUTER[index]) * t
                    });
                    items.push(SceneItem {
                        transform: Transform2D {
                            translation: [
                                pupil_center[0] - focal_shift,
                                pupil_center[1] - focal_shift,
                            ],
                            scale: [svg_scale(radius), svg_scale(radius) * blink],
                            ..Transform2D::default()
                        },
                        z_index: 2,
                        node: SceneNode::Shape {
                            shape: Shape::Circle { radius: 1.0 },
                            color,
                        },
                    });
                }
                items.push(SceneItem {
                    transform: Transform2D {
                        translation: [
                            svg_x(x - 3.0 + pupil_motion[0]),
                            svg_y(y - 3.0 + pupil_motion[1]),
                        ],
                        scale: [svg_scale(7.5), svg_scale(7.5) * blink],
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
        items.push(SceneItem {
            transform: Transform2D::default(),
            z_index: 3,
            node: SceneNode::Path(Path {
                points: (0..=12)
                    .map(|step| {
                        let t = step as f32 / 12.0;
                        let u = 1.0 - t;
                        svg_point(
                            u * u * 232.0 + 2.0 * u * t * 256.0 + t * t * 280.0,
                            u * u * 332.0 + 2.0 * u * t * 354.0 + t * t * 332.0,
                        )
                    })
                    .collect(),
                closed: false,
                fill: None,
                stroke: Some([0.10, 0.23, 0.43, 1.0]),
                stroke_width: 9.0 / 160.0,
            }),
        });
        let dock_enabled = ctx.config.get("dock_tint", true);
        let docked = ctx.dock.left || ctx.dock.right || ctx.dock.top || ctx.dock.bottom;
        let sway = if docked && !ctx.drag.is_dragging() {
            0.0
        } else {
            (ctx.time_secs as f32 * 2.2).sin() * 0.018
        };
        let tuck_progress = if docked && dock_enabled && !ctx.drag.is_dragging() {
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
        for item in &mut items[pet_start..] {
            if ctx.dock.top
                && !ctx.drag.is_dragging()
                && !matches!(item.node, SceneNode::HitRegion { .. })
            {
                // A pet coming down from the top hangs head-first.
                scale_scene_item(item, [0.0, 0.0], [1.0, -1.0]);
            }
            let (x, y) = if matches!(item.node, SceneNode::HitRegion { .. }) {
                (hit_x, hit_y)
            } else {
                (dock_x, dock_y)
            };
            translate_scene_item(item, x, y + sway);
            if !matches!(item.node, SceneNode::HitRegion { .. }) {
                scale_scene_item(item, [0.0, sway], drag_scale);
                if !ctx.drag.is_dragging() {
                    rotate_scene_item(item, dock_tilt(ctx.dock), [x, y + sway]);
                }
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
                    scale: [1.0, 1.0],
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
        // First leave the window completely, then return to a half-visible
        // mounted pose. The scene remains one rigid piece so facial features
        // cannot separate during docking.
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

fn mochi_body_path() -> Path {
    // The preview uses a soft, three-lobed lower contour.  Sampling each
    // cubic densely keeps those shallow valleys visible at runtime instead
    // of turning them into a coarse, almost circular edge.
    let mut points = Vec::with_capacity(128);
    let segments = [
        ([85.0, 308.0], [79.0, 232.0], [95.0, 128.0], [236.0, 111.0]),
        (
            [236.0, 111.0],
            [244.0, 110.0],
            [251.0, 110.0],
            [256.0, 110.0],
        ),
        (
            [256.0, 110.0],
            [261.0, 110.0],
            [268.0, 110.0],
            [276.0, 111.0],
        ),
        (
            [276.0, 111.0],
            [417.0, 128.0],
            [433.0, 232.0],
            [427.0, 308.0],
        ),
        (
            [427.0, 308.0],
            [421.0, 363.0],
            [387.0, 397.0],
            [354.0, 378.0],
        ),
        (
            [354.0, 378.0],
            [326.0, 430.0],
            [293.0, 430.0],
            [256.0, 384.0],
        ),
        (
            [256.0, 384.0],
            [219.0, 430.0],
            [186.0, 430.0],
            [158.0, 378.0],
        ),
        ([158.0, 378.0], [125.0, 397.0], [91.0, 363.0], [85.0, 308.0]),
    ];
    for (index, (a, b, c, d)) in segments.into_iter().enumerate() {
        if index == 0 {
            points.push(svg_point(a[0], a[1]));
        }
        for step in 1..=16 {
            let t = step as f32 / 16.0;
            let u = 1.0 - t;
            points.push(svg_point(
                u * u * u * a[0]
                    + 3.0 * u * u * t * b[0]
                    + 3.0 * u * t * t * c[0]
                    + t * t * t * d[0],
                u * u * u * a[1]
                    + 3.0 * u * u * t * b[1]
                    + 3.0 * u * t * t * c[1]
                    + t * t * t * d[1],
            ));
        }
    }
    Path {
        points,
        closed: true,
        fill: None,
        stroke: Some([0.549, 0.776, 1.0, 1.0]),
        stroke_width: 9.0 / 160.0,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use deskhud_engine::{
        DockState, DragState, MouseState, PetBubbleStyle, PetConfigBag, PetKind, PetPaintCtx,
        PetScene, PetTheme, SceneNode,
    };

    use super::{BlinkState, BuiltinMochiPet, bounded_eye_offset};

    fn scene_at(
        pet: &BuiltinMochiPet,
        config: &HashMap<String, bool>,
        time_secs: f64,
        pointer_dir: [f32; 2],
    ) -> PetScene {
        scene_at_with_drag(pet, config, time_secs, pointer_dir, DragState::IDLE)
    }

    fn scene_at_with_drag(
        pet: &BuiltinMochiPet,
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
        pet: &BuiltinMochiPet,
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
    fn scene_contains_mochi_vector_artwork() {
        let config = HashMap::new();
        let scene = scene_at(&BuiltinMochiPet::default(), &config, 0.0, [0.0, 0.0]);
        assert!(scene.validate().is_ok());
        assert!(
            scene
                .items
                .iter()
                .any(|item| matches!(item.node, SceneNode::GradientPath { .. }))
        );
        assert!(
            scene
                .items
                .iter()
                .filter(|item| matches!(item.node, SceneNode::Shape { .. }))
                .count()
                >= 45
        );
        let body = scene
            .items
            .iter()
            .find_map(|item| match &item.node {
                SceneNode::GradientPath { path, .. } if item.z_index == 0 => Some(path),
                _ => None,
            })
            .expect("mochi body path");
        assert_eq!(body.points.len(), 129);
        assert_eq!(body.points.first(), body.points.last());
    }

    #[test]
    fn custom_bubble_uses_mochi_palette() {
        let config = HashMap::from([("custom_bubble".to_owned(), true)]);
        let pet = BuiltinMochiPet::default();
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
                background_rgba: [0.12, 0.28, 0.58, 0.96],
                text_rgba: [1.0, 1.0, 1.0, 1.0],
                corner_radius: 12.0,
            }
        );
    }

    #[test]
    fn pupil_offsets_stay_inside_mochi_eye_socket() {
        let limits = [26.0, 32.0];
        for direction in [[-1.0, 0.0], [1.0, 0.0], [0.0, -1.0], [0.0, 1.0], [1.0, 1.0]] {
            let offset = bounded_eye_offset(direction, [6.0, 6.0], limits);
            let ellipse = (offset[0] / limits[0]).powi(2) + (offset[1] / limits[1]).powi(2);
            assert!(ellipse <= 1.000_1);
        }
        assert_eq!(
            bounded_eye_offset([0.0, 0.0], [6.0, 6.0], limits),
            [6.0, 6.0]
        );
    }

    #[test]
    fn closed_eyes_hide_pupils_and_highlights() {
        let config = HashMap::new();
        let pet = BuiltinMochiPet::default();
        pet.blink.lock().expect("blink state").elapsed_secs = Some(BlinkState::CLOSE_SECS + 0.01);
        let scene = scene_at(&pet, &config, 0.0, [0.0, 0.0]);

        assert_eq!(
            scene
                .items
                .iter()
                .filter(|item| item.z_index == 1 && matches!(item.node, SceneNode::Shape { .. }))
                .count(),
            2
        );
        assert!(
            !scene
                .items
                .iter()
                .any(|item| item.z_index == 2 && matches!(item.node, SceneNode::Shape { .. }))
        );
        assert!(
            !scene
                .items
                .iter()
                .any(|item| item.z_index == 3 && matches!(item.node, SceneNode::Shape { .. }))
        );
    }

    #[test]
    fn highlight_moves_with_the_pupil() {
        let config = HashMap::new();
        let pet = BuiltinMochiPet::default();
        let centered = scene_at(&pet, &config, 0.0, [0.0, 0.0]);
        let shifted = scene_at(&pet, &config, 0.1, [1.0, 0.0]);
        let pupil_x = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find(|item| item.z_index == 2 && matches!(item.node, SceneNode::Shape { .. }))
                .expect("mochi pupil")
                .transform
                .translation[0]
        };
        let highlight_x = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find(|item| item.z_index == 3 && matches!(item.node, SceneNode::Shape { .. }))
                .expect("mochi highlight")
                .transform
                .translation[0]
        };

        let pupil_delta = pupil_x(&shifted) - pupil_x(&centered);
        let highlight_delta = highlight_x(&shifted) - highlight_x(&centered);
        assert!((highlight_delta - pupil_delta).abs() < 0.000_1);
    }

    #[test]
    fn clicking_does_not_resize_mochi_pupils() {
        let config = HashMap::new();
        let pet = BuiltinMochiPet::default();
        let normal = scene_at(&pet, &config, 0.0, [0.0, 0.0]);
        pet.click_ms
            .store(420, std::sync::atomic::Ordering::Relaxed);
        let clicked = scene_at(&pet, &config, 0.0, [0.0, 0.0]);
        let pupil_scale = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find(|item| item.z_index == 2 && matches!(item.node, SceneNode::Shape { .. }))
                .expect("mochi pupil")
                .transform
                .scale
        };
        assert_eq!(pupil_scale(&normal), pupil_scale(&clicked));
    }

    #[test]
    fn dragging_continuously_squashes_and_restores_mochi() {
        let config = HashMap::new();
        let pet = BuiltinMochiPet::default();
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
                .expect("mochi body path")
        };
        let (resting_min, resting_max) = body_width(&resting);
        let (squashed_min, squashed_max) = body_width(&squashed);
        assert!((squashed_max - squashed_min) > (resting_max - resting_min));
    }

    #[test]
    fn docked_mochi_scene_moves_over_time() {
        let config = HashMap::new();
        let pet = BuiltinMochiPet::default();
        pet.dock_anim_ms
            .store(0, std::sync::atomic::Ordering::Relaxed);
        let first = scene_at_docked(
            &pet,
            &config,
            0.0,
            DockState {
                left: true,
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
                left: true,
                ..DockState::FREE
            },
        );
        let path_x = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find_map(|item| match &item.node {
                    SceneNode::GradientPath { path, .. } if item.z_index == 0 => {
                        path.points.first().map(|p| p[0])
                    }
                    _ => None,
                })
                .expect("mochi body path")
        };
        assert_ne!(path_x(&first), path_x(&second));
    }

    #[test]
    fn floating_moves_body_and_eyes_together_but_keeps_shadow_fixed() {
        let config = HashMap::new();
        let pet = BuiltinMochiPet::default();
        let resting = scene_at(&pet, &config, 0.0, [0.0, 0.0]);
        let peak = scene_at(&pet, &config, std::f64::consts::FRAC_PI_2 / 2.2, [0.0, 0.0]);
        let body_y = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find_map(|item| match &item.node {
                    SceneNode::GradientPath { path, .. } if item.z_index == 0 => {
                        path.points.first().map(|point| point[1])
                    }
                    _ => None,
                })
                .expect("mochi body path")
        };
        let eye_y = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find_map(|item| match &item.node {
                    SceneNode::Shape { color, .. }
                        if item.z_index == 1 && *color == [1.0, 1.0, 1.0, 0.98] =>
                    {
                        Some(item.transform.translation[1])
                    }
                    _ => None,
                })
                .expect("mochi eye white")
        };
        let shadow_y = |scene: &PetScene| {
            scene
                .items
                .iter()
                .find(|item| item.z_index == -3)
                .expect("mochi shadow")
                .transform
                .translation[1]
        };

        let body_delta = body_y(&peak) - body_y(&resting);
        let eye_delta = eye_y(&peak) - eye_y(&resting);
        assert!((body_delta - 0.018).abs() < 0.000_1);
        assert!((eye_delta - body_delta).abs() < 0.000_1);
        assert!((shadow_y(&peak) - shadow_y(&resting)).abs() < 0.000_1);
    }
}
