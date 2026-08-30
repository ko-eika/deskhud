//! Wasmtime Component Model adapter for community pet Guests.
//!
//! The adapter is deliberately the only layer that knows Wasmtime. The engine
//! crate remains a neutral contract and the SDK remains guest-only.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use deskhud_engine::{
    AssetFrame, AssetKind, PetAsset, PetEvent, PetKind, PetKindInfo, PetPaint, PetPaintCtx,
    PetScene,
};
use wasmtime::{Config, Engine, Store, component::Component};

use crate::RuntimeError;

mod bindings {
    wasmtime::component::bindgen!({
        path: "../deskhud-sdk/wit",
        world: "pet-guest",
    });
}

use bindings::exports::deskhud::guest::pet_api;

/// Limits applied to every community Guest instance.
#[derive(Debug, Clone, Copy)]
pub struct WasmLimits {
    /// Maximum fuel consumed by one Guest call.
    pub fuel_per_call: u64,
    /// Maximum linear memory in bytes.
    pub max_memory_bytes: usize,
    /// Maximum host time spent in one Guest call.
    pub call_timeout: Duration,
}

impl Default for WasmLimits {
    fn default() -> Self {
        Self {
            fuel_per_call: 2_000_000,
            max_memory_bytes: 16 * 1024 * 1024,
            call_timeout: Duration::from_millis(25),
        }
    }
}

/// A loaded WASM pet implementing the same engine trait as built-in Rust pets.
pub struct WasmPet {
    info: PetKindInfo,
    config_options: &'static [deskhud_engine::PetConfigOption],
    guest: Mutex<GuestState>,
    limits: WasmLimits,
    assets: HashMap<String, GuestAsset>,
}

/// Loads a community pet component with the default sandbox limits.
pub fn load_wasm_guest(wasm_bytes: &[u8]) -> Result<Arc<WasmPet>, RuntimeError> {
    WasmPet::load(wasm_bytes, WasmLimits::default())
}

struct GuestState {
    store: Store<wasmtime::StoreLimits>,
    guest: bindings::PetGuest,
}

impl std::fmt::Debug for WasmPet {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WasmPet").field("info", &self.info).finish()
    }
}

impl WasmPet {
    /// Loads a component with no WASI linker and validates its initial metadata.
    pub fn load(wasm_bytes: &[u8], limits: WasmLimits) -> Result<Arc<Self>, RuntimeError> {
        Self::load_with_preview(wasm_bytes, limits, None)
    }

    /// Loads a component and attaches the package-provided settings preview.
    pub fn load_with_preview(
        wasm_bytes: &[u8],
        limits: WasmLimits,
        preview: Option<Vec<u8>>,
    ) -> Result<Arc<Self>, RuntimeError> {
        Self::load_with_preview_and_assets(wasm_bytes, limits, preview, HashMap::new())
    }

    /// Loads a component and injects resources already validated by the
    /// package layer. The Guest still only receives scene-neutral data.
    pub fn load_with_preview_and_assets(
        wasm_bytes: &[u8],
        limits: WasmLimits,
        preview: Option<Vec<u8>>,
        assets: HashMap<String, GuestAsset>,
    ) -> Result<Arc<Self>, RuntimeError> {
        let mut config = Config::new();
        config.consume_fuel(true);
        config.wasm_component_model(true);
        let engine = Engine::new(&config).map_err(wasm_error)?;
        let component = Component::new(&engine, wasm_bytes).map_err(wasm_error)?;
        let linker = wasmtime::component::Linker::new(&engine);
        let store_limits = wasmtime::StoreLimitsBuilder::new()
            .memory_size(limits.max_memory_bytes)
            .instances(1)
            .tables(4)
            .build();
        let mut store = Store::new(&engine, store_limits);
        store.limiter(|limits| limits);
        store.set_fuel(limits.fuel_per_call).map_err(wasm_error)?;
        let guest =
            bindings::PetGuest::instantiate(&mut store, &component, &linker).map_err(wasm_error)?;
        let info = guest
            .deskhud_guest_pet_api()
            .call_info(&mut store)
            .map_err(wasm_error)?;
        let (info, config_options) = info_to_engine(info, preview)?;
        let pet = Arc::new(Self {
            info,
            config_options,
            guest: Mutex::new(GuestState { store, guest }),
            limits,
            assets,
        });
        Ok(pet)
    }

    fn call<T>(
        &self,
        f: impl FnOnce(&mut GuestState) -> Result<T, wasmtime::Error>,
    ) -> Result<T, RuntimeError> {
        let started = Instant::now();
        let mut state = self
            .guest
            .lock()
            .map_err(|_| RuntimeError::Wasm("guest lock poisoned".into()))?;
        state
            .store
            .set_fuel(self.limits.fuel_per_call)
            .map_err(wasm_error)?;
        let out = f(&mut state).map_err(wasm_error)?;
        if started.elapsed() > self.limits.call_timeout {
            return Err(RuntimeError::Wasm("guest call exceeded time limit".into()));
        }
        Ok(out)
    }
}

impl PetKind for WasmPet {
    fn info(&self) -> PetKindInfo {
        self.info.clone()
    }
    fn config_options(&self) -> &'static [deskhud_engine::PetConfigOption] {
        self.config_options
    }
    fn apply_config(&self, config: deskhud_engine::PetConfigBag<'_>) {
        let config: Vec<_> = config
            .iter()
            .map(|(key, value)| pet_api::ConfigEntry {
                key: key.to_owned(),
                value,
            })
            .collect();
        let _ = self.call(|s| {
            s.guest
                .deskhud_guest_pet_api()
                .call_apply_config(&mut s.store, &config)
        });
    }
    fn tick(&self, dt_secs: f32) {
        let _ = self.call(|s| {
            s.guest
                .deskhud_guest_pet_api()
                .call_tick(&mut s.store, dt_secs)
        });
    }
    fn on_event(&self, event: PetEvent) {
        if let Some(event) = event_to_guest(event) {
            let _ = self.call(|s| {
                s.guest
                    .deskhud_guest_pet_api()
                    .call_on_event(&mut s.store, event)
            });
        }
    }
    fn asset(&self, id: &str) -> Option<PetAsset<'_>> {
        self.assets.get(id).map(GuestAsset::view)
    }
    fn paint(&self, _ctx: PetPaintCtx<'_>) -> PetPaint {
        PetPaint::default()
    }
    fn scene(&self, ctx: PetPaintCtx<'_>) -> PetScene {
        let result = self.call(|s| {
            s.guest
                .deskhud_guest_pet_api()
                .call_render(&mut s.store, &ctx_to_guest(ctx))
        });
        match result.and_then(scene_to_engine) {
            Ok(scene) => scene,
            Err(error) => {
                tracing::warn!(%error, "WASM pet scene rejected");
                PetScene::default()
            }
        }
    }
}

/// Resource data copied from the package index into the WASM pet adapter.
#[derive(Debug, Clone)]
pub struct GuestAsset {
    /// Raw validated resource bytes.
    pub bytes: Vec<u8>,
    /// Neutral resource category.
    pub kind: AssetKind,
    /// Atlas or sequence frame rectangles.
    pub frames: Vec<AssetFrame>,
}

impl GuestAsset {
    fn view(&self) -> PetAsset<'_> {
        PetAsset {
            bytes: &self.bytes,
            kind: self.kind,
            frames: &self.frames,
        }
    }
}

fn wasm_error(error: impl std::fmt::Display) -> RuntimeError {
    RuntimeError::Wasm(error.to_string())
}

fn info_to_engine(
    info: pet_api::PetInfo,
    preview: Option<Vec<u8>>,
) -> Result<(PetKindInfo, &'static [deskhud_engine::PetConfigOption]), RuntimeError> {
    if !valid_guest_text(&info.id)
        || !valid_guest_text(&info.display_name)
        || !valid_guest_text(&info.description)
        || !valid_guest_text(&info.author)
        || info.homepage.as_ref().is_some_and(|s| !valid_guest_text(s))
        || !valid_guest_text(&info.version)
        || !valid_guest_text(&info.engine)
        || !info.window_width.is_finite()
        || !info.window_height.is_finite()
        || !(1.0..=4096.0).contains(&info.window_width)
        || !(1.0..=4096.0).contains(&info.window_height)
        || info.config_options.len() > 64
    {
        return Err(RuntimeError::Wasm(
            "guest returned invalid pet metadata".into(),
        ));
    }
    let mut option_keys = std::collections::HashSet::new();
    if info.config_options.iter().any(|option| {
        !valid_guest_text(&option.key)
            || !valid_guest_text(&option.label)
            || !valid_guest_text(&option.description)
            || !option_keys.insert(option.key.as_str())
    }) {
        return Err(RuntimeError::Wasm(
            "guest returned invalid or duplicate config option".into(),
        ));
    }
    let config_options = info
        .config_options
        .into_iter()
        .map(|option| deskhud_engine::PetConfigOption {
            key: Box::leak(option.key.into_boxed_str()),
            label: Box::leak(option.label.into_boxed_str()),
            description: Box::leak(option.description.into_boxed_str()),
            default: option.default,
        })
        .collect::<Vec<_>>();
    let config_options = Box::leak(config_options.into_boxed_slice());
    let preview = preview.map(|bytes| Box::leak(bytes.into_boxed_slice()) as &'static [u8]);
    Ok((
        PetKindInfo {
            id: Box::leak(info.id.into_boxed_str()),
            display_name: Box::leak(info.display_name.into_boxed_str()),
            description: Box::leak(info.description.into_boxed_str()),
            author: Box::leak(info.author.into_boxed_str()),
            homepage: info
                .homepage
                .map(|s| Box::leak(s.into_boxed_str()) as &'static str),
            version: Box::leak(info.version.into_boxed_str()),
            engine: Box::leak(info.engine.into_boxed_str()),
            window_width: info.window_width,
            window_height: info.window_height,
            preview,
        },
        config_options,
    ))
}

fn valid_guest_text(text: &str) -> bool {
    !text.trim().is_empty() && text.chars().count() <= 4096
}

fn ctx_to_guest(ctx: PetPaintCtx<'_>) -> pet_api::PaintContext {
    pet_api::PaintContext {
        time_secs: ctx.time_secs,
        pointer_dir: (ctx.pointer_dir[0], ctx.pointer_dir[1]),
        status_line: ctx.status_line.to_owned(),
        dock: pet_api::DockState {
            left: ctx.dock.left,
            right: ctx.dock.right,
            top: ctx.dock.top,
            bottom: ctx.dock.bottom,
        },
        drag: pet_api::DragState {
            active: ctx.drag.active,
        },
        mouse: pet_api::MouseState {
            hovering: ctx.mouse.hovering,
            primary_down: ctx.mouse.primary_down,
            secondary_down: ctx.mouse.secondary_down,
            middle_down: ctx.mouse.middle_down,
            global_primary_down: ctx.mouse.global_primary_down,
            global_secondary_down: ctx.mouse.global_secondary_down,
            global_middle_down: ctx.mouse.global_middle_down,
        },
        config: ctx
            .config
            .iter()
            .map(|(key, value)| pet_api::ConfigEntry {
                key: key.to_owned(),
                value,
            })
            .collect(),
        theme_dark: matches!(ctx.theme, deskhud_engine::PetTheme::Dark),
        shadows: ctx.shadows,
    }
}

fn event_to_guest(event: PetEvent) -> Option<pet_api::Event> {
    use deskhud_engine::{PetKey, PetMouseButton};
    use pet_api::{Event, KeyValue, Modifiers, MouseButton};
    let modifiers = |m: deskhud_engine::PetModifiers| Modifiers {
        shift: m.shift,
        ctrl: m.ctrl,
        alt: m.alt,
        meta: m.meta,
    };
    let button = |b: PetMouseButton| match b {
        PetMouseButton::Primary => MouseButton::Primary,
        PetMouseButton::Secondary => MouseButton::Secondary,
        PetMouseButton::Middle => MouseButton::Middle,
    };
    let key = |k: PetKey| match k {
        PetKey::Function(n) => KeyValue::Function(n),
        PetKey::Letter(c) => KeyValue::Letter(c),
        PetKey::Digit(c) => KeyValue::Digit(c),
        PetKey::Punct(c) => KeyValue::Punct(c),
        PetKey::NumpadDigit(n) => KeyValue::NumpadDigit(n),
        PetKey::Escape => KeyValue::Named(pet_api::Key::Escape),
        PetKey::Tab => KeyValue::Named(pet_api::Key::Tab),
        PetKey::Enter => KeyValue::Named(pet_api::Key::Enter),
        PetKey::Space => KeyValue::Named(pet_api::Key::Space),
        PetKey::Backspace => KeyValue::Named(pet_api::Key::Backspace),
        PetKey::Delete => KeyValue::Named(pet_api::Key::Delete),
        PetKey::Insert => KeyValue::Named(pet_api::Key::Insert),
        PetKey::Clear => KeyValue::Named(pet_api::Key::Clear),
        PetKey::ArrowUp => KeyValue::Named(pet_api::Key::ArrowUp),
        PetKey::ArrowDown => KeyValue::Named(pet_api::Key::ArrowDown),
        PetKey::ArrowLeft => KeyValue::Named(pet_api::Key::ArrowLeft),
        PetKey::ArrowRight => KeyValue::Named(pet_api::Key::ArrowRight),
        PetKey::Home => KeyValue::Named(pet_api::Key::Home),
        PetKey::End => KeyValue::Named(pet_api::Key::End),
        PetKey::PageUp => KeyValue::Named(pet_api::Key::PageUp),
        PetKey::PageDown => KeyValue::Named(pet_api::Key::PageDown),
        PetKey::Shift => KeyValue::Named(pet_api::Key::Shift),
        PetKey::Ctrl => KeyValue::Named(pet_api::Key::Ctrl),
        PetKey::Alt => KeyValue::Named(pet_api::Key::Alt),
        PetKey::Super => KeyValue::Named(pet_api::Key::Super),
        PetKey::CapsLock => KeyValue::Named(pet_api::Key::CapsLock),
        PetKey::NumLock => KeyValue::Named(pet_api::Key::NumLock),
        PetKey::NumpadEnter => KeyValue::Named(pet_api::Key::NumpadEnter),
        PetKey::NumpadAdd => KeyValue::Named(pet_api::Key::NumpadAdd),
        PetKey::NumpadSubtract => KeyValue::Named(pet_api::Key::NumpadSubtract),
        PetKey::NumpadMultiply => KeyValue::Named(pet_api::Key::NumpadMultiply),
        PetKey::NumpadDivide => KeyValue::Named(pet_api::Key::NumpadDivide),
        PetKey::NumpadDecimal => KeyValue::Named(pet_api::Key::NumpadDecimal),
        PetKey::NumpadSeparator => KeyValue::Named(pet_api::Key::NumpadSeparator),
    };
    let pair = |b, m| (button(b), modifiers(m));
    Some(match event {
        PetEvent::DragStarted => Event::DragStarted,
        PetEvent::DragEnded { drag } => Event::DragEnded(pet_api::DragState {
            active: drag.active,
        }),
        PetEvent::DockChanged { from, to } => Event::DockChanged((
            pet_api::DockState {
                left: from.left,
                right: from.right,
                top: from.top,
                bottom: from.bottom,
            },
            pet_api::DockState {
                left: to.left,
                right: to.right,
                top: to.top,
                bottom: to.bottom,
            },
        )),
        PetEvent::MouseHover { inside } => Event::MouseHover(inside),
        PetEvent::MousePressed {
            button: b,
            modifiers: m,
        } => Event::MousePressed(pair(b, m)),
        PetEvent::MouseReleased {
            button: b,
            modifiers: m,
        } => Event::MouseReleased(pair(b, m)),
        PetEvent::MouseClicked {
            button: b,
            modifiers: m,
        } => Event::MouseClicked(pair(b, m)),
        PetEvent::MouseDoubleClicked {
            button: b,
            modifiers: m,
        } => Event::MouseDoubleClicked(pair(b, m)),
        PetEvent::MouseWheel {
            delta,
            modifiers: m,
        } => Event::MouseWheel((delta, modifiers(m))),
        PetEvent::GlobalMousePressed {
            button: b,
            modifiers: m,
        } => Event::GlobalMousePressed(pair(b, m)),
        PetEvent::GlobalMouseReleased {
            button: b,
            modifiers: m,
        } => Event::GlobalMouseReleased(pair(b, m)),
        PetEvent::GlobalMouseWheel {
            delta,
            modifiers: m,
        } => Event::GlobalMouseWheel((delta, modifiers(m))),
        PetEvent::GlobalKeyPressed {
            key: k,
            modifiers: m,
        } => Event::GlobalKeyPressed((key(k), modifiers(m))),
        PetEvent::GlobalKeyReleased {
            key: k,
            modifiers: m,
        } => Event::GlobalKeyReleased((key(k), modifiers(m))),
        PetEvent::KeyPressed {
            key: k,
            modifiers: m,
        } => Event::KeyPressed((key(k), modifiers(m))),
        PetEvent::KeyReleased {
            key: k,
            modifiers: m,
        } => Event::KeyReleased((key(k), modifiers(m))),
    })
}

fn scene_to_engine(scene: pet_api::Scene) -> Result<PetScene, RuntimeError> {
    use deskhud_engine::{AssetId, Path, SceneItem, SceneNode, Transform2D};
    let items = scene
        .items
        .into_iter()
        .map(|item| {
            let transform = Transform2D {
                translation: [item.transform.translation.0, item.transform.translation.1],
                rotation_radians: item.transform.rotation_radians,
                scale: [item.transform.scale.0, item.transform.scale.1],
            };
            let node = match item.node {
                pet_api::Node::Sprite((asset, size, opacity)) => SceneNode::Sprite {
                    asset: AssetId(asset),
                    size: [size.0, size.1],
                    opacity,
                },
                pet_api::Node::AtlasFrame((asset, frame, size, opacity)) => SceneNode::AtlasFrame {
                    asset: AssetId(asset),
                    frame,
                    size: [size.0, size.1],
                    opacity,
                },
                pet_api::Node::Path(path) => SceneNode::Path(Path {
                    points: path.points.into_iter().map(|p| [p.0, p.1]).collect(),
                    closed: path.closed,
                    fill: path.fill.map(color_to_engine),
                    stroke: path.stroke.map(color_to_engine),
                    stroke_width: path.stroke_width,
                }),
                pet_api::Node::GradientPath((path, top_color, bottom_color)) => {
                    SceneNode::GradientPath {
                        path: Path {
                            points: path.points.into_iter().map(|p| [p.0, p.1]).collect(),
                            closed: path.closed,
                            fill: path.fill.map(color_to_engine),
                            stroke: path.stroke.map(color_to_engine),
                            stroke_width: path.stroke_width,
                        },
                        top_color: color_to_engine(top_color),
                        bottom_color: color_to_engine(bottom_color),
                    }
                }
                pet_api::Node::Shape((shape, color)) => SceneNode::Shape {
                    shape: shape_to_engine(shape),
                    color: color_to_engine(color),
                },
                pet_api::Node::Text((text, color, size)) => SceneNode::Text {
                    text,
                    color: color_to_engine(color),
                    size,
                },
                pet_api::Node::Bubble((text, color, background, corner_radius)) => {
                    SceneNode::Bubble {
                        text,
                        color: color_to_engine(color),
                        background: color_to_engine(background),
                        corner_radius,
                    }
                }
                pet_api::Node::HitRegion(shape) => SceneNode::HitRegion {
                    shape: shape_to_engine(shape),
                },
            };
            SceneItem {
                transform,
                z_index: item.z_index,
                node,
            }
        })
        .collect();
    let scene = PetScene { items };
    scene
        .validate()
        .map_err(|e| RuntimeError::Wasm(format!("invalid guest scene: {e:?}")))?;
    Ok(scene)
}

fn color_to_engine(c: pet_api::Color) -> [f32; 4] {
    [c.r, c.g, c.b, c.a]
}

fn shape_to_engine(shape: pet_api::Shape) -> deskhud_engine::Shape {
    match shape {
        pet_api::Shape::Circle(radius) => deskhud_engine::Shape::Circle { radius },
        pet_api::Shape::Rect((size, radius)) => deskhud_engine::Shape::Rect {
            size: [size.0, size.1],
            corner_radius: radius,
        },
        pet_api::Shape::Ellipse(radii) => deskhud_engine::Shape::Ellipse {
            radii: [radii.0, radii.1],
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_non_component_bytes_without_panicking() {
        let error = load_wasm_guest(b"not a wasm component").expect_err("invalid component");
        assert!(matches!(error, RuntimeError::Wasm(_)));
    }

    #[test]
    fn default_limits_are_bounded() {
        let limits = WasmLimits::default();
        assert!(limits.fuel_per_call > 0);
        assert!(limits.max_memory_bytes <= 16 * 1024 * 1024);
        assert!(limits.call_timeout <= Duration::from_millis(25));
    }

    #[test]
    fn converts_guest_metadata_and_package_preview() {
        let (info, options) = info_to_engine(
            pet_api::PetInfo {
                id: "pet.example.test".into(),
                display_name: "Test Pet".into(),
                description: "test".into(),
                author: "DeskHud".into(),
                homepage: None,
                version: "1.0.0".into(),
                engine: "0.6".into(),
                window_width: 96.0,
                window_height: 96.0,
                config_options: vec![pet_api::ConfigOption {
                    key: "enabled".into(),
                    label: "Enabled".into(),
                    description: "Enable the pet".into(),
                    default: true,
                }],
            },
            Some(vec![1, 2, 3]),
        )
        .expect("valid guest metadata");
        assert_eq!(info.id, "pet.example.test");
        assert_eq!(info.preview, Some([1, 2, 3].as_slice()));
        assert_eq!(options.len(), 1);
        assert_eq!(options[0].key, "enabled");
    }

    #[test]
    fn rejects_invalid_guest_dimensions_and_duplicate_options() {
        let mut info = pet_api::PetInfo {
            id: "pet.example.test".into(),
            display_name: "Test Pet".into(),
            description: "test".into(),
            author: "DeskHud".into(),
            homepage: None,
            version: "1.0.0".into(),
            engine: "0.8".into(),
            window_width: 0.0,
            window_height: 96.0,
            config_options: vec![],
        };
        assert!(info_to_engine(info.clone(), None).is_err());
        info.window_width = 96.0;
        info.config_options = vec![
            pet_api::ConfigOption {
                key: "same".into(),
                label: "One".into(),
                description: "one".into(),
                default: true,
            },
            pet_api::ConfigOption {
                key: "same".into(),
                label: "Two".into(),
                description: "two".into(),
                default: false,
            },
        ];
        assert!(info_to_engine(info, None).is_err());
    }
}
