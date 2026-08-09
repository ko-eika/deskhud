//! 桌宠主应用：透明宠窗 + 右键菜单 + 统一设置窗。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Color32, Frame, Sense, Stroke, Vec2};
use deskhud_engine::{
    DockState, DragState, EngineRegistry, MouseState, PetConfigBag, PetEvent, PetModifiers,
    PetPaintCtx,
};
use deskhud_ui::{persist, UiPreferences};
use raw_window_handle::HasWindowHandle;
use tracing::{info, warn};

use crate::fonts;
use crate::pet_dock;
use crate::pet_draw;
use crate::pet_input;
use crate::hud_overlay::HudOverlayHost;
use crate::pet_menu::PetMenuHost;
use crate::settings::{SettingsHost, SettingsTab};
use crate::platform;

const PREFS_SAVE_DEBOUNCE: Duration = Duration::from_millis(400);

/// egui 桌宠状态。
pub struct PetApp {
    prefs: UiPreferences,
    host: EngineRegistry,
    /// 外壳 + 已发现包的合并文案（内置层已含多语言；打开设置时传入）。
    catalogs: deskhud_ui::CatalogStore,
    settings: SettingsHost,
    pet_menu: PetMenuHost,
    quitting: bool,
    hwnd: Option<isize>,
    pupil_smooth: [f32; 2],
    drag_grab_px: Option<(i32, i32)>,
    /// 非 Windows：拖拽时指针相对窗左上角的逻辑点偏移。
    drag_grab_points: Option<(f32, f32)>,
    /// 偏好相对上次成功落盘是否有变更。
    prefs_dirty: bool,
    /// 脏标记出现时间，用于防抖写盘。
    prefs_dirty_since: Option<Instant>,
    /// 启动后是否已应用一次 OuterPosition。
    position_applied: bool,
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
    hud_overlay: Arc<Mutex<HudOverlayHost>>,
    /// 上次已下发的全局置顶，避免每帧重复 WindowLevel。
    last_sent_topmost: Option<bool>,
    /// 关设置后再同步置顶（同帧关窗+改 WindowLevel 易 AV）。
    defer_topmost_sync: bool,
    /// 延迟执行宠窗改尺寸。
    pending_resize: bool,
    /// 延迟执行全局置顶同步。
    pending_topmost: bool,
    /// 设置已软关后、改尺寸/置顶前的等待帧。
    apply_delay_frames: u8,
}

impl PetApp {
    pub fn new(cc: &eframe::CreationContext<'_>, mut prefs: UiPreferences) -> Self {
        crate::theme::apply(&cc.egui_ctx, prefs.shell.ui_theme);
        fonts::configure_typography(
            &cc.egui_ctx,
            &prefs.shell.ui_font_id,
            prefs.shell.ui_font_size,
        );
        let hwnd = hwnd_from_cc(cc);
        if let Some(h) = hwnd {
            platform::ensure_pet_chrome(h);
        }

        let boot = deskhud_runtime::bootstrap_registry();
        let mut host = boot.registry;
        let catalogs = deskhud_runtime::build_catalog_store(&boot.discovered, prefs.locale);
        info!(
            packs = boot.discovered.len(),
            "local packages discovered"
        );
        if !host.set_active_pet(&prefs.pet.kind) {
            warn!(
                id = %prefs.pet.kind,
                "saved pet missing; falling back to default"
            );
        }
        // 尺寸以当前宠元数据为准（包升级后仍正确）
        sync_size_from_pet(&mut prefs, &host);

        let hud_overlay = Arc::new(Mutex::new(HudOverlayHost::default()));
        let mut app = Self {
            settings: SettingsHost::new(prefs.clone(), Arc::clone(&hud_overlay)),
            pet_menu: PetMenuHost::new(prefs.clone()),
            prefs,
            host,
            catalogs,
            quitting: false,
            hwnd,
            pupil_smooth: [0.0, 0.0],
            drag_grab_px: None,
            drag_grab_points: None,
            prefs_dirty: false,
            prefs_dirty_since: None,
            position_applied: false,
            pet_dock: DockState::FREE,
            pet_drag: DragState::IDLE,
            pet_mouse: MouseState::IDLE,
            prev_global_mouse: (false, false, false),
            global_mouse_primed: false,
            prev_global_keys: vec![false; pet_input::global_tracked_keys().len()],
            global_keys_primed: false,
            hud_overlay,
            last_sent_topmost: None,
            defer_topmost_sync: false,
            pending_resize: false,
            pending_topmost: false,
            apply_delay_frames: 0,
        };
        app.apply_size(&cc.egui_ctx);
        app.sync_global_topmost(&cc.egui_ctx);
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
        let order = self.prefs_write_order();
        match persist::save_ordered(&self.prefs, &order) {
            Ok(()) => {
                self.prefs_dirty = false;
                self.prefs_dirty_since = None;
                info!("prefs saved");
            }
            Err(e) => warn!(error = %e, "failed to save prefs"),
        }
    }

    fn prefs_write_order(&self) -> persist::PrefsWriteOrder {
        let mut order = persist::PrefsWriteOrder::default();
        for pet in self.host.pets() {
            let id = pet.info().id.to_string();
            order.pet_ids.push(id.clone());
            order.pet_option_keys.push((
                id,
                pet.config_options()
                    .iter()
                    .map(|o| o.key.to_string())
                    .collect(),
            ));
        }
        let mut plugin_seen = std::collections::HashSet::new();
        for (pid, c) in self.host.all_hud_contributions() {
            if plugin_seen.insert(pid) {
                order.plugin_ids.push(pid.to_string());
                order.plugin_contrib_ids.push((pid.to_string(), Vec::new()));
            }
            if let Some((_, contribs)) = order
                .plugin_contrib_ids
                .iter_mut()
                .find(|(id, _)| id == pid)
            {
                contribs.push(c.id.to_string());
            }
        }
        order
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
            self.prefs.pet.width,
            self.prefs.pet.height,
        )));
    }

    /// 全局置顶：仅已应用 prefs（设置草稿不即时改层级，否则设置页会卡死）。
    fn effective_topmost(&self) -> bool {
        self.prefs.shell.topmost
    }

    /// 宠 / HUD / 菜单 / 设置共用同一 WindowLevel；禁止混用置顶。
    fn sync_global_topmost(&mut self, ctx: &egui::Context) {
        let want = self.effective_topmost();
        if self.last_sent_topmost == Some(want) {
            return;
        }
        let level = if want {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
        self.last_sent_topmost = Some(want);
    }

    /// 视口实际应使用的置顶（延后同步期间保持旧值，避免关设置同帧改层级）。
    fn window_topmost(&self) -> bool {
        if self.apply_sequence_busy() {
            self.last_sent_topmost
                .unwrap_or_else(|| self.effective_topmost())
        } else {
            self.effective_topmost()
        }
    }

    /// 消费布局编辑的完成/取消/开始（设置页与编辑器）；可在 logic/ui 任一侧调用。
    fn consume_hud_layout_actions(&mut self, ctx: &egui::Context) {
        let (begin, finish_flag, cancel_flag) = {
            let s = self.settings.lock();
            (s.hud_layout_begin, s.hud_layout_finish, s.hud_layout_cancel)
        };

        if begin {
            let mut s = self.settings.lock();
            s.hud_layout_begin = false;
            let master_on = s.prefs.hud.is_master_enabled();
            let items = s.hud_items.clone();
            let draft = s.prefs.clone();
            drop(s);
            // 总开关关闭时忽略布局请求（设置草稿为准）
            if master_on {
                // 同步设置草稿（含 HUD 开关）；字体未改时不会重配
                let _ = self.apply_prefs_soft(ctx, draft.clone(), false);
                // 先藏设置窗（本帧不 Close），再开编辑；Close 延到 logic 下一拍
                self.settings.force_close_for_layout_edit(ctx);
                self.pet_menu.dismiss(ctx);
                if let Ok(mut h) = self.hud_overlay.lock() {
                    let pairs: Vec<(&str, deskhud_engine::HudContribution)> =
                        items.iter().map(|(p, c)| (*p, c.clone())).collect();
                    h.begin_edit(&draft.hud, &pairs);
                }
                let pairs: Vec<(&str, deskhud_engine::HudContribution)> =
                    items.iter().map(|(p, c)| (*p, c.clone())).collect();
                crate::hud_overlay::suppress_hud_slots_now(ctx, &self.hud_overlay, &pairs);
                ctx.request_repaint();
            }
        }

        // 优先消费 overlay 的 pending（编辑器点击时已立刻退出编辑 UI）
        let pending = self
            .hud_overlay
            .lock()
            .ok()
            .and_then(|mut h| h.take_pending());

        let do_finish =
            finish_flag || matches!(pending, Some(crate::hud_overlay::LayoutPending::Commit(_)));
        let do_cancel =
            cancel_flag || matches!(pending, Some(crate::hud_overlay::LayoutPending::Abort));

        if do_finish {
            {
                let mut s = self.settings.lock();
                s.hud_layout_finish = false;
                s.hud_layout_editing = false;
            }
            match pending {
                Some(crate::hud_overlay::LayoutPending::Commit(draft)) => {
                    if let Ok(mut h) = self.hud_overlay.lock() {
                        h.apply_draft_map(&mut self.prefs.hud, draft);
                    }
                }
                _ => {
                    if let Ok(mut h) = self.hud_overlay.lock() {
                        h.apply_edit(&mut self.prefs.hud);
                    }
                }
            }
            if self.settings.is_open() {
                let mut s = self.settings.lock();
                s.prefs.hud.copy_layout_keys_from(&self.prefs.hud);
                s.baseline.hud.copy_layout_keys_from(&self.prefs.hud);
            }
            self.mark_prefs_dirty();
            self.maybe_save_prefs(true);
            crate::hud_overlay::force_close_editor(ctx, &self.hud_overlay);
            ctx.request_repaint();
        } else if do_cancel {
            {
                let mut s = self.settings.lock();
                s.hud_layout_cancel = false;
                s.hud_layout_editing = false;
            }
            if let Ok(mut h) = self.hud_overlay.lock() {
                h.cancel_edit();
            }
            crate::hud_overlay::force_close_editor(ctx, &self.hud_overlay);
            ctx.request_repaint();
        }

        if let Ok(h) = self.hud_overlay.lock() {
            let mut s = self.settings.lock();
            if s.hud_layout_editing != h.editing {
                s.hud_layout_editing = h.editing;
            }
        }
    }

    fn apply_position_once(&mut self, ctx: &egui::Context) {
        if self.position_applied {
            return;
        }
        if let Some([x, y]) = self.prefs.pet.pos() {
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(x, y)));
        }
        self.position_applied = true;
    }

    fn capture_pet_position(&mut self, ctx: &egui::Context) {
        let ppp = ctx.pixels_per_point().max(0.01);
        let (x, y) = {
            #[cfg(windows)]
            {
                let Some(hwnd) = self.hwnd else {
                    return;
                };
                let Some(pos) = platform::window_screen_pos(hwnd) else {
                    return;
                };
                pos
            }
            #[cfg(not(windows))]
            {
                let Some(pos) = platform::window_screen_pos_from_ctx(ctx) else {
                    return;
                };
                pos
            }
        };
        let nx = x as f32 / ppp;
        let ny = y as f32 / ppp;
        let changed = self.prefs.pet.pos() != Some([nx, ny]);
        if changed {
            self.prefs.pet.set_pos(nx, ny);
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
            self.catalogs.clone(),
            tab,
        );
    }

    /// 设置打开时用已应用宠配置；草稿开关不进运行态，避免异常路径闪退。
    fn active_pet_config_map(&self) -> std::collections::HashMap<String, bool> {
        let pet = self.host.active_pet();
        let id = pet.info().id;
        let pairs: Vec<_> = pet
            .config_options()
            .iter()
            .map(|o| (o.key, o.default))
            .collect();
        self.prefs.pet.short_map_for(id, &pairs)
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

            let size_changed = draft.pet.width != self.prefs.pet.width
                || draft.pet.height != self.prefs.pet.height
                || draft.pet.kind != self.prefs.pet.kind;
            let topmost_changed = draft.shell.topmost != self.prefs.shell.topmost;

            // 只提交数据；不关设置窗。HUD 槽交给 show_slots 按 prefs 自然拆建
            // （勿 Visible(false) 压制：恢复时不发 Visible(true) 会一直藏着）
            let _ = self.apply_prefs_soft(ctx, draft, true);
            {
                let mut s = self.settings.lock();
                s.prefs = self.prefs.clone();
                s.baseline = self.prefs.clone();
                s.pending_flush = false;
            }

            self.pending_resize = size_changed;
            self.pending_topmost = topmost_changed;
            self.defer_topmost_sync = false;
            self.apply_delay_frames = if size_changed || topmost_changed { 4 } else { 0 };
            self.maybe_save_prefs(true);
            ctx.request_repaint();
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
                crate::theme::apply(ctx, self.prefs.shell.ui_theme);
                fonts::configure_typography(
                    ctx,
                    &self.prefs.shell.ui_font_id,
                    self.prefs.shell.ui_font_size,
                );
                self.prefs.shell.settings_width = draft.shell.settings_width;
                self.prefs.shell.settings_height = draft.shell.settings_height;
                self.prefs.shell.settings_pos_x = draft.shell.settings_pos_x;
                self.prefs.shell.settings_pos_y = draft.shell.settings_pos_y;
                self.mark_prefs_dirty();
            }
            if self.last_sent_topmost != Some(self.prefs.shell.topmost) {
                self.defer_topmost_sync = true;
            }
            ctx.request_repaint();
        }
    }


    /// 延迟结束后再改宠窗尺寸 / 置顶。
    fn flush_pending_window_ops(&mut self, ctx: &egui::Context) {
        if self.apply_delay_frames > 0 {
            return;
        }
        if !(self.pending_resize || self.pending_topmost || self.defer_topmost_sync) {
            return;
        }
        if self.pending_resize {
            self.pending_resize = false;
            self.apply_size(ctx);
            self.reanchor_pet_after_size_change(ctx, self.pet_dock);
        }
        if self.pending_topmost || self.defer_topmost_sync {
            self.pending_topmost = false;
            self.defer_topmost_sync = false;
            self.last_sent_topmost = None;
            if let Ok(mut h) = self.hud_overlay.lock() {
                h.invalidate_applied_topmost();
            }
            self.sync_global_topmost(ctx);
        }
    }

    fn apply_sequence_busy(&self) -> bool {
        self.apply_delay_frames > 0
            || self.pending_resize
            || self.pending_topmost
            || self.defer_topmost_sync
    }

    /// 只写 prefs / 切宠 / 主题字体；不发 InnerSize、WindowLevel。
    fn apply_prefs_soft(
        &mut self,
        ctx: &egui::Context,
        draft: UiPreferences,
        save: bool,
    ) -> bool {
        let size_changed = draft.pet.width != self.prefs.pet.width
            || draft.pet.height != self.prefs.pet.height
            || draft.pet.kind != self.prefs.pet.kind;
        let topmost_changed = draft.shell.topmost != self.prefs.shell.topmost;
        let font_changed = draft.shell.ui_font_id != self.prefs.shell.ui_font_id
            || (draft.shell.ui_font_size - self.prefs.shell.ui_font_size).abs() > 0.01
            || draft.shell.ui_font_style != self.prefs.shell.ui_font_style
            || draft.shell.ui_font_family != self.prefs.shell.ui_font_family;
        let theme_changed = draft.shell.ui_theme != self.prefs.shell.ui_theme;
        let pos = (self.prefs.pet.pos_x, self.prefs.pet.pos_y);
        self.prefs = draft;
        self.prefs.pet.pos_x = pos.0;
        self.prefs.pet.pos_y = pos.1;
        let _ = self.host.set_active_pet(&self.prefs.pet.kind);
        if size_changed {
            sync_size_from_pet(&mut self.prefs, &self.host);
        }
        if theme_changed {
            crate::theme::apply(ctx, self.prefs.shell.ui_theme);
        }
        if font_changed || theme_changed {
            fonts::configure_typography(
                ctx,
                &self.prefs.shell.ui_font_id,
                self.prefs.shell.ui_font_size,
            );
        }
        if save {
            self.mark_prefs_dirty();
        }
        topmost_changed
    }

    fn pull_pet_menu(&mut self, ctx: &egui::Context) {
        let mut s = self.pet_menu.lock();
        let open_settings = s.open_settings;
        let begin_hud_layout = s.begin_hud_layout;
        let toggle_master = s.toggle_master.take();
        let toggle_topmost = s.toggle_topmost.take();
        let quit = s.quit;
        s.open_settings = false;
        s.begin_hud_layout = false;
        s.quit = false;
        drop(s);

        if open_settings {
            self.pet_menu.dismiss(ctx);
            self.open_settings(SettingsTab::General);
        }
        if let Some(enabled) = toggle_master {
            self.prefs.hud.set_master_enabled(enabled);
            if self.settings.is_open() {
                self.settings
                    .lock()
                    .prefs
                    .hud
                    .set_master_enabled(enabled);
            }
            // 关闭总开关时若正在布局编辑，取消
            if !enabled {
                if let Ok(mut h) = self.hud_overlay.lock() {
                    if h.editing {
                        h.request_cancel();
                    }
                }
                self.settings.lock().hud_layout_cancel = true;
            }
            self.mark_prefs_dirty();
            self.maybe_save_prefs(true);
            ctx.request_repaint();
        }
        if let Some(on) = toggle_topmost {
            self.prefs.shell.topmost = on;
            if self.settings.is_open() {
                self.settings.lock().prefs.shell.topmost = on;
                self.settings.lock().baseline.shell.topmost = on;
                // 设置开着时勿立刻改 WindowLevel（设置页会丢输入）
                self.defer_topmost_sync = true;
            } else {
                self.last_sent_topmost = None;
                self.sync_global_topmost(ctx);
            }
            self.mark_prefs_dirty();
            self.maybe_save_prefs(true);
            ctx.request_repaint();
        }
        if begin_hud_layout && self.prefs.hud.is_master_enabled() {
            self.pet_menu.dismiss(ctx);
            {
                let mut s = self.settings.lock();
                // 菜单入口可能从未打开过设置，补齐列表与当前 prefs
                s.prefs = self.prefs.clone();
                s.plugins = self.host.plugin_infos();
                s.hud_items = self.host.all_hud_contributions();
                s.catalogs = self.catalogs.clone();
                s.hud_layout_begin = true;
            }
            ctx.request_repaint();
        }
        if quit {
            self.quit(ctx);
        }
    }

    fn pointer_dir(&self, ctx: &egui::Context, center: egui::Pos2) -> [f32; 2] {
        if let Some(hwnd) = self.hwnd {
            if let Some((cx, cy)) = platform::cursor_client_px(hwnd) {
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
        let cursor = platform::cursor_screen_px()
            .map(|(x, y)| egui::pos2(x as f32 / ppp, y as f32 / ppp))
            .or_else(|| ctx.pointer_interact_pos())
            .unwrap_or(egui::pos2(100.0, 100.0));
        let master = self.prefs.hud.is_master_enabled();
        let topmost = self.prefs.shell.topmost;
        self.pet_menu
            .open_at(&self.prefs, cursor, ppp, master, topmost);
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
        let ppp = ctx.pixels_per_point();
        #[cfg(windows)]
        if let Some(hwnd) = self.hwnd {
            let dock =
                pet_dock::snap_on_release(hwnd, pet_dock::SNAP_THRESHOLD_POINTS, ppp);
            self.set_pet_dock(dock);
        }
        #[cfg(not(windows))]
        {
            let dock =
                pet_dock::snap_on_release_ctx(ctx, pet_dock::SNAP_THRESHOLD_POINTS, ppp);
            self.set_pet_dock(dock);
        }
        self.capture_pet_position(ctx);
    }

    fn reanchor_pet_after_size_change(&mut self, ctx: &egui::Context, prefer: DockState) {
        let ppp = ctx.pixels_per_point();
        #[cfg(windows)]
        {
            let Some(hwnd) = self.hwnd else {
                return;
            };
            let dock = pet_dock::reanchor_after_size_change(
                hwnd,
                self.prefs.pet.width,
                self.prefs.pet.height,
                prefer,
                pet_dock::SNAP_THRESHOLD_POINTS,
                ppp,
            );
            self.set_pet_dock(dock);
        }
        #[cfg(not(windows))]
        {
            let dock = pet_dock::reanchor_after_size_change_ctx(
                ctx,
                self.prefs.pet.width,
                self.prefs.pet.height,
                prefer,
                pet_dock::SNAP_THRESHOLD_POINTS,
                ppp,
            );
            self.set_pet_dock(dock);
        }
        self.capture_pet_position(ctx);
    }

    fn refresh_pet_dock(&mut self, ctx: &egui::Context) {
        if self.pet_drag.is_dragging() {
            return;
        }
        #[cfg(windows)]
        {
            let _ = ctx;
            let Some(hwnd) = self.hwnd else {
                return;
            };
            self.set_pet_dock(pet_dock::current_dock(hwnd));
        }
        #[cfg(not(windows))]
        {
            self.set_pet_dock(pet_dock::current_dock_ctx(ctx));
        }
    }

    fn emit_pet(&self, event: PetEvent) {
        self.host.active_pet().on_event(event);
    }

    fn sync_global_mouse(&mut self, ui: &egui::Ui) {
        let (gp, gs, gm) = platform::global_mouse_buttons();
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
            (prev.0, gp, deskhud_engine::PetMouseButton::Primary),
            (prev.1, gs, deskhud_engine::PetMouseButton::Secondary),
            (prev.2, gm, deskhud_engine::PetMouseButton::Middle),
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
        let raw = platform::take_wheel_delta();
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
            let down = pet_input::global_pet_key_down(*key, platform::global_key_down);
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
                button: deskhud_engine::PetMouseButton::Primary,
                modifiers: mods,
            });
            let focused = ui.input(|i| i.viewport().focused).unwrap_or(false);
            if !focused {
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Focus);
            }
        }
        if response.secondary_clicked() {
            self.emit_pet(PetEvent::MouseClicked {
                button: deskhud_engine::PetMouseButton::Secondary,
                modifiers: mods,
            });
        }
        if response.middle_clicked() {
            self.emit_pet(PetEvent::MouseClicked {
                button: deskhud_engine::PetMouseButton::Middle,
                modifiers: mods,
            });
        }
        if response.double_clicked_by(egui::PointerButton::Primary) {
            self.emit_pet(PetEvent::MouseDoubleClicked {
                button: deskhud_engine::PetMouseButton::Primary,
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

        self.refresh_pet_dock(&ctx);

        let config_map = self.active_pet_config_map();
        let config = PetConfigBag::new(&config_map);
        self.host.active_pet().apply_config(config);

        let center = ui.max_rect().center();
        let pointer_dir = self.pointer_dir(&ctx, center);

        let base_radius =
            pet_draw::pet_base_radius(self.prefs.pet.width, self.prefs.pet.height);
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

        let _radius = base_radius * paint.bounce;

        if response.drag_started_by(egui::PointerButton::Primary)
            || response.clicked_by(egui::PointerButton::Primary)
        {
            if self.pet_menu.is_open() {
                self.pet_menu.dismiss(&ctx);
            }
        }
        if response.drag_started_by(egui::PointerButton::Primary) {
            #[cfg(windows)]
            if let Some(hwnd) = self.hwnd {
                if let (Some(cur), Some(origin)) = (
                    platform::cursor_screen_px(),
                    platform::window_screen_pos(hwnd),
                ) {
                    self.drag_grab_px = Some((cur.0 - origin.0, cur.1 - origin.1));
                    self.set_pet_dragging(true);
                }
            }
            #[cfg(not(windows))]
            {
                let (pointer, outer) = ui.input(|i| (i.pointer.latest_pos(), i.viewport().outer_rect));
                if let (Some(pointer), Some(outer)) = (pointer, outer) {
                    self.drag_grab_points =
                        Some((pointer.x - outer.min.x, pointer.y - outer.min.y));
                    self.set_pet_dragging(true);
                }
            }
        }
        #[cfg(windows)]
        if self.drag_grab_px.is_some() {
            let primary_down = ui.input(|i| i.pointer.primary_down());
            if primary_down {
                if let (Some(hwnd), Some(grab), Some(cur)) = (
                    self.hwnd,
                    self.drag_grab_px,
                    platform::cursor_screen_px(),
                ) {
                    platform::move_window_screen(hwnd, cur.0 - grab.0, cur.1 - grab.1);
                }
            } else {
                self.drag_grab_px = None;
                self.finish_pet_drag(&ctx);
            }
        }
        #[cfg(not(windows))]
        if self.drag_grab_points.is_some() {
            let primary_down = ui.input(|i| i.pointer.primary_down());
            if primary_down {
                if let (Some(grab), Some(pointer)) = (
                    self.drag_grab_points,
                    ui.input(|i| i.pointer.latest_pos()),
                ) {
                    let x = pointer.x - grab.0;
                    let y = pointer.y - grab.1;
                    platform::move_viewport_points(&ctx, x, y);
                }
            } else {
                self.drag_grab_points = None;
                self.finish_pet_drag(&ctx);
            }
        }

        if response.secondary_clicked() {
            if self.pet_drag.is_dragging() {
                self.drag_grab_px = None;
                self.drag_grab_points = None;
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
            self.prefs.pet.width,
        );
    }
}

fn sync_size_from_pet(prefs: &mut UiPreferences, host: &EngineRegistry) {
    let info = host.active_pet().info();
    prefs
        .pet
        .apply_window_size(info.window_width, info.window_height);
    prefs.pet.kind = info.id.to_string();
}

fn dir_from_center(center: egui::Pos2, pos: egui::Pos2) -> [f32; 2] {
    let d = pos - center;
    let len = (d.x * d.x + d.y * d.y).sqrt().max(1.0);
    [(d.x / len).clamp(-1.0, 1.0), (d.y / len).clamp(-1.0, 1.0)]
}

fn hwnd_from_cc(cc: &eframe::CreationContext<'_>) -> Option<isize> {
    let handle = cc.window_handle().ok()?;
    platform::native_window_id(handle.as_raw())
}

fn hwnd_from_frame(frame: &eframe::Frame) -> Option<isize> {
    let handle = frame.window_handle().ok()?;
    platform::native_window_id(handle.as_raw())
}

fn global_pet_modifiers(_ui: &egui::Ui) -> PetModifiers {
    let (shift, ctrl, alt) = platform::global_modifiers();
    let meta = platform::global_key_down(0x5B) || platform::global_key_down(0x5C);
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
        self.consume_hud_layout_actions(ctx);
        // 布局编辑切入后的下一拍再 Close 设置视口，避免同帧 AV
        self.settings.finish_layout_edit_close(ctx);
        if self.apply_delay_frames > 0 {
            self.apply_delay_frames -= 1;
            ctx.request_repaint();
        }
        // 冷静后再改宠窗尺寸 / 置顶
        self.flush_pending_window_ops(ctx);
        self.maybe_save_prefs(false);

        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting {
            self.capture_pet_position(ctx);
            self.maybe_save_prefs(true);
            self.quitting = true;
        }

        // 布局编辑中保持主动刷新
        if self
            .hud_overlay
            .lock()
            .map(|h| h.editing || h.has_pending())
            .unwrap_or(false)
        {
            ctx.request_repaint();
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(16));
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        // 每帧刷新 HWND：重建视口时句柄会变，旧子类化若不拆会 AV
        if let Some(h) = hwnd_from_frame(frame) {
            self.hwnd = Some(h);
            platform::ensure_pet_chrome(h);
            // 全局同层级，宠窗不再为设置开点击穿透
            platform::set_click_through(h, false);
        }

        let ctx = ui.ctx().clone();
        // 应用延后序列期间勿抢发 WindowLevel
        if !self.apply_sequence_busy() {
            self.sync_global_topmost(&ctx);
        }
        self.apply_position_once(&ctx);

        if self.settings.is_open() {
            self.settings.show(&ctx, self.hwnd);
        }
        if self.pet_menu.is_open() {
            self.pet_menu.show(&ctx, self.hwnd);
        }

        let hud_items = self.host.all_hud_contributions();
        // 设置打开时 HUD 仍用已应用 prefs：草稿开关即时拆建槽窗极易 AV
        let (done_l, cancel_l, reset_l, reset_size_l, hint_l) = (
            self.prefs.t(deskhud_ui::MessageKey::HudLayoutDone).to_string(),
            self.prefs.t(deskhud_ui::MessageKey::HudLayoutCancel).to_string(),
            self.prefs.t(deskhud_ui::MessageKey::ActionReset).to_string(),
            self.prefs.t(deskhud_ui::MessageKey::HudLayoutResetSize).to_string(),
            self.prefs.t(deskhud_ui::MessageKey::HudLayoutHint).to_string(),
        );
        let topmost = self.window_topmost();
        HudOverlayHost::show(
            &self.hud_overlay,
            &ctx,
            &self.prefs,
            &hud_items,
            None,
            &done_l,
            &cancel_l,
            &reset_l,
            &reset_size_l,
            &hint_l,
            topmost,
        );

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
