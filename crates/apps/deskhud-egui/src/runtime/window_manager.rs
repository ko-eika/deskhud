//! 应用窗口总调度器。
//!
//! 具体视口的创建、事件处理和绘制封装在 [`crate::views`] 与 [`super::viewport`] 下，
//! 本模块只负责视口之间的协调和生命周期管理。
#![cfg_attr(target_os = "macos", allow(dead_code))]

use deskhud_engine::{EngineRegistry, HudSourceId, PetEvent, PetKeyTracker};
use deskhud_runtime::{bootstrap_registry, build_catalog_store};
use deskhud_ui::{
    CatalogStore, LayerPreference, PrefsWriteOrder, UiPreferences, load, save, save_ordered,
};
use std::sync::Arc;
use winit::{
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use super::viewport::UserEvent;
use crate::views::{
    bubble::PetBubbleWindow,
    hud::HudWindow,
    pet::{PetMenu, PetMenuAction, PetWindow},
    setting::SettingsWindow,
};

pub(crate) struct WindowManager {
    /// 用于唤醒 winit 主线程的事件代理。
    proxy: EventLoopProxy<UserEvent>,
    /// Pet 主窗口。
    pet: Option<PetWindow>,
    bubble: Option<PetBubbleWindow>,
    /// HUD 窗口。
    hud: Option<HudWindow>,
    /// Settings 窗口。
    settings: Option<SettingsWindow>,
    /// 右键菜单及其子菜单窗口树。
    menu: Option<PetMenu>,
    registry: Arc<EngineRegistry>,
    catalogs: CatalogStore,
    prefs: UiPreferences,
    frame_rate: u32,
    key_tracker: PetKeyTracker,
}

impl WindowManager {
    pub(crate) fn new(proxy: EventLoopProxy<UserEvent>) -> Self {
        let bootstrap = bootstrap_registry();
        tracing::info!(path = ?deskhud_ui::prefs_path(), "loading preferences");
        let (mut prefs, loaded_from_disk) = match load() {
            Ok(prefs) => {
                tracing::info!("preferences loaded");
                (prefs, true)
            }
            Err(error) => {
                tracing::warn!(%error, "load preferences failed; using defaults");
                (UiPreferences::default(), false)
            }
        };
        let rewrite_hud_model = !prefs.hud.is_model_format_current();
        let migrated_instances = prefs.hud.ensure_default_instances(
            bootstrap.registry.all_hud_contributions().into_iter().map(
                |(plugin_id, contribution)| {
                    (
                        HudSourceId::new(plugin_id, contribution.id),
                        contribution.default_enabled,
                    )
                },
            ),
        );
        if migrated_instances > 0 {
            tracing::info!(
                migrated_instances,
                "mapped HUD contributions to stable instances"
            );
        }
        if !bootstrap
            .registry
            .pets()
            .iter()
            .any(|pet| pet.info().id == prefs.pet.kind)
        {
            prefs.pet.kind = bootstrap.registry.active_pet_id().to_owned();
        }
        if (migrated_instances > 0 || rewrite_hud_model) && loaded_from_disk {
            prefs.hud.mark_model_format_current();
            if let Err(error) = save(&prefs) {
                tracing::warn!(%error, "failed to persist formatted HUD instances");
            }
        }
        // Keep the persisted pet size. Pack metadata is used when a pet is
        // selected in Settings; applying it during startup would silently
        // overwrite the user's saved preset on every launch.
        let catalogs = build_catalog_store(&bootstrap.discovered, prefs.locale);
        let frame_rate = super::render::frame_rate_for(&prefs.graphics);
        Self {
            proxy,
            pet: None,
            bubble: None,
            hud: None,
            settings: None,
            menu: None,
            registry: Arc::new(bootstrap.registry),
            catalogs,
            prefs,
            frame_rate,
            key_tracker: PetKeyTracker::default(),
        }
    }

    pub(crate) fn create_pet(&mut self, event_loop: &ActiveEventLoop) {
        // 所有视口随应用一起创建，显示和隐藏只改变原生窗口的可见状态。
        if self.pet.is_none() {
            self.pet = Some(PetWindow::create(
                event_loop,
                &self.proxy,
                self.registry.clone(),
                self.prefs.clone(),
            ));
        }
        if self.hud.is_none() {
            self.hud = Some(HudWindow::create(
                event_loop,
                &self.proxy,
                self.registry.clone(),
                self.prefs.clone(),
            ));
        }
        if self.bubble.is_none() {
            let mut bubble = PetBubbleWindow::create(event_loop, &self.proxy);
            if let Some(pet) = self.pet.as_ref() {
                bubble.set_window_layer(pet.window_layer());
            }
            self.bubble = Some(bubble);
        }
        if self.settings.is_none() {
            self.settings = Some(SettingsWindow::create(
                event_loop,
                &self.proxy,
                self.registry.clone(),
                self.catalogs.clone(),
                self.prefs.clone(),
            ));
        }
        self.sync_hud_visibility();
        if self.menu.is_none() {
            self.menu = Some(PetMenu::create(event_loop, &self.proxy));
        }
    }

    pub(crate) fn window_handles(&self) -> Vec<(WindowId, std::sync::Arc<winit::window::Window>)> {
        let mut handles = Vec::new();
        if let Some(pet) = &self.pet {
            handles.push((pet.window_id(), pet.window_handle()));
        }
        if let Some(bubble) = &self.bubble {
            handles.push((bubble.window_id(), bubble.window_handle()));
        }
        if let Some(hud) = &self.hud {
            handles.push((hud.window_id(), hud.window_handle()));
        }
        if let Some(settings) = &self.settings {
            handles.push((settings.window_id(), settings.window_handle()));
        }
        if let Some(menu) = &self.menu {
            handles.extend(menu.window_handles());
        }
        handles
    }

    /// 返回宠物主窗口和其对话气泡窗口的标识，供 winit 主线程直接同步几何位置。
    pub(crate) fn pet_and_bubble_window_ids(&self) -> Option<(WindowId, WindowId)> {
        Some((
            self.pet.as_ref()?.window_id(),
            self.bubble.as_ref()?.window_id(),
        ))
    }

    fn show_hud(&mut self) {
        if self.hud.is_none() {
            return;
        }
        let hud = self.hud.as_mut().expect("HUD viewport disappeared");
        hud.show();
    }

    fn show_settings(&mut self) {
        if self.settings.is_none() {
            return;
        }
        let settings = self
            .settings
            .as_mut()
            .expect("settings viewport disappeared");
        settings.show(&self.prefs);
        #[cfg(target_os = "macos")]
        self.set_dock_icon(true);
    }

    fn hide_hud(&mut self) {
        if let Some(hud) = self.hud.as_mut() {
            hud.hide();
        }
    }

    fn hide_settings(&mut self) {
        if let Some(settings) = self.settings.as_mut() {
            // Closing discards the draft; only the native window preset is kept.
            let geometry = settings.geometry();
            self.prefs.shell.settings_width = geometry[0];
            self.prefs.shell.settings_height = geometry[1];
            self.prefs.shell.settings_pos_x = geometry[2];
            self.prefs.shell.settings_pos_y = geometry[3];
            self.save_geometry();
        }
        if let Some(settings) = self.settings.as_mut() {
            settings.hide();
        }
        #[cfg(target_os = "macos")]
        self.set_dock_icon(false);
    }

    fn commit_preferences(&mut self, prefs: deskhud_ui::UiPreferences) {
        tracing::info!("settings applied; saving preferences");
        let mut prefs = prefs;
        // The settings model is intentionally unaware of live native window
        // movement. Preserve the position captured from the pet window so an
        // unrelated Apply cannot move the pet back to an older snapshot.
        if let Some(pet) = self.pet.as_ref()
            && let Some(position) = pet.current_position()
        {
            prefs.pet.set_pos(position.x as f32, position.y as f32);
        }
        self.prefs = prefs;
        let _ = self.proxy.send_event(UserEvent::SetGlobalInputMonitoring {
            keyboard: self.prefs.pet.global_keyboard_input,
            mouse: self.prefs.pet.global_mouse_input,
        });
        if !self.prefs.pet.bubbles {
            // 设置页关闭总开关时立即收起已存在的独立气泡，不等待下一次场景帧。
            if let Some(bubble) = self.bubble.as_mut() {
                bubble.hide();
            }
        }
        self.frame_rate = super::render::frame_rate_for(&self.prefs.graphics);
        if let Some(pet) = self.pet.as_mut() {
            pet.apply_preferences(&self.registry, self.prefs.clone());
            if let Some(bubble) = self.bubble.as_mut() {
                bubble.set_window_layer(pet.window_layer());
            }
        }
        if let Some(hud) = self.hud.as_mut() {
            hud.apply_preferences(self.prefs.clone());
        }
        self.sync_hud_visibility();
        self.save_preferences();
    }

    /// Keeps the native HUD window visibility derived from the same persisted
    /// switches that gate individual contributions.
    fn sync_hud_visibility(&mut self) {
        let should_show = crate::views::hud::has_active_hud(&self.registry, &self.prefs);
        if should_show {
            self.show_hud();
        } else {
            self.hide_hud();
        }
    }

    fn save_geometry(&mut self) {
        self.save_preferences();
    }

    /// Saves the single applied preference snapshot. All persistence is
    /// serialized here so pet movement cannot overwrite newer settings with
    /// a stale PetWindow-local clone.
    fn save_preferences(&mut self) {
        if let Some(position) = self.pet.as_ref().and_then(PetWindow::current_position) {
            self.prefs.pet.set_pos(position.x as f32, position.y as f32);
        }
        self.prefs.shell.ui_font_id =
            crate::fonts::persistable_font_id(&self.prefs.shell.ui_font_id);
        let order = PrefsWriteOrder {
            pet_ids: self
                .registry
                .pet_infos()
                .into_iter()
                .map(|p| p.id.to_owned())
                .collect(),
            pet_option_keys: self
                .registry
                .pets()
                .into_iter()
                .map(|p| {
                    (
                        p.info().id.to_owned(),
                        p.config_options()
                            .iter()
                            .map(|o| o.key.to_owned())
                            .collect(),
                    )
                })
                .collect(),
            plugin_ids: self
                .registry
                .plugin_infos()
                .into_iter()
                .map(|p| p.id.to_owned())
                .collect(),
            plugin_contrib_ids: Vec::new(),
        };
        if let Err(error) = save_ordered(&self.prefs, &order) {
            tracing::warn!(%error, path = ?deskhud_ui::prefs_path(), "save preferences failed");
        } else {
            tracing::info!(path = ?deskhud_ui::prefs_path(), "preferences saved");
        }
    }

    pub(crate) fn frame_rate(&self) -> u32 {
        self.frame_rate
    }

    #[cfg(target_os = "macos")]
    fn set_dock_icon(&self, visible: bool) {
        let _ = self.proxy.send_event(UserEvent::WindowCommand {
            window_id: self.settings.as_ref().map_or_else(
                || {
                    self.pet
                        .as_ref()
                        .expect("pet viewport disappeared")
                        .window_id()
                },
                SettingsWindow::window_id,
            ),
            command: super::render::WindowCommand::SetDockIcon { visible },
        });
    }

    fn show_pet_menu(&mut self) {
        let Some(pet) = self.pet.as_ref() else {
            return;
        };
        let Some(anchor) = pet.cursor_screen_position() else {
            return;
        };
        if self.menu.is_none() {
            return;
        }
        self.menu.as_mut().expect("pet menu disappeared").open(
            anchor,
            &self.prefs,
            pet.window_layer(),
            self.hud.as_ref().map_or(
                super::viewport::WindowLayer::Normal,
                HudWindow::window_layer,
            ),
            self.prefs.hud.is_master_enabled(),
        );
    }

    fn hide_menu(&mut self) {
        if let Some(menu) = self.menu.as_mut() {
            menu.close();
        }
    }

    fn viewport_for(&self, window_id: WindowId) -> ViewportKind {
        if self
            .pet
            .as_ref()
            .is_some_and(|v| v.window_id() == window_id)
        {
            ViewportKind::Pet
        } else if self
            .bubble
            .as_ref()
            .is_some_and(|v| v.window_id() == window_id)
        {
            ViewportKind::Bubble
        } else if self
            .hud
            .as_ref()
            .is_some_and(|v| v.window_id() == window_id)
        {
            ViewportKind::Hud
        } else if self
            .settings
            .as_ref()
            .is_some_and(|v| v.window_id() == window_id)
        {
            ViewportKind::Settings
        } else if self
            .menu
            .as_ref()
            .is_some_and(|v| v.contains_window(window_id))
        {
            ViewportKind::Menu
        } else {
            ViewportKind::Unknown
        }
    }

    /// 返回该原生窗口是否为宠物主窗口。
    ///
    /// 原生拖动时，宠物几何变化需要立即带动气泡；其它窗口（尤其是气泡
    /// 自身）移动则不应触发额外渲染，避免形成重绘循环。
    pub(crate) fn is_pet_window(&self, window_id: WindowId) -> bool {
        self.pet
            .as_ref()
            .is_some_and(|pet| pet.window_id() == window_id)
    }

    /// 将平台采集后的中性全局输入事件交给当前宠物实例。
    pub(crate) fn dispatch_pet_event(&mut self, event: PetEvent) {
        if matches!(
            event,
            PetEvent::GlobalKeyPressed { .. } | PetEvent::GlobalKeyReleased { .. }
        ) {
            tracing::info!(?event, "global keyboard event dispatched to pet");
        } else {
            tracing::debug!(?event, "global input event dispatched to pet");
        }
        if let Some(pet) = self.pet.as_ref() {
            pet.dispatch_event(event);
        }
        match event {
            PetEvent::GlobalKeyPressed { key, modifiers } => {
                if let Some(combo) = self.key_tracker.press(key, modifiers)
                    && let Some(pet) = self.pet.as_ref()
                {
                    pet.dispatch_event(combo);
                }
            }
            PetEvent::GlobalKeyReleased { modifiers, .. } => {
                self.key_tracker.release(modifiers);
            }
            _ => {}
        }
    }

    #[cfg(any(target_os = "windows", target_os = "macos"))]
    pub(crate) fn global_input_monitoring(&self) -> (bool, bool) {
        (
            self.prefs.pet.global_keyboard_input,
            self.prefs.pet.global_mouse_input,
        )
    }

    /// 将原生输入事件路由到对应的视口控制器。
    pub(crate) fn handle_event(&mut self, window_id: WindowId, event: WindowEvent) -> bool {
        match self.viewport_for(window_id) {
            ViewportKind::Pet => {
                if is_window_close(&event) {
                    return true;
                }
                if matches!(
                    event,
                    WindowEvent::MouseInput {
                        state: ElementState::Pressed,
                        ..
                    }
                ) {
                    self.hide_menu();
                }
                let open_menu = is_right_click(&event);
                let local_key_event = match &event {
                    WindowEvent::KeyboardInput { event, .. } => {
                        let key = match event.physical_key {
                            winit::keyboard::PhysicalKey::Code(code) => {
                                crate::input::winit_key_to_pet_key(code)
                            }
                            winit::keyboard::PhysicalKey::Unidentified(_) => None,
                        };
                        key.map(|key| {
                            if event.state == ElementState::Pressed {
                                PetEvent::GlobalKeyPressed {
                                    key,
                                    modifiers: self
                                        .pet
                                        .as_ref()
                                        .expect("pet viewport disappeared")
                                        .local_modifiers(),
                                }
                            } else {
                                PetEvent::GlobalKeyReleased {
                                    key,
                                    modifiers: self
                                        .pet
                                        .as_ref()
                                        .expect("pet viewport disappeared")
                                        .local_modifiers(),
                                }
                            }
                        })
                    }
                    _ => None,
                };
                self.pet
                    .as_mut()
                    .expect("pet viewport disappeared")
                    .handle_event(&event);
                if let Some(event) = local_key_event {
                    tracing::info!(?event, "focused window keyboard event normalized");
                    self.dispatch_pet_event(event);
                }
                if open_menu {
                    self.show_pet_menu();
                }
            }
            ViewportKind::Bubble => {
                if !is_window_close(&event) {
                    self.bubble
                        .as_mut()
                        .expect("bubble viewport disappeared")
                        .handle_event(&event);
                }
            }
            ViewportKind::Hud => self.handle_secondary_event(event, true),
            ViewportKind::Settings => self.handle_secondary_event(event, false),
            ViewportKind::Menu => {
                self.menu
                    .as_mut()
                    .expect("pet menu disappeared")
                    .handle_event(window_id, &event);
                // 菜单的 hover/click 状态必须在输入事件后立即进入下一帧，
                // 不依赖 egui 重绘回调是否已经发出代理事件。
            }
            ViewportKind::Unknown => {}
        }
        false
    }

    fn handle_secondary_event(&mut self, event: WindowEvent, hud: bool) {
        if matches!(
            event,
            WindowEvent::MouseInput {
                state: ElementState::Pressed,
                ..
            }
        ) {
            // 菜单是独立窗口；点击 HUD 或 Settings 时也应关闭它。
            self.hide_menu();
        }
        if is_window_close(&event) {
            if hud {
                self.hide_hud();
            } else {
                self.hide_settings();
            }
        } else if !matches!(event, WindowEvent::RedrawRequested) {
            if hud {
                self.hud
                    .as_mut()
                    .expect("HUD viewport disappeared")
                    .handle_event(&event);
            } else {
                self.settings
                    .as_mut()
                    .expect("settings viewport disappeared")
                    .handle_event(&event);
            }
        }
        if !hud && matches!(event, WindowEvent::Moved(_) | WindowEvent::Resized(_)) {
            if let Some(settings) = self.settings.as_mut() {
                let geometry = settings.geometry();
                self.prefs.shell.settings_width = geometry[0];
                self.prefs.shell.settings_height = geometry[1];
                self.prefs.shell.settings_pos_x = geometry[2];
                self.prefs.shell.settings_pos_y = geometry[3];
                self.save_geometry();
            }
        }
    }

    /// 绘制指定窗口对应的视口，并处理该视口产生的关闭或菜单动作。
    pub(crate) fn render_window(&mut self, window_id: WindowId) -> bool {
        match self.viewport_for(window_id) {
            ViewportKind::Pet => {
                let should_close = {
                    let pet = self.pet.as_mut().expect("pet viewport disappeared");
                    let should_close = pet.render();
                    if pet.take_position_dirty() {
                        tracing::info!("pet position changed; saving preferences");
                        self.save_preferences();
                    }
                    should_close
                };
                if let (Some(pet), Some(bubble)) = (self.pet.as_ref(), self.bubble.as_mut()) {
                    if let Some(anchor) = pet.bubble_anchor() {
                        let was_visible = bubble.is_visible();
                        let content = pet.bubble_content().map(|mut content| {
                            content.text = content
                                .text
                                .split('+')
                                .map(|part| {
                                    let part = part.trim();
                                    #[cfg(target_os = "macos")]
                                    let part = match part {
                                        "InputKey.Super" => "InputKey.Command",
                                        "InputKey.Alt" => "InputKey.Option",
                                        other => other,
                                    };
                                    self.catalogs.t(self.prefs.locale, part, part)
                                })
                                .collect::<Vec<_>>()
                                .join(" + ");
                            content
                        });
                        let content_changed = bubble.update(content, anchor, &self.prefs);
                        if content_changed || (!was_visible && bubble.is_visible()) {
                            tracing::info!(
                                visible_before = was_visible,
                                visible_after = bubble.is_visible(),
                                content_changed,
                                anchor = ?anchor,
                                "pet bubble content updated"
                            );
                        }
                        if content_changed || (!was_visible && bubble.is_visible()) {
                            // The bubble is added to the next render list only
                            // after this Pet pass. Render immediately so both
                            // newly shown and already visible bubbles reflect
                            // event-triggered text changes in the same frame.
                            bubble.render();
                        }
                    } else {
                        // 无法读取宠物窗口坐标时宁可暂时不显示，也不能把气泡
                        // 放到左上角这一错误位置。
                        bubble.hide();
                    }
                }
                should_close
            }
            ViewportKind::Bubble => {
                self.bubble
                    .as_mut()
                    .expect("bubble viewport disappeared")
                    .render();
                false
            }
            ViewportKind::Hud => {
                if self.hud.as_ref().is_some_and(HudWindow::is_visible) {
                    let (should_close, changed) = self
                        .hud
                        .as_mut()
                        .expect("HUD viewport disappeared")
                        .should_close();
                    if let Some(prefs) = changed {
                        // Layout edits are committed on every frame. Keep the HUD's
                        // live editor state intact; applying the whole snapshot here
                        // would clear the positions currently being dragged.
                        self.prefs.hud = prefs.hud;
                        self.save_preferences();
                    }
                    if should_close {
                        self.hide_hud();
                    }
                }
                false
            }
            ViewportKind::Settings => {
                let (should_close, applied) = if self
                    .settings
                    .as_ref()
                    .is_some_and(SettingsWindow::is_visible)
                {
                    self.settings
                        .as_mut()
                        .expect("settings viewport disappeared")
                        .render()
                } else {
                    (false, None)
                };
                if let Some(prefs) = applied {
                    self.commit_preferences(prefs);
                }
                if should_close {
                    self.hide_settings();
                }
                false
            }
            ViewportKind::Menu => {
                let layer = self.pet.as_ref().map_or(
                    super::viewport::WindowLayer::Normal,
                    PetWindow::window_layer,
                );
                let (action, should_close) =
                    self.menu.as_mut().expect("pet menu disappeared").render(
                        layer,
                        self.hud.as_ref().map_or(
                            super::viewport::WindowLayer::Normal,
                            HudWindow::window_layer,
                        ),
                        self.prefs.hud.is_master_enabled(),
                        &self.prefs,
                    );
                if should_close {
                    self.hide_menu();
                }
                action.is_some_and(|action| self.handle_menu_action(action))
            }
            ViewportKind::Unknown => false,
        }
    }

    /// 统一绘制所有当前可见视口。
    ///
    /// HUD、Settings 和 Pet 的动画/布局状态存在跨窗口联动，
    /// 因此不能只依赖单个 RedrawRequested 对应的窗口。
    pub(crate) fn render_all(&mut self) -> bool {
        let mut window_ids = Vec::new();
        if let Some(pet) = &self.pet {
            window_ids.push(pet.window_id());
        }
        if self.hud.as_ref().is_some_and(HudWindow::is_visible) {
            window_ids.push(
                self.hud
                    .as_ref()
                    .expect("HUD viewport disappeared")
                    .window_id(),
            );
        }
        if self
            .bubble
            .as_ref()
            .is_some_and(PetBubbleWindow::is_visible)
        {
            window_ids.push(
                self.bubble
                    .as_ref()
                    .expect("bubble viewport disappeared")
                    .window_id(),
            );
        }
        if self
            .settings
            .as_ref()
            .is_some_and(SettingsWindow::is_visible)
        {
            window_ids.push(
                self.settings
                    .as_ref()
                    .expect("settings viewport disappeared")
                    .window_id(),
            );
        }
        if self.menu.as_ref().is_some_and(PetMenu::is_visible) {
            window_ids.push(
                self.menu
                    .as_ref()
                    .expect("pet menu disappeared")
                    .window_id(),
            );
        }
        let should_close = window_ids
            .into_iter()
            .any(|window_id| self.render_window(window_id));
        if let Some(hud) = self.hud.as_mut() {
            hud.maintain_surface();
        }
        if let Some(settings) = self.settings.as_mut() {
            settings.maintain_surface();
        }
        should_close
    }

    fn handle_menu_action(&mut self, action: PetMenuAction) -> bool {
        match action {
            PetMenuAction::SetPetLayer(layer) => {
                if let Some(pet) = self.pet.as_mut() {
                    pet.set_window_layer(layer);
                    if let Some(bubble) = self.bubble.as_mut() {
                        bubble.set_window_layer(layer);
                    }
                }
                self.prefs.pet.layer = layer_preference(layer);
                if let Some(settings) = self.settings.as_mut() {
                    settings.sync_pet_layer(self.prefs.pet.layer);
                }
                self.save_geometry();
            }
            PetMenuAction::OpenSettings => self.show_settings(),
            PetMenuAction::OpenHud => {
                let enabled = !self.prefs.hud.is_master_enabled();
                self.prefs.hud.set_master_enabled(enabled);
                if let Some(settings) = self.settings.as_mut() {
                    settings.sync_hud_master_enabled(enabled);
                }
                self.sync_hud_visibility();
                self.save_preferences();
            }
            PetMenuAction::HudLayout => {
                if let Some(hud) = self.hud.as_mut() {
                    hud.enter_layout_mode();
                }
            }
            PetMenuAction::SetHudLayer(layer) => {
                if let Some(hud) = self.hud.as_mut() {
                    hud.set_window_layer(layer);
                }
                self.prefs.hud.layer = layer_preference(layer);
                if let Some(settings) = self.settings.as_mut() {
                    settings.sync_hud_layer(self.prefs.hud.layer);
                }
                self.save_geometry();
            }
            PetMenuAction::ExitApplication => return true,
        }
        false
    }

    pub(crate) fn destroy_all(&mut self) {
        if let Some(pet) = self.pet.as_mut() {
            pet.sync_position();
        }
        if let Some(position) = self.pet.as_ref().and_then(PetWindow::current_position) {
            self.prefs.pet.set_pos(position.x as f32, position.y as f32);
        }
        if let Some(pet) = self.pet.as_mut() {
            pet.destroy();
        }
        if let Some(bubble) = self.bubble.as_mut() {
            bubble.destroy();
        }
        if let Some(hud) = self.hud.as_mut() {
            hud.destroy();
        }
        if let Some(settings) = self.settings.as_mut() {
            settings.destroy();
        }
        if let Some(menu) = self.menu.as_mut() {
            menu.destroy();
        }
    }
}

fn layer_preference(layer: super::viewport::WindowLayer) -> LayerPreference {
    match layer {
        super::viewport::WindowLayer::AlwaysOnTop => LayerPreference::Top,
        super::viewport::WindowLayer::Normal => LayerPreference::Normal,
        super::viewport::WindowLayer::AlwaysOnBottom => LayerPreference::Bottom,
    }
}

#[derive(Clone, Copy)]
enum ViewportKind {
    Pet,
    Bubble,
    Hud,
    Settings,
    Menu,
    Unknown,
}

fn is_right_click(event: &WindowEvent) -> bool {
    matches!(
        event,
        WindowEvent::MouseInput {
            state: ElementState::Pressed,
            button: MouseButton::Right | MouseButton::Other(3),
            ..
        }
    )
}

fn is_window_close(event: &WindowEvent) -> bool {
    matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed)
}
