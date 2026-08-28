//! Platform-neutral native UI host contracts.
//!
//! This crate deliberately contains only data that can cross a platform
//! boundary. Concrete backends own handles, widgets, event loops, and their
//! platform-specific error types.

use std::collections::BTreeMap;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct WindowId(pub u64);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowRole {
    Settings,
    Pet,
    Overlay,
    HudLayout,
    Menu,
    Dialog,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostCommand {
    OpenSettings,
    ToggleTopmost,
    ToggleHud,
    EnterHudLayout,
    ExitHudLayout,
    Quit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WindowLevel {
    Normal,
    AlwaysOnTop,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayInfo {
    pub bounds: Rect,
    pub work_area: Rect,
    pub scale: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputKind {
    Pointer,
    Keyboard,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PlatformEvent {
    WindowClosed(WindowId),
    Input(InputKind),
    QuitRequested,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlatformError {
    pub message: String,
}

impl PlatformError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}
impl fmt::Display for PlatformError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for PlatformError {}

pub trait SettingsHost {
    fn open_settings(&mut self) -> Result<(), PlatformError>;
}
pub trait OverlayHost {
    fn create_overlay(&mut self, role: WindowRole) -> Result<WindowId, PlatformError>;
    fn set_overlay_visible(&mut self, id: WindowId, visible: bool) -> Result<(), PlatformError>;
    fn set_overlay_level(&mut self, id: WindowId, level: WindowLevel) -> Result<(), PlatformError>;
}
pub trait MenuHost {
    fn show_menu(&mut self, anchor: Rect) -> Result<(), PlatformError>;
}
pub trait WindowHost {
    fn close_window(&mut self, id: WindowId) -> Result<(), PlatformError>;
}
pub trait DisplayHost {
    fn displays(&self) -> Result<Vec<DisplayInfo>, PlatformError>;
}
pub trait InputHost {
    fn poll_events(&mut self) -> Result<Vec<PlatformEvent>, PlatformError>;
}

/// Deterministic host state shared by backend smoke implementations.
///
/// Native crates can replace the storage and event-loop plumbing without
/// changing the six capability contracts or exposing an OS handle upstream.
#[derive(Default)]
pub struct HostState {
    next_window: u64,
    windows: BTreeMap<WindowId, WindowRecord>,
    displays: Vec<DisplayInfo>,
    events: Vec<PlatformEvent>,
    pub last_menu_anchor: Option<Rect>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WindowRecord {
    pub role: WindowRole,
    pub visible: bool,
    pub level: WindowLevel,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct HudWindowState {
    pub enabled: bool,
    pub layout_mode: bool,
    pub topmost: bool,
}

impl HudWindowState {
    pub fn apply(&mut self, command: HostCommand) -> bool {
        match command {
            HostCommand::ToggleHud => {
                self.enabled = !self.enabled;
                true
            }
            HostCommand::EnterHudLayout if self.enabled => {
                self.layout_mode = true;
                true
            }
            HostCommand::ExitHudLayout if self.layout_mode => {
                self.layout_mode = false;
                true
            }
            HostCommand::ToggleTopmost => {
                self.topmost = !self.topmost;
                true
            }
            _ => false,
        }
    }
}

impl HostState {
    pub fn with_displays(displays: Vec<DisplayInfo>) -> Self {
        Self {
            displays,
            ..Self::default()
        }
    }

    pub fn window(&self, id: WindowId) -> Option<WindowRecord> {
        self.windows.get(&id).copied()
    }
    pub fn window_ids(&self) -> Vec<WindowId> {
        self.windows.keys().copied().collect()
    }
    pub fn push_event(&mut self, event: PlatformEvent) {
        self.events.push(event);
    }

    pub fn create_overlay(&mut self, role: WindowRole) -> WindowId {
        self.next_window = self.next_window.saturating_add(1);
        let id = WindowId(self.next_window);
        self.windows.insert(
            id,
            WindowRecord {
                role,
                visible: false,
                level: WindowLevel::Normal,
            },
        );
        id
    }

    pub fn set_overlay_visible(
        &mut self,
        id: WindowId,
        visible: bool,
    ) -> Result<(), PlatformError> {
        self.window_mut(id).map(|window| window.visible = visible)
    }

    pub fn set_overlay_level(
        &mut self,
        id: WindowId,
        level: WindowLevel,
    ) -> Result<(), PlatformError> {
        self.window_mut(id).map(|window| window.level = level)
    }

    pub fn close_window(&mut self, id: WindowId) -> Result<(), PlatformError> {
        if self.windows.remove(&id).is_some() {
            self.events.push(PlatformEvent::WindowClosed(id));
            Ok(())
        } else {
            Err(PlatformError::new(format!("unknown window {}", id.0)))
        }
    }

    pub fn show_menu(&mut self, anchor: Rect) {
        self.last_menu_anchor = Some(anchor);
    }
    pub fn displays(&self) -> Vec<DisplayInfo> {
        self.displays.clone()
    }
    pub fn poll_events(&mut self) -> Vec<PlatformEvent> {
        std::mem::take(&mut self.events)
    }

    fn window_mut(&mut self, id: WindowId) -> Result<&mut WindowRecord, PlatformError> {
        self.windows
            .get_mut(&id)
            .ok_or_else(|| PlatformError::new(format!("unknown window {}", id.0)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neutral_geometry_and_events_are_platform_independent() {
        let display = DisplayInfo {
            bounds: Rect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1080.0,
            },
            work_area: Rect {
                x: 0.0,
                y: 0.0,
                width: 1920.0,
                height: 1040.0,
            },
            scale: 1.0,
        };
        assert_eq!(display.work_area.width, 1920.0);
        assert_eq!(
            PlatformEvent::WindowClosed(WindowId(7)),
            PlatformEvent::WindowClosed(WindowId(7))
        );
    }

    #[test]
    fn host_state_models_window_lifecycle_and_event_polling() {
        let mut state = HostState::default();
        let id = state.create_overlay(WindowRole::Pet);
        assert_eq!(state.window(id).map(|window| window.visible), Some(false));
        state
            .set_overlay_visible(id, true)
            .expect("created window must be addressable");
        state
            .set_overlay_level(id, WindowLevel::AlwaysOnTop)
            .expect("created window must be addressable");
        assert_eq!(
            state.window(id).map(|window| window.level),
            Some(WindowLevel::AlwaysOnTop)
        );
        state
            .close_window(id)
            .expect("created window must be closable");
        assert_eq!(state.poll_events(), vec![PlatformEvent::WindowClosed(id)]);
        assert!(state.close_window(id).is_err());
    }
}
