//! 内置：大眼小球；全局键鼠对话气泡 + 贴边/拖动演示。

use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU32, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use deskhud_engine::{
    DockState, PetBubbleStyle, PetConfigBag, PetConfigOption, PetEvent, PetKey, PetKind,
    PetKindInfo, PetModifiers, PetMouseButton, PetPaint, PetPaintCtx, PetScene, PetTheme,
    SceneItem, SceneNode, Shape, Transform2D,
};

/// 默认宠物 `pet.deskhud.specs`。
#[derive(Debug)]
pub struct BuiltinSpecsPet {
    last_dock_bits: AtomicU8,
    last_dragging: AtomicBool,
    bubble_ms: AtomicU32,
    bubble_text: Mutex<String>,
    follow_eyes: AtomicBool,
    key_tips: AtomicBool,
    mouse_tips: AtomicBool,
    hover_highlight: AtomicBool,
    dock_tint: AtomicBool,
    click_ms: AtomicU32,
    last_pointer: Mutex<[i8; 2]>,
    idle_ms: AtomicU32,
    blink: Mutex<BlinkState>,
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

impl Default for BuiltinSpecsPet {
    fn default() -> Self {
        Self {
            last_dock_bits: AtomicU8::new(0),
            last_dragging: AtomicBool::new(false),
            bubble_ms: AtomicU32::new(0),
            bubble_text: Mutex::new(String::new()),
            follow_eyes: AtomicBool::new(true),
            key_tips: AtomicBool::new(true),
            mouse_tips: AtomicBool::new(true),
            hover_highlight: AtomicBool::new(true),
            dock_tint: AtomicBool::new(true),
            click_ms: AtomicU32::new(0),
            last_pointer: Mutex::new([0, 0]),
            idle_ms: AtomicU32::new(0),
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
        key: "follow_eyes",
        label: "鼠标跟随",
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

impl BuiltinSpecsPet {
    fn show_bubble(&self, text: impl Into<String>, ms: u32) {
        if let Ok(mut g) = self.bubble_text.lock() {
            *g = text.into();
        }
        self.bubble_ms.store(ms, Ordering::Relaxed);
    }
}

impl PetKind for BuiltinSpecsPet {
    fn info(&self) -> PetKindInfo {
        PetKindInfo {
            id: "pet.deskhud.specs",
            version: deskhud_engine::ENGINE_PRODUCT_VERSION,
            engine: deskhud_engine::ENGINE_COMPAT_FAMILY,
            display_name: "大眼球",
            description: "会眨眼、跟随鼠标视线的大眼小球。",
            author: "DeskHud",
            homepage: Some("https://github.com/ko-eika/deskhud"),
            window_width: 160.0,
            window_height: 168.0,
            preview: Some(include_bytes!("../assets/preview.svg")),
        }
    }

    fn config_options(&self) -> &'static [PetConfigOption] {
        SPECS_OPTIONS_ORDERED
    }

    fn apply_config(&self, config: PetConfigBag<'_>) {
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
            bubble_style: Default::default(),
        }
    }

    fn scene(&self, ctx: PetPaintCtx<'_>) -> PetScene {
        let paint = self.paint(ctx);
        let blink = self
            .blink
            .lock()
            .map(|state| state.eye_open())
            .unwrap_or(1.0);
        let pupil = [
            ctx.pointer_dir[0].clamp(-1.0, 1.0) * 4.5,
            ctx.pointer_dir[1].clamp(-1.0, 1.0) * 4.0,
        ];
        let mut items = vec![SceneItem {
            transform: Transform2D::default(),
            z_index: 0,
            node: SceneNode::Shape {
                shape: Shape::Circle { radius: 1.0 },
                color: [paint.body_rgb[0], paint.body_rgb[1], paint.body_rgb[2], 1.0],
            },
        }];
        items.push(SceneItem {
            transform: Transform2D::default(),
            z_index: -1,
            node: SceneNode::HitRegion {
                shape: Shape::Circle { radius: 1.0 },
            },
        });
        // Keep the original 160px pet's visual proportions: eye radius ~11px,
        // pupil radius ~4.5px, centers ±14px and 12px above center.
        // The renderer's base is 32% of the 160px window (51.2px).
        for x in [-0.273, 0.273] {
            items.push(SceneItem {
                transform: Transform2D {
                    translation: [x, -0.234],
                    scale: [0.215, 0.215 * blink.max(0.08)],
                    ..Transform2D::default()
                },
                z_index: 1,
                node: SceneNode::Shape {
                    shape: Shape::Ellipse { radii: [1.0, 1.0] },
                    color: [1.0, 1.0, 1.0, 1.0],
                },
            });
            items.push(SceneItem {
                transform: Transform2D {
                    translation: [x + pupil[0] / 100.0, -0.234 + pupil[1] / 100.0],
                    scale: [0.088, 0.088],
                    ..Transform2D::default()
                },
                z_index: 2,
                node: SceneNode::Shape {
                    shape: Shape::Circle { radius: 1.0 },
                    color: [0.08, 0.08, 0.1, 1.0],
                },
            });
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
