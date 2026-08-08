//! 桌宠主应用：透明宠窗 + 右键菜单 + 统一设置窗。

use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, FontId, Frame, Sense, Stroke, Vec2};
use deskhud_host::{
    DockState, DragState, HostRegistry, MouseState, PetConfigBag, PetEvent, PetModifiers,
    PetPaintCtx,
};
use deskhud_ui::{persist, UiPreferences};
use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use tracing::{info, warn};

use crate::fonts;
use crate::pet_dock;
use crate::pet_draw;
use crate::pet_input;
use crate::pet_menu::PetMenuHost;
use crate::settings::{SettingsHost, SettingsTab};
use crate::win_chrome;

const PREFS_SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

/// egui 桌宠状态。
pub struct PetApp {
    prefs: UiPreferences,
    host: HostRegistry,
    settings: SettingsHost,
    pet_menu: PetMenuHost,
    quitting: bool,
    hwnd: Option<isize>,
    pupil_smooth: [f32; 2],
    drag_grab_px: Option<(i32, i32)>,
    /// 偏好相对上次成功落盘是否有变更。
    prefs_dirty: bool,
    /// 脏标记出现时间，用于防抖写盘。
    prefs_dirty_since: Option<Instant>,
    /// 启动后是否已应用一次 OuterPosition。
    position_applied: bool,
    /// 设置打开时临时取消宠窗置顶，避免挡住设置交互。
    settings_opened_suspend_topmost: bool,
    /// 当前贴边状态（松手吸附 / 每帧复核后更新）。
    pet_dock: DockState,
    /// 当前拖拽状态。
    pet_drag: DragState,
    /// 当前鼠标快照（局部 + 全局）。
    pet_mouse: MouseState,
    /// 上一帧全局鼠标键，用于边沿事件。
    prev_global_mouse: (bool, bool, bool),
    /// 全局鼠标是否已完成首帧采样（首帧只对齐，不发边沿，避免误触发）。
    global_mouse_primed: bool,
    /// 上一帧全局按键子集状态（与 `pet_input::global_tracked_keys` 对齐）。
    prev_global_keys: Vec<bool>,
    global_keys_primed: bool,
}

impl PetApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut prefs: UiPreferences) -> Self {
        fonts::configure_fonts(&cc.egui_ctx);
        let hwnd = hwnd_from_cc(cc);
        if let Some(h) = hwnd {
            win_chrome::ensure_pet_chrome(h);
        }

        let mut visuals = egui::Visuals::light();
        visuals.panel_fill = Color32::TRANSPARENT;
        visuals.window_fill = Color32::TRANSPARENT;
        visuals.extreme_bg_color = Color32::TRANSPARENT;
        cc.egui_ctx.set_visuals_of(egui::Theme::Light, visuals);

        let mut style = (*cc.egui_ctx.style_of(egui::Theme::Light)).clone();
        style.visuals.panel_fill = Color32::TRANSPARENT;
        style.visuals.window_fill = Color32::TRANSPARENT;
        style.visuals.extreme_bg_color = Color32::TRANSPARENT;
        style.visuals.popup_shadow = egui::Shadow::NONE;
        style.visuals.window_stroke = Stroke::NONE;
        cc.egui_ctx.set_style_of(egui::Theme::Light, style);
        cc.egui_ctx.set_theme(egui::ThemePreference::Light);

        let mut host = HostRegistry::new();
        if !host.set_active_pet(&prefs.shell.active_pet_kind_id) {
            warn!(
                id = %prefs.shell.active_pet_kind_id,
                "saved pet missing; falling back to default"
            );
        }
        // 尺寸以当前宠元数据为准（包升级后仍正确）
        sync_size_from_pet(&mut prefs, &host);

        let mut app = Self {
            settings: SettingsHost::new(prefs.clone()),
            pet_menu: PetMenuHost::new(prefs.clone()),
            prefs,
            host,
            quitting: false,
            hwnd,
            pupil_smooth: [0.0, 0.0],
            drag_grab_px: None,
            prefs_dirty: false,
            prefs_dirty_since: None,
            position_applied: false,
            settings_opened_suspend_topmost: false,
            pet_dock: DockState::FREE,
            pet_drag: DragState::IDLE,
            pet_mouse: MouseState::IDLE,
            prev_global_mouse: (false, false, false),
            global_mouse_primed: false,
            prev_global_keys: vec![false; pet_input::global_tracked_keys().len()],
            global_keys_primed: false,
        };
        app.apply_size(&cc.egui_ctx);
        app.apply_topmost(&cc.egui_ctx);
        app.apply_position_once(&cc.egui_ctx);
        app
    }

    fn mark_prefs_dirty(&mut self) {
        if !self.prefs_dirty {
            self.prefs_dirty = true;
            self.prefs_dirty_since = Some(Instant::now());
        }
    }

    fn save_prefs_now(&mut self) {
        match persist::save(&self.prefs) {
            Ok(()) => {
                self.prefs_dirty = false;
                self.prefs_dirty_since = None;
                info!("prefs saved");
            }
            Err(e) => warn!(error = %e, "failed to save prefs"),
        }
    }

    fn maybe_save_prefs(&mut self, force: bool) {
        if !self.prefs_dirty {
            return;
        }
        let due = force
            || self
                .prefs_dirty_since
                .is_some_and(|t| t.elapsed() >= PREFS_SAVE_DEBOUNCE);
        if due {
            self.save_prefs_now();
        }
    }

    fn apply_size(&self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            self.prefs.shell.pet_width,
            self.prefs.shell.pet_height,
        )));
    }

    fn apply_topmost(&self, ctx: &egui::Context) {
        let level = if self.prefs.shell.pet_topmost {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }

    /// 设置窗打开时临时取消宠窗置顶（设置本身不 AlwaysOnTop，否则会被宠挡住且难操作）。
    fn sync_topmost_for_settings(&mut self, ctx: &egui::Context) {
        let open = self.settings.is_open();
        if open == self.settings_opened_suspend_topmost {
            return;
        }
        self.settings_opened_suspend_topmost = open;
        if open {
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::Normal,
            ));
        } else {
            self.apply_topmost(ctx);
        }
    }

    fn apply_position_once(&mut self, ctx: &egui::Context) {
        if self.position_applied {
            return;
        }
        if let Some([x, y]) = self.prefs.shell.pet_pos() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        }
        self.position_applied = true;
    }

    fn capture_pet_position(&mut self, ctx: &egui::Context) {
        let Some(hwnd) = self.hwnd else {
            return;
        };
        let Some((x, y)) = win_chrome::window_screen_pos(hwnd) else {
            return;
        };
        let ppp = ctx.pixels_per_point().max(0.01);
        let nx = x as f32 / ppp;
        let ny = y as f32 / ppp;
        let changed = self.prefs.shell.pet_pos() != Some([nx, ny]);
        if changed {
            self.prefs.shell.set_pet_pos(nx, ny);
            self.mark_prefs_dirty();
        }
    }

    fn quit(&mut self, ctx: &egui::Context) {
        self.capture_pet_position(ctx);
        self.maybe_save_prefs(true);
        self.quitting = true;
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    fn open_settings(&mut self, tab: SettingsTab) {
        let mut pet_options = std::collections::HashMap::new();
        for pet in self.host.pets() {
            let info = pet.info();
            pet_options.insert(info.id.to_string(), pet.config_options().to_vec());
        }
        self.settings.open(
            &self.prefs,
            self.host.pet_infos(),
            pet_options,
            self.host.plugin_infos(),
            self.host.all_hud_contributions(),
            tab,
        );
    }

    /// 设置打开时用草稿宠配置预览；否则用已应用偏好。
    fn active_pet_config_map(&self) -> std::collections::HashMap<String, bool> {
        let pet = self.host.active_pet();
        let id = pet.info().id;
        let pairs: Vec<_> = pet
            .config_options()
            .iter()
            .map(|o| (o.key, o.default))
            .collect();
        let pet_prefs = if self.settings.is_open() {
            self.settings.lock().prefs.pet.clone()
        } else {
            self.prefs.pet.clone()
        };
        pet_prefs.short_map_for(id, &pairs)
    }

    fn pull_settings(&mut self, ctx: &egui::Context) {
        let (pending_flush, apply_requested) = {
            let s = self.settings.lock();
            (s.pending_flush, s.apply_requested)
        };

        if apply_requested {
            let mut s = self.settings.lock();
            s.apply_requested = false;
            let draft = s.prefs.clone();
            drop(s);
            self.apply_prefs_from_settings(ctx, draft, true);
            let mut s = self.settings.lock();
            s.prefs = self.prefs.clone();
            s.baseline = self.prefs.clone();
            return;
        }

        if pending_flush {
            let mut s = self.settings.lock();
            s.pending_flush = false;
            let discard = s.discard_draft;
            s.discard_draft = false;
            let draft = s.prefs.clone();
            drop(s);
            if discard {
                // 取消：只同步设置窗几何 + 视图模式偏好可保留在草稿里已丢弃
                // 几何写在 draft 上（关闭前 capture），合并到 app
                self.prefs.shell.settings_width = draft.shell.settings_width;
                self.prefs.shell.settings_height = draft.shell.settings_height;
                self.prefs.shell.settings_pos_x = draft.shell.settings_pos_x;
                self.prefs.shell.settings_pos_y = draft.shell.settings_pos_y;
                self.mark_prefs_dirty();
            }
            return;
        }
    }

    fn apply_prefs_from_settings(
        &mut self,
        ctx: &egui::Context,
        draft: UiPreferences,
        save: bool,
    ) {
        let size_changed = draft.shell.pet_width != self.prefs.shell.pet_width
            || draft.shell.pet_height != self.prefs.shell.pet_height
            || draft.shell.active_pet_kind_id != self.prefs.shell.active_pet_kind_id;
        let topmost_changed = draft.shell.pet_topmost != self.prefs.shell.pet_topmost;
        let pos = (self.prefs.shell.pet_pos_x, self.prefs.shell.pet_pos_y);
        self.prefs = draft;
        self.prefs.shell.pet_pos_x = pos.0;
        self.prefs.shell.pet_pos_y = pos.1;
        let _ = self
            .host
            .set_active_pet(&self.prefs.shell.active_pet_kind_id);
        if size_changed {
            let prefer_dock = self.pet_dock;
            sync_size_from_pet(&mut self.prefs, &self.host);
            self.apply_size(ctx);
            self.reanchor_pet_after_size_change(ctx, prefer_dock);
        }
        if topmost_changed {
            self.apply_topmost(ctx);
        }
        if save {
            self.mark_prefs_dirty();
        }
    }

    fn pull_pet_menu(&mut self, ctx: &egui::Context) {
        let mut s = self.pet_menu.lock();
        let open_settings = s.open_settings;
        let quit = s.quit;
        s.open_settings = false;
        s.quit = false;
        drop(s);

        if open_settings {
            self.pet_menu.dismiss(ctx);
            self.open_settings(SettingsTab::General);
        }
        if quit {
            self.quit(ctx);
        }
    }

    fn pointer_dir(&self, ctx: &egui::Context, center: egui::Pos2) -> [f32; 2] {
        if let Some(hwnd) = self.hwnd {
            if let Some((cx, cy)) = win_chrome::cursor_client_px(hwnd) {
                let ppp = ctx.pixels_per_point();
                let pos = egui::pos2(cx as f32 / ppp, cy as f32 / ppp);
                return dir_from_center(center, pos);
            }
        }
        ctx.input(|i| {
            i.pointer
                .hover_pos()
                .or_else(|| i.pointer.latest_pos())
                .map(|p| dir_from_center(center, p))
                .unwrap_or([0.0, 0.0])
        })
    }

    fn open_context_menu(&self, ctx: &egui::Context) {
        let ppp = ctx.pixels_per_point();
        let cursor = win_chrome::cursor_screen_px()
            .map(|(x, y)| egui::pos2(x as f32 / ppp, y as f32 / ppp))
            .or_else(|| ctx.pointer_interact_pos())
            .unwrap_or(egui::pos2(100.0, 100.0));
        self.pet_menu.open_at(&self.prefs, cursor, ppp);
    }

    fn set_pet_dock(&mut self, next: DockState) {
        if next == self.pet_dock {
            return;
        }
        let from = self.pet_dock;
        self.pet_dock = next;
        self.host.active_pet().on_event(PetEvent::DockChanged {
            from,
            to: next,
        });
    }

    fn set_pet_dragging(&mut self, active: bool) {
        let next = if active {
            DragState::ACTIVE
        } else {
            DragState::IDLE
        };
        if next == self.pet_drag {
            return;
        }
        self.pet_drag = next;
        if active {
            self.host.active_pet().on_event(PetEvent::DragStarted);
        } else {
            self.host
                .active_pet()
                .on_event(PetEvent::DragEnded { drag: next });
        }
    }

    fn finish_pet_drag(&mut self, ctx: &egui::Context) {
        self.set_pet_dragging(false);
        if let Some(hwnd) = self.hwnd {
            let ppp = ctx.pixels_per_point();
            let dock =
                pet_dock::snap_on_release(hwnd, pet_dock::SNAP_THRESHOLD_POINTS, ppp);
            self.set_pet_dock(dock);
        }
        self.capture_pet_position(ctx);
    }

    fn reanchor_pet_after_size_change(&mut self, ctx: &egui::Context, prefer: DockState) {
        let Some(hwnd) = self.hwnd else {
            return;
        };
        let ppp = ctx.pixels_per_point();
        let dock = pet_dock::reanchor_after_size_change(
            hwnd,
            self.prefs.shell.pet_width,
            self.prefs.shell.pet_height,
            prefer,
            pet_dock::SNAP_THRESHOLD_POINTS,
            ppp,
        );
        self.set_pet_dock(dock);
        self.capture_pet_position(ctx);
    }

    fn refresh_pet_dock(&mut self) {
        let Some(hwnd) = self.hwnd else {
            return;
        };
        if self.pet_drag.is_dragging() {
            return;
        }
        self.set_pet_dock(pet_dock::current_dock(hwnd));
    }

    fn emit_pet(&self, event: PetEvent) {
        self.host.active_pet().on_event(event);
    }

    fn sync_global_mouse(&mut self, ui: &egui::Ui) {
        let (gp, gs, gm) = win_chrome::global_mouse_buttons();
        if !self.global_mouse_primed {
            self.pet_mouse.global_primary_down = gp;
            self.pet_mouse.global_secondary_down = gs;
            self.pet_mouse.global_middle_down = gm;
            self.prev_global_mouse = (gp, gs, gm);
            self.global_mouse_primed = true;
            return;
        }
        let mods = global_pet_modifiers(ui);
        let prev = self.prev_global_mouse;
        let pairs = [
            (prev.0, gp, deskhud_host::PetMouseButton::Primary),
            (prev.1, gs, deskhud_host::PetMouseButton::Secondary),
            (prev.2, gm, deskhud_host::PetMouseButton::Middle),
        ];
        for (was, now, button) in pairs {
            if !was && now {
                self.emit_pet(PetEvent::GlobalMousePressed {
                    button,
                    modifiers: mods,
                });
            } else if was && !now {
                self.emit_pet(PetEvent::GlobalMouseReleased {
                    button,
                    modifiers: mods,
                });
            }
        }
        self.pet_mouse.global_primary_down = gp;
        self.pet_mouse.global_secondary_down = gs;
        self.pet_mouse.global_middle_down = gm;
        self.prev_global_mouse = (gp, gs, gm);
    }

    fn sync_global_wheel(&mut self, ui: &egui::Ui) {
        let raw = win_chrome::take_wheel_delta();
        if raw == 0 {
            return;
        }
        // Windows：+120 ≈ 一格向上
        let notches = (raw as f32 / 120.0).round() as i32;
        let delta = notches.clamp(i8::MIN as i32, i8::MAX as i32) as i8;
        if delta == 0 {
            return;
        }
        let mods = global_pet_modifiers(ui);
        self.emit_pet(PetEvent::GlobalMouseWheel {
            delta,
            modifiers: mods,
        });
    }

    fn sync_global_keys(&mut self, ui: &egui::Ui) {
        let tracked = pet_input::global_tracked_keys();
        let mut now = Vec::with_capacity(tracked.len());
        for key in tracked {
            let down = pet_input::global_pet_key_down(*key, win_chrome::global_key_down);
            now.push(down);
        }
        if !self.global_keys_primed {
            self.prev_global_keys = now;
            self.global_keys_primed = true;
            return;
        }
        let mods = global_pet_modifiers(ui);
        for (i, key) in tracked.iter().enumerate() {
            let was = self.prev_global_keys.get(i).copied().unwrap_or(false);
            let is_down = now[i];
            if !was && is_down {
                self.emit_pet(PetEvent::GlobalKeyPressed {
                    key: *key,
                    modifiers: mods,
                });
            } else if was && !is_down {
                self.emit_pet(PetEvent::GlobalKeyReleased {
                    key: *key,
                    modifiers: mods,
                });
            }
        }
        self.prev_global_keys = now;
    }

    fn sync_pet_mouse_and_input(&mut self, ui: &mut egui::Ui, response: &egui::Response) {
        self.sync_global_mouse(ui);
        self.sync_global_wheel(ui);
        self.sync_global_keys(ui);

        let mods = ui.input(|i| pet_input::modifiers_from_egui(&i.modifiers));
        let hovering = response.hovered();
        if hovering != self.pet_mouse.hovering {
            self.pet_mouse.hovering = hovering;
            self.emit_pet(PetEvent::MouseHover { inside: hovering });
        }

        let (p_down, s_down, m_down) = if hovering {
            ui.input(|i| {
                (
                    i.pointer.button_down(egui::PointerButton::Primary),
                    i.pointer.button_down(egui::PointerButton::Secondary),
                    i.pointer.button_down(egui::PointerButton::Middle),
                )
            })
        } else {
            (false, false, false)
        };
        self.pet_mouse.primary_down = p_down;
        self.pet_mouse.secondary_down = s_down;
        self.pet_mouse.middle_down = m_down;

        let track_buttons = hovering || response.dragged() || response.drag_stopped();
        if track_buttons {
            for button in [
                egui::PointerButton::Primary,
                egui::PointerButton::Secondary,
                egui::PointerButton::Middle,
            ] {
                let Some(pb) = pet_input::mouse_button_from_egui(button) else {
                    continue;
                };
                if ui.input(|i| i.pointer.button_pressed(button)) {
                    self.emit_pet(PetEvent::MousePressed {
                        button: pb,
                        modifiers: mods,
                    });
                }
                if ui.input(|i| i.pointer.button_released(button)) {
                    self.emit_pet(PetEvent::MouseReleased {
                        button: pb,
                        modifiers: mods,
                    });
                }
            }
        }

        if response.clicked_by(egui::PointerButton::Primary) {
            self.emit_pet(PetEvent::MouseClicked {
                button: deskhud_host::PetMouseButton::Primary,
                modifiers: mods,
            });
            let focused = ui.input(|i| i.viewport().focused).unwrap_or(false);
            if !focused {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        if response.secondary_clicked() {
            self.emit_pet(PetEvent::MouseClicked {
                button: deskhud_host::PetMouseButton::Secondary,
                modifiers: mods,
            });
        }
        if response.middle_clicked() {
            self.emit_pet(PetEvent::MouseClicked {
                button: deskhud_host::PetMouseButton::Middle,
                modifiers: mods,
            });
        }
        if response.double_clicked_by(egui::PointerButton::Primary) {
            self.emit_pet(PetEvent::MouseDoubleClicked {
                button: deskhud_host::PetMouseButton::Primary,
                modifiers: mods,
            });
        }

        let focused = ui.input(|i| i.viewport().focused).unwrap_or(false);
        if !focused {
            return;
        }
        let key_events: Vec<(bool, egui::Key)> = ui.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed,
                        repeat,
                        ..
                    } if !*repeat => Some((*pressed, *key)),
                    _ => None,
                })
                .collect()
        });
        for (pressed, key) in key_events {
            let Some(pk) = pet_input::key_from_egui(key) else {
                continue;
            };
            if pressed {
                self.emit_pet(PetEvent::KeyPressed {
                    key: pk,
                    modifiers: mods,
                });
            } else {
                self.emit_pet(PetEvent::KeyReleased {
                    key: pk,
                    modifiers: mods,
                });
            }
        }
    }

    fn draw_pet(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        let time = ui.input(|i| i.time);
        let dt = ui.input(|i| i.stable_dt).max(0.0);
        self.host.active_pet().tick(dt);

        self.refresh_pet_dock();

        let config_map = self.active_pet_config_map();
        let config = PetConfigBag::new(&config_map);
        self.host.active_pet().apply_config(config);

        let center = ui.max_rect().center();
        let pointer_dir = self.pointer_dir(&ctx, center);

        let base_radius =
            pet_draw::pet_base_radius(self.prefs.shell.pet_width, self.prefs.shell.pet_height);
        // 先用近似半径做命中，再按 paint.bounce 微调绘制
        let hit = egui::Rect::from_center_size(center, Vec2::splat(base_radius * 2.15));
        let response = ui.interact(hit, ui.id().with("pet"), Sense::click_and_drag());

        self.sync_pet_mouse_and_input(ui, &response);

        let paint = self.host.active_pet().paint(PetPaintCtx {
            time_secs: time,
            pointer_dir,
            status_line: "",
            dock: self.pet_dock,
            drag: self.pet_drag,
            mouse: self.pet_mouse,
            config,
        });

        let lerp = 0.28;
        self.pupil_smooth[0] += (paint.pupil_offset[0] - self.pupil_smooth[0]) * lerp;
        self.pupil_smooth[1] += (paint.pupil_offset[1] - self.pupil_smooth[1]) * lerp;

        let radius = base_radius * paint.bounce;

        if response.drag_started_by(egui::PointerButton::Primary)
            || response.clicked_by(egui::PointerButton::Primary)
        {
            if self.pet_menu.is_open() {
                self.pet_menu.dismiss(&ctx);
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            if let Some(hwnd) = self.hwnd {
                if let (Some(cur), Some(origin)) = (
                    win_chrome::cursor_screen_px(),
                    win_chrome::window_screen_pos(hwnd),
                ) {
                    self.drag_grab_px = Some((cur.0 - origin.0, cur.1 - origin.1));
                    self.set_pet_dragging(true);
                }
            }
        }
        if self.drag_grab_px.is_some() {
            let primary_down = ui.input(|i| i.pointer.primary_down());
            if primary_down {
                if let (Some(hwnd), Some(grab), Some(cur)) = (
                    self.hwnd,
                    self.drag_grab_px,
                    win_chrome::cursor_screen_px(),
                ) {
                    win_chrome::move_window_screen(hwnd, cur.0 - grab.0, cur.1 - grab.1);
                }
            } else {
                self.drag_grab_px = None;
                self.finish_pet_drag(&ctx);
            }
        }

        if response.secondary_clicked() {
            if self.pet_drag.is_dragging() {
                self.drag_grab_px = None;
                self.set_pet_dragging(false);
            }
            self.open_context_menu(&ctx);
            ctx.request_repaint();
        }

        pet_draw::draw_pet_frame(
            ui.painter(),
            center,
            base_radius,
            &paint,
            self.pupil_smooth,
            self.prefs.shell.pet_width,
        );

        self.draw_enabled_hud_strip(ui, center, radius);
    }

    /// 设置打开时用草稿 HUD 开关做预览；否则用已应用偏好。
    fn hud_active(&self, plugin_id: &str, id: &str, default_enabled: bool) -> bool {
        if self.settings.is_open() {
            self.settings
                .lock()
                .prefs
                .hud
                .is_active(plugin_id, id, default_enabled)
        } else {
            self.prefs.hud.is_active(plugin_id, id, default_enabled)
        }
    }

    fn draw_enabled_hud_strip(&self, ui: &mut egui::Ui, center: egui::Pos2, radius: f32) {
        let contribs = self.host.all_hud_contributions();
        let enabled: Vec<_> = contribs
            .into_iter()
            .filter(|(pid, c)| self.hud_active(pid, c.id, c.default_enabled))
            .collect();
        if enabled.is_empty() {
            return;
        }

        let painter = ui.painter();
        let win_h = self.prefs.shell.pet_height;
        let win_w = self.prefs.shell.pet_width;
        // 画在宠窗底部内侧，避免被透明窗裁切（旧实现画在头顶外）
        let mut y = (win_h - 6.0).min(center.y + radius + 18.0).max(24.0);
        let font = FontId::proportional(11.0);
        let text_color = Color32::from_rgb(245, 247, 250);
        let chip_fill = Color32::from_rgba_unmultiplied(20, 24, 32, 200);
        let chip_stroke = Stroke::new(1.0, Color32::from_rgba_unmultiplied(255, 255, 255, 40));

        for (_, c) in enabled.iter().rev() {
            let text = match c.id {
                "clock" => {
                    let t = ui.input(|i| i.time);
                    let m = ((t / 60.0) as u64) % 60;
                    let s = (t as u64) % 60;
                    format!("时钟 {m:02}:{s:02}")
                }
                "tip" => "DeskHud 演示".to_string(),
                _ => c.label.to_string(),
            };
            let galley = painter.layout_no_wrap(text, font.clone(), text_color);
            let pad_x = 8.0;
            let pad_y = 3.0;
            let tw = galley.size().x;
            let th = galley.size().y;
            let bw = (tw + pad_x * 2.0).min(win_w - 8.0);
            let bh = th + pad_y * 2.0;
            let chip = egui::Rect::from_center_size(
                egui::pos2(center.x, y - bh * 0.5),
                Vec2::new(bw, bh),
            );
            painter.rect(
                chip,
                egui::CornerRadius::same(8),
                chip_fill,
                chip_stroke,
                egui::StrokeKind::Inside,
            );
            painter.galley(
                egui::pos2(chip.center().x - tw * 0.5, chip.center().y - th * 0.5),
                galley,
                text_color,
            );
            y -= bh + 4.0;
            if y < 8.0 {
                break;
            }
        }
    }
}

fn sync_size_from_pet(prefs: &mut UiPreferences, host: &HostRegistry) {
    let info = host.active_pet().info();
    prefs
        .shell
        .apply_pet_window_size(info.window_width, info.window_height);
    prefs.shell.active_pet_kind_id = info.id.to_string();
}

fn dir_from_center(center: egui::Pos2, pos: egui::Pos2) -> [f32; 2] {
    let d = pos - center;
    let len = (d.x * d.x + d.y * d.y).sqrt().max(1.0);
    [(d.x / len).clamp(-1.0, 1.0), (d.y / len).clamp(-1.0, 1.0)]
}

fn hwnd_from_cc(cc: &eframe::CreationContext<'_>) -> Option<isize> {
    let handle = cc.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(win) => Some(win.hwnd.get() as isize),
        _ => None,
    }
}

fn hwnd_from_frame(frame: &eframe::Frame) -> Option<isize> {
    let handle = frame.window_handle().ok()?;
    match handle.as_raw() {
        RawWindowHandle::Win32(win) => Some(win.hwnd.get() as isize),
        _ => None,
    }
}

fn global_pet_modifiers(_ui: &egui::Ui) -> PetModifiers {
    let (shift, ctrl, alt) = win_chrome::global_modifiers();
    let meta = win_chrome::global_key_down(0x5B) || win_chrome::global_key_down(0x5C);
    PetModifiers {
        shift,
        ctrl,
        alt,
        meta,
    }
}

impl eframe::App for PetApp {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.pull_settings(ctx);
        self.pull_pet_menu(ctx);
        self.maybe_save_prefs(false);
        self.sync_topmost_for_settings(ctx);

        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            self.capture_pet_position(ctx);
            self.maybe_save_prefs(true);
            self.quitting = true;
        }

        // 设置打开时降频，减轻双视口抢 CPU
        let ms = if self.settings.is_open() { 50 } else { 16 };
        ctx.request_repaint_after(std::time::Duration::from_millis(ms));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // 每帧刷新 HWND：重建视口时句柄会变，旧子类化若不拆会 AV
        if let Some(h) = hwnd_from_frame(frame) {
            self.hwnd = Some(h);
            win_chrome::ensure_pet_chrome(h);
        }

        let ctx = ui.ctx().clone();
        self.apply_position_once(&ctx);

        if self.settings.is_open() {
            self.settings.show(&ctx, self.hwnd);
        }
        if self.pet_menu.is_open() {
            self.pet_menu.show(&ctx, self.hwnd);
        }

        ui.visuals_mut().panel_fill = Color32::TRANSPARENT;
        ui.visuals_mut().window_fill = Color32::TRANSPARENT;
        ui.visuals_mut().extreme_bg_color = Color32::TRANSPARENT;
        ui.visuals_mut().window_stroke = Stroke::NONE;

        Frame::NONE
            .fill(Color32::TRANSPARENT)
            .stroke(Stroke::NONE)
            .inner_margin(0.0)
            .show(ui, |ui| {
                self.draw_pet(ui);
            });
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.maybe_save_prefs(true);
    }
}
