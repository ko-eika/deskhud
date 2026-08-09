//! HUD：平时每条一个小窗；布局编辑时关闭全部小窗，打开单个截图式编辑视口。
//! 子窗勿 `with_transparent(true)`；半透明靠截图底 + 半透明遮罩绘制模拟。

use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};

use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, FontId, Frame, Pos2, Rect, RichText, Sense,
    Stroke, TextureHandle, TextureOptions, Vec2, ViewportCommand,
};
use deskhud_engine::HudContribution;
use deskhud_ui::{HudPrefs, HudSlotLayout, UiPreferences};

use crate::platform::{self, DisplayInfo};

fn slot_viewport_id(plugin_id: &str, contrib_id: &str) -> egui::ViewportId {
    egui::ViewportId::from_hash_of(("deskhud_hud_slot", plugin_id, contrib_id))
}

fn editor_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("deskhud_hud_layout_editor")
}

fn toolbar_viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("deskhud_hud_layout_toolbar")
}

/// 布局编辑网格步长（逻辑点）；基准尺寸与缩放均按此对齐，保证四边可贴格。
pub const LAYOUT_GRID: f32 = 24.0;

fn align_dim_to_grid(v: f32, grid: f32) -> f32 {
    (v / grid).round().max(1.0) * grid
}

/// 内容基准逻辑像素（已对齐网格）；显示尺寸 = 基准 × scale。
pub fn base_size_points(contrib_id: &str, label: &str) -> Vec2 {
    let raw = match contrib_id {
        "clock" => Vec2::new(152.0, 36.0),
        "tip" => Vec2::new(132.0, 36.0),
        _ => {
            let w = (label.chars().count() as f32 * 9.0 + 28.0).clamp(112.0, 200.0);
            Vec2::new(w, 36.0)
        }
    };
    Vec2::new(
        align_dim_to_grid(raw.x, LAYOUT_GRID),
        align_dim_to_grid(raw.y, LAYOUT_GRID),
    )
}

/// 布局编辑待提交动作（点击完成/取消时立刻产生，主循环落盘）。
#[derive(Debug)]
pub enum LayoutPending {
    /// 提交草稿布局。
    Commit(HashMap<String, HudSlotLayout>),
    /// 放弃编辑。
    Abort,
}

/// HUD 布局编辑状态。
#[derive(Default)]
pub struct HudOverlayHost {
    pub editing: bool,
    draft: HashMap<String, HudSlotLayout>,
    /// 进入编辑时的快照；「重置」恢复到此。
    baseline: HashMap<String, HudSlotLayout>,
    drag_key: Option<String>,
    drag_mode: DragMode,
    drag_scale_corner: ScaleCorner,
    /// 缩放时对角固定点（编辑器逻辑坐标）。
    drag_fixed_pos: Pos2,
    /// 移动时指针相对条目左上角的抓取偏移。
    drag_grab_offset: Vec2,
    drag_origin_layout: HudSlotLayout,
    /// 当前选中的条目（「重置大小」只作用于它）。
    selected_key: Option<String>,
    live_slots: HashSet<(String, String)>,
    slot_hwnd: HashMap<String, isize>,
    slot_click_through: HashMap<String, bool>,
    pending: Option<LayoutPending>,
    editor_open: bool,
    /// 编辑用截图（RGBA）；无截图时用纯色底。
    screenshot: Option<ColorImage>,
    screenshot_tex: Option<TextureHandle>,
    /// 截图平均亮度（0..=1），供网格/提示对比色。
    bg_luma: f32,
    editor_display: Option<DisplayInfo>,
    /// 进入编辑后先藏槽窗，再延后截屏（避免拍到旧 HUD）。
    skip_screenshot_frames: u8,
    /// 槽窗已应用的全局置顶（变化时才下发 WindowLevel，避免每帧 SetWindowPos）。
    applied_topmost: Option<bool>,
    /// 上一帧全局左键，用于可靠检测按下边沿。
    lmb_prev: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
enum DragMode {
    #[default]
    None,
    Move,
    Scale,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(u8)]
enum ScaleCorner {
    #[default]
    Se = 0,
    Sw = 1,
    Ne = 2,
    Nw = 3,
}

impl HudOverlayHost {
    pub fn begin_edit(&mut self, prefs: &HudPrefs, items: &[(&str, HudContribution)]) {
        self.editing = true;
        self.draft.clear();
        self.baseline.clear();
        self.pending = None;
        self.editor_open = false;
        for (i, (pid, c)) in items.iter().enumerate() {
            if !prefs.is_plugin_enabled(pid) {
                continue;
            }
            let key = HudPrefs::layout_key(pid, c.id);
            let label = slot_label(c.id, c.label);
            let base = base_size_points(c.id, &label);
            let mut layout = prefs.slot_layout(pid, c.id, i);
            // 进入编辑即把尺寸对齐到网格，保证四边可吸附
            layout.scale = snap_scale_to_grid(layout.scale, base, LAYOUT_GRID);
            self.draft.insert(key.clone(), layout.clone());
            self.baseline.insert(key, layout);
        }
        self.drag_key = None;
        self.drag_mode = DragMode::None;
        self.selected_key = None;

        let displays = platform::list_displays();
        self.editor_display = displays
            .iter()
            .find(|d| d.primary)
            .or(displays.first())
            .cloned();
        // 截图延后到关槽窗之后（show 里），避免把 HUD 小窗拍进去
        self.screenshot = None;
        self.screenshot_tex = None;
        self.bg_luma = 0.25;
        self.skip_screenshot_frames = 2;
    }

    /// 恢复进入编辑时的布局。
    pub fn reset_draft(&mut self) {
        self.draft = self.baseline.clone();
        self.drag_key = None;
        self.drag_mode = DragMode::None;
    }

    /// 将选中条目缩放恢复为 1×；无选中则不做事。
    pub fn reset_selected_scale_to_one(&mut self) -> bool {
        let Some(key) = self.selected_key.clone() else {
            return false;
        };
        if let Some(layout) = self.draft.get_mut(&key) {
            layout.scale = 1.0;
            self.drag_key = None;
            self.drag_mode = DragMode::None;
            true
        } else {
            false
        }
    }

    pub fn cancel_edit(&mut self) {
        self.editing = false;
        self.draft.clear();
        self.baseline.clear();
        self.drag_key = None;
        self.drag_mode = DragMode::None;
        self.pending = None;
        self.selected_key = None;
        self.editor_open = false;
        self.screenshot = None;
        self.screenshot_tex = None;
        self.editor_display = None;
    }

    /// 「完成」：立刻退出编辑 UI，草稿进入 pending。
    /// 注意：保留 `editor_open=true`，由 ROOT 的 `show` 发 Visible(false)+Close。
    pub fn request_finish(&mut self) {
        self.pending = Some(LayoutPending::Commit(self.draft.clone()));
        self.editing = false;
        self.drag_key = None;
        self.drag_mode = DragMode::None;
    }

    /// 「取消」：立刻退出。
    pub fn request_cancel(&mut self) {
        self.pending = Some(LayoutPending::Abort);
        self.editing = false;
        self.draft.clear();
        self.baseline.clear();
        self.drag_key = None;
        self.drag_mode = DragMode::None;
    }

    pub fn has_pending(&self) -> bool {
        self.pending.is_some()
    }

    /// 置顶变更后清掉槽窗缓存层级，下一帧重新下发。
    pub fn invalidate_applied_topmost(&mut self) {
        self.applied_topmost = None;
    }

    pub fn take_pending(&mut self) -> Option<LayoutPending> {
        self.pending.take()
    }

    pub fn apply_draft_map(
        &mut self,
        prefs: &mut HudPrefs,
        draft: HashMap<String, HudSlotLayout>,
    ) {
        for (key, layout) in draft {
            if let Some((plugin, contrib)) = key.rsplit_once('.') {
                prefs.set_slot_layout(plugin, contrib, layout);
            }
        }
        self.editing = false;
        self.draft.clear();
        self.baseline.clear();
        self.pending = None;
        self.editor_open = false;
        self.screenshot = None;
        self.screenshot_tex = None;
        self.editor_display = None;
    }

    pub fn apply_edit(&mut self, prefs: &mut HudPrefs) {
        let draft = std::mem::take(&mut self.draft);
        self.apply_draft_map(prefs, draft);
    }

    /// 绘制 HUD / 编辑器。
    pub fn show(
        host: &Arc<Mutex<Self>>,
        ctx: &egui::Context,
        prefs: &UiPreferences,
        hud_items: &[(&str, HudContribution)],
        draft_prefs: Option<&HudPrefs>,
        done_label: &str,
        cancel_label: &str,
        reset_label: &str,
        reset_size_label: &str,
        hint_label: &str,
        // 全局置顶：宠 / HUD / 菜单 / 设置同一层级，禁止混用。
        topmost: bool,
    ) {
        // 关掉历史工具条视口
        ctx.send_viewport_cmd_to(toolbar_viewport_id(), ViewportCommand::Close);

        let active_prefs = draft_prefs.unwrap_or(&prefs.hud);
        let (editing, finish_pending) = host
            .lock()
            .map(|h| {
                (
                    h.editing,
                    matches!(h.pending, Some(LayoutPending::Commit(_))),
                )
            })
            .unwrap_or((false, false));

        let layout_prefs_owned;
        let layout_source: &HudPrefs = if editing || finish_pending {
            layout_prefs_owned = {
                let mut p = active_prefs.clone();
                if let Ok(h) = host.lock() {
                    for (key, layout) in &h.draft {
                        if let Some((plugin, contrib)) = key.rsplit_once('.') {
                            p.set_slot_layout(plugin, contrib, layout.clone());
                        }
                    }
                }
                p
            };
            &layout_prefs_owned
        } else {
            active_prefs
        };

        if editing {
            // 每帧强制藏掉所有 HUD 槽窗（含已知条目，Close 不可靠）
            suppress_all_slots(host, ctx, hud_items);

            // 必须先截到桌面，再开编辑窗；否则截到的是不透明编辑窗本身
            let editor_ready = match host.lock() {
                Ok(mut g) => {
                    if g.skip_screenshot_frames > 0 {
                        g.skip_screenshot_frames -= 1;
                        ctx.send_viewport_cmd_to(
                            editor_viewport_id(),
                            ViewportCommand::Visible(false),
                        );
                        ctx.send_viewport_cmd_to(editor_viewport_id(), ViewportCommand::Close);
                        ctx.request_repaint();
                        false
                    } else {
                        if g.screenshot.is_none() {
                            // 截屏瞬间再枚举一次，拿最新 rcWork（任务栏位置/自动隐藏可能变）
                            let displays = platform::list_displays();
                            let d = g
                                .editor_display
                                .as_ref()
                                .and_then(|prev| {
                                    displays.iter().find(|x| x.id == prev.id).cloned()
                                })
                                .or_else(|| {
                                    displays
                                        .iter()
                                        .find(|d| d.primary)
                                        .cloned()
                                })
                                .or_else(|| displays.into_iter().next());
                            if let Some(d) = d {
                                g.editor_display = Some(d.clone());
                                g.screenshot = platform::capture_screen_rgba(
                                    d.x, d.y, d.width, d.height,
                                )
                                .map(|(w, h, rgba)| {
                                    ColorImage::from_rgba_unmultiplied(
                                        [w as usize, h as usize],
                                        &rgba,
                                    )
                                });
                                g.bg_luma = g
                                    .screenshot
                                    .as_ref()
                                    .map(sample_image_luma)
                                    .unwrap_or(0.25);
                                g.screenshot_tex = None;
                            }
                        }
                        if g.screenshot_tex.is_none() {
                            if let Some(img) = g.screenshot.clone() {
                                g.screenshot_tex = Some(ctx.load_texture(
                                    "deskhud_hud_layout_shot",
                                    img,
                                    TextureOptions::LINEAR,
                                ));
                            }
                        }
                        true
                    }
                }
                Err(_) => false,
            };

            if editor_ready {
                show_editor(
                    host,
                    ctx,
                    layout_source,
                    hud_items,
                    done_label,
                    cancel_label,
                    reset_label,
                    reset_size_label,
                    hint_label,
                );
            }
        } else {
            // 非编辑态：每帧强制关编辑视口（子窗 Close 不可靠）
            force_close_editor(ctx, host);
            show_slots(host, ctx, layout_source, hud_items, topmost);
        }
    }
}

/// 从 ROOT 关掉布局编辑视口，并清掉编辑资源。
pub fn force_close_editor(ctx: &egui::Context, host: &Arc<Mutex<HudOverlayHost>>) {
    ctx.send_viewport_cmd_to(editor_viewport_id(), ViewportCommand::Visible(false));
    ctx.send_viewport_cmd_to(editor_viewport_id(), ViewportCommand::Close);
    ctx.send_viewport_cmd_to(toolbar_viewport_id(), ViewportCommand::Close);
    if let Ok(mut g) = host.lock() {
        g.editor_open = false;
        g.screenshot = None;
        g.screenshot_tex = None;
        g.editor_display = None;
    }
}

/// 从外部（进入编辑瞬间）强制藏掉 HUD 槽窗。
pub fn suppress_hud_slots_now(
    ctx: &egui::Context,
    host: &Arc<Mutex<HudOverlayHost>>,
    hud_items: &[(&str, HudContribution)],
) {
    suppress_all_slots(host, ctx, hud_items);
}

fn suppress_all_slots(
    host: &Arc<Mutex<HudOverlayHost>>,
    ctx: &egui::Context,
    hud_items: &[(&str, HudContribution)],
) {
    let mut pairs: HashSet<(String, String)> = HashSet::new();
    if let Ok(mut guard) = host.lock() {
        pairs.extend(guard.live_slots.drain());
        // 原生立刻隐藏已缓存 HWND
        let hwnds: Vec<isize> = guard.slot_hwnd.values().copied().collect();
        for h in hwnds {
            platform::set_window_visible(h, false);
        }
        guard.slot_hwnd.clear();
        guard.slot_click_through.clear();
    }
    for (pid, c) in hud_items {
        pairs.insert(((*pid).to_string(), c.id.to_string()));
    }
    for (p, c) in pairs {
        let vp = slot_viewport_id(&p, &c);
        ctx.send_viewport_cmd_to(vp, ViewportCommand::Visible(false));
        ctx.send_viewport_cmd_to(vp, ViewportCommand::Close);
        let title = format!("DeskHud HUD {p}.{c}");
        if let Some(h) = platform::find_window_by_title(&title) {
            platform::set_window_visible(h, false);
        }
    }
}

fn show_slots(
    host: &Arc<Mutex<HudOverlayHost>>,
    ctx: &egui::Context,
    active_prefs: &HudPrefs,
    hud_items: &[(&str, HudContribution)],
    topmost: bool,
) {
    let displays = platform::list_displays();
    let ppp = ctx.pixels_per_point().max(0.01);

    let mut wanted: Vec<(String, String, HudContribution, usize)> = Vec::new();
    let mut index = 0usize;
    for (pid, c) in hud_items {
        if !active_prefs.is_active(pid, c.id, c.default_enabled) {
            continue;
        }
        wanted.push(((*pid).to_string(), c.id.to_string(), c.clone(), index));
        index += 1;
    }

    let wanted_keys: HashSet<(String, String)> = wanted
        .iter()
        .map(|(p, c, _, _)| (p.clone(), c.clone()))
        .collect();
    if let Ok(mut guard) = host.lock() {
        let stale: Vec<_> = guard
            .live_slots
            .difference(&wanted_keys)
            .cloned()
            .collect();
        for (p, c) in stale {
            ctx.send_viewport_cmd_to(slot_viewport_id(&p, &c), ViewportCommand::Close);
            let key = HudPrefs::layout_key(&p, &c);
            guard.slot_hwnd.remove(&key);
            guard.slot_click_through.remove(&key);
        }
        guard.live_slots = wanted_keys.clone();
    }

    let level = if topmost {
        egui::WindowLevel::AlwaysOnTop
    } else {
        egui::WindowLevel::Normal
    };

    let topmost_changed = host
        .lock()
        .map(|mut g| {
            let changed = g.applied_topmost != Some(topmost);
            if changed {
                g.applied_topmost = Some(topmost);
            }
            changed
        })
        .unwrap_or(false);
    if topmost_changed {
        // 只在置顶变化时对已有槽窗改层级；勿在 deferred 回调里每帧 WindowLevel（易 AV）
        for (p, c) in &wanted_keys {
            ctx.send_viewport_cmd_to(slot_viewport_id(p, c), ViewportCommand::WindowLevel(level));
        }
    }

    for (pid, cid, contrib, idx) in wanted {
        let key = HudPrefs::layout_key(&pid, &cid);
        let layout = active_prefs.slot_layout(&pid, &cid, idx);
        let Some(display) = resolve_display(&displays, &layout.display) else {
            continue;
        };
        let label = slot_label(&cid, contrib.label);
        let (pos, size) = slot_outer_points(display, &layout, ppp, &cid, &label);

        let title = format!("DeskHud HUD {pid}.{cid}");
        let mut builder = egui::ViewportBuilder::default()
            .with_title(title.clone())
            .with_decorations(false)
            .with_transparent(false)
            .with_has_shadow(false)
            .with_taskbar(false)
            .with_resizable(false)
            .with_position(pos)
            .with_inner_size(size)
            .with_window_level(level);
        if topmost {
            builder = builder.with_always_on_top();
        }

        let host_c = Arc::clone(host);
        let label_c = label.clone();
        let title_c = title.clone();
        let display_c = display.clone();
        let vp_id = slot_viewport_id(&pid, &cid);
        let cid_c = cid.clone();
        let live_key = (pid.clone(), cid.clone());

        ctx.show_viewport_deferred(vp_id, builder, move |ctx, _class| {
            let dead = host_c
                .lock()
                .map(|g| g.editing || !g.live_slots.contains(&live_key))
                .unwrap_or(true);
            if dead {
                // 已被 ROOT Close / 编辑态：勿再发 WindowLevel/几何命令（关窗途中易 AV）
                return;
            }

            // 编辑/压制曾发 Visible(false)；活槽必须显式拉回，否则会一直藏着
            ctx.send_viewport_cmd(ViewportCommand::Visible(true));
            let (pos_now, size_now) =
                slot_outer_points(&display_c, &layout, ppp, &cid_c, &label_c);
            ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos_now));
            ctx.send_viewport_cmd(ViewportCommand::InnerSize(size_now));

            egui::CentralPanel::default()
                .frame(Frame::NONE.fill(Color32::from_rgb(28, 32, 40)))
                .show(ctx, |ui| {
                    if let Ok(mut guard) = host_c.lock() {
                        if !guard.slot_hwnd.contains_key(&key) {
                            if let Some(h) = platform::find_window_by_title(&title_c) {
                                platform::set_window_visible(h, true);
                                guard.slot_hwnd.insert(key.clone(), h);
                            }
                        }
                        if guard.slot_click_through.get(&key).copied() != Some(true) {
                            if let Some(&h) = guard.slot_hwnd.get(&key) {
                                platform::set_click_through(h, true);
                            }
                            guard.slot_click_through.insert(key.clone(), true);
                        }
                    }

                    let rect = ui.max_rect();
                    paint_chip(ui, rect, &label_c, layout.scale, false);
                });
        });
    }
}


fn show_editor(
    host: &Arc<Mutex<HudOverlayHost>>,
    ctx: &egui::Context,
    active_prefs: &HudPrefs,
    hud_items: &[(&str, HudContribution)],
    done_label: &str,
    cancel_label: &str,
    reset_label: &str,
    reset_size_label: &str,
    hint_label: &str,
) {
    let (display, tex_id, bg_luma) = {
        let Ok(g) = host.lock() else {
            return;
        };
        let display = g.editor_display.clone().or_else(|| {
            let displays = platform::list_displays();
            displays
                .into_iter()
                .find(|d| d.primary)
                .or_else(|| platform::list_displays().into_iter().next())
        });
        let tex_id = g.screenshot_tex.as_ref().map(|t| t.id());
        (display, tex_id, g.bg_luma)
    };
    let Some(display) = display else {
        return;
    };
    let chrome = layout_chrome(bg_luma);
    let ppp = ctx.pixels_per_point().max(0.01);
    let pos = Pos2::new(display.x as f32 / ppp, display.y as f32 / ppp);
    let size = Vec2::new(display.width as f32 / ppp, display.height as f32 / ppp);

    if let Ok(mut g) = host.lock() {
        g.editor_open = true;
        if g.editor_display.is_none() {
            g.editor_display = Some(display.clone());
        }
    }

    let mut items: Vec<(String, String, String, usize)> = Vec::new();
    let mut index = 0usize;
    for (pid, c) in hud_items {
        if !active_prefs.is_plugin_enabled(pid) {
            continue;
        }
        let label = slot_label(c.id, c.label);
        items.push(((*pid).to_string(), c.id.to_string(), label, index));
        index += 1;
    }

    let host_c = Arc::clone(host);
    let done_l = done_label.to_string();
    let cancel_l = cancel_label.to_string();
    let reset_l = reset_label.to_string();
    let reset_size_l = reset_size_label.to_string();
    let hint_l = hint_label.to_string();
    let display_c = display.clone();
    let prefs_c = active_prefs.clone();

    let builder = egui::ViewportBuilder::default()
        .with_title("DeskHud HUD Layout Editor")
        .with_decorations(false)
        .with_transparent(false)
        .with_has_shadow(false)
        .with_taskbar(false)
        .with_resizable(false)
        .with_always_on_top()
        .with_position(pos)
        .with_inner_size(size);

    ctx.show_viewport_deferred(editor_viewport_id(), builder, move |ctx, _class| {
        // 已退出编辑：立刻隐藏，避免本帧回调把窗又顶起来
        let still_editing = host_c.lock().map(|g| g.editing).unwrap_or(false);
        if !still_editing {
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            ctx.send_viewport_cmd(ViewportCommand::Close);
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            return;
        }

        ctx.send_viewport_cmd(ViewportCommand::OuterPosition(pos));
        ctx.send_viewport_cmd(ViewportCommand::InnerSize(size));
        ctx.send_viewport_cmd(ViewportCommand::WindowLevel(
            egui::WindowLevel::AlwaysOnTop,
        ));
        ctx.request_repaint();

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if let Ok(mut g) = host_c.lock() {
                g.request_cancel();
            }
            ctx.send_viewport_cmd(ViewportCommand::Visible(false));
            ctx.send_viewport_cmd(ViewportCommand::Close);
            ctx.request_repaint_of(egui::ViewportId::ROOT);
            return;
        }

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(Color32::from_rgb(18, 20, 26)))
            .show(ctx, |ui| {
                let full = ui.max_rect();
                let painter = ui.painter();
                let ppp = ui.ctx().pixels_per_point().max(0.01);

                if let Some(id) = tex_id {
                    painter.image(
                        id,
                        full,
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                        Color32::WHITE,
                    );
                } else {
                    painter.rect_filled(full, 0.0, Color32::from_rgb(22, 24, 30));
                }

                // 轻遮罩：截图底透出，模拟半透明
                painter.rect_filled(
                    full,
                    0.0,
                    Color32::from_rgba_unmultiplied(8, 10, 16, 56),
                );

                const BAR_H: f32 = 56.0;
                const BAR_INSET: f32 = 20.0;
                const CORNER_HIT: f32 = 22.0;
                let grid = LAYOUT_GRID;
                let screen = full;

                // 统一用屏幕光标 → 编辑器坐标（deferred 视口里 egui 指针常不可靠）
                let pointer = platform::cursor_screen_px().map(|(sx, sy)| {
                    Pos2::new(
                        screen.min.x + (sx - display_c.x) as f32 / ppp,
                        screen.min.y + (sy - display_c.y) as f32 / ppp,
                    )
                });
                let (lmb, _, _) = platform::global_mouse_buttons();

                // rcWork → 视口；按钮浮在顶上，不占用可布局安全区
                let work = safe_rect_from_work(screen, &display_c);
                let bar = Rect::from_min_max(
                    Pos2::new(screen.min.x + BAR_INSET, screen.min.y + BAR_INSET),
                    Pos2::new(
                        screen.max.x - BAR_INSET,
                        screen.min.y + BAR_INSET + BAR_H,
                    ),
                );
                // 仅按任务栏缩边 + 贴齐完整格子（底栏任务栏时顶部可贴到工作区最上）
                let safe = layout_safe_rect(work, screen, grid);
                let canvas = safe;

                // 任务栏等系统栏外侧加深；顶栏按钮不单独挖禁放带
                paint_unsafe_dim(painter, screen, safe);

                painter.rect_filled(
                    canvas,
                    0.0,
                    chrome.canvas_wash,
                );
                paint_grid(painter, canvas, grid, chrome.grid);
                paint_dashed_rect(
                    painter,
                    canvas.shrink(1.0),
                    chrome.safe_border,
                    1.5,
                    8.0,
                    5.0,
                );

                for (pid, cid, label, idx) in &items {
                    let key = HudPrefs::layout_key(pid, cid);
                    let layout = host_c
                        .lock()
                        .ok()
                        .and_then(|g| g.draft.get(&key).cloned())
                        .unwrap_or_else(|| prefs_c.slot_layout(pid, cid, *idx));

                    let base = base_size_points(cid, label);
                    let chip_size = Vec2::new(base.x * layout.scale, base.y * layout.scale);
                    let chip_pos = Pos2::new(
                        screen.min.x + layout.x * screen.width(),
                        screen.min.y + layout.y * screen.height(),
                    );
                    let chip = Rect::from_min_size(chip_pos, chip_size);
                    let selected = host_c
                        .lock()
                        .map(|g| g.selected_key.as_deref() == Some(key.as_str()))
                        .unwrap_or(false);

                    paint_chip(ui, chip, label, layout.scale, true);
                    let border = if selected {
                        Color32::from_rgb(255, 200, 90)
                    } else {
                        Color32::from_rgb(90, 170, 255)
                    };
                    paint_dashed_rect(ui.painter(), chip.expand(2.0), border, 1.25, 6.0, 4.0);

                    // 四角透明热区（不画方块）；整块移动避开四角
                    let corners = corner_hit_rects(chip, CORNER_HIT);
                    let on_corner = pointer
                        .map(|p| corners.iter().any(|(_, r)| r.contains(p)))
                        .unwrap_or(false);
                    let move_id = ui.id().with(("hud_ed_mv", pid.as_str(), cid.as_str()));
                    let mut scale_started: Option<ScaleCorner> = None;
                    for (corner, rect) in &corners {
                        let id = ui
                            .id()
                            .with(("hud_ed_sc", pid.as_str(), cid.as_str(), *corner as u8));
                        let resp = ui.interact(*rect, id, Sense::drag());
                        if resp.hovered() || resp.dragged() {
                            ui.ctx().set_cursor_icon(match corner {
                                ScaleCorner::Nw | ScaleCorner::Se => {
                                    egui::CursorIcon::ResizeNwSe
                                }
                                ScaleCorner::Ne | ScaleCorner::Sw => {
                                    egui::CursorIcon::ResizeNeSw
                                }
                            });
                        }
                        if resp.drag_started() {
                            scale_started = Some(*corner);
                        }
                    }
                    // 中部可点选/拖移（四角留给缩放）
                    let move_rect = chip.shrink(CORNER_HIT * 0.5);
                    let move_resp = ui.interact(
                        move_rect,
                        move_id,
                        Sense::click_and_drag(),
                    );

                    let mut guard = match host_c.lock() {
                        Ok(g) => g,
                        Err(_) => continue,
                    };

                    let lmb_pressed = lmb && !guard.lmb_prev;

                    if move_resp.clicked() {
                        guard.selected_key = Some(key.clone());
                    }

                    // 启动拖拽：优先 egui，回退到全局 LMB 按下边沿（子视口更稳）
                    if guard.drag_key.is_none() {
                        if let Some(corner) = scale_started {
                            guard.drag_key = Some(key.clone());
                            guard.drag_mode = DragMode::Scale;
                            guard.drag_scale_corner = corner;
                            guard.drag_origin_layout = layout.clone();
                            guard.drag_fixed_pos = opposite_corner_pos(chip, corner);
                            guard.selected_key = Some(key.clone());
                        } else if move_resp.drag_started() && !on_corner {
                            guard.drag_key = Some(key.clone());
                            guard.drag_mode = DragMode::Move;
                            guard.drag_origin_layout = layout.clone();
                            guard.selected_key = Some(key.clone());
                            if let Some(p) = pointer {
                                guard.drag_grab_offset = p - chip.min;
                            } else {
                                guard.drag_grab_offset = chip_size * 0.5;
                            }
                        } else if lmb_pressed {
                            if let Some(p) = pointer {
                                if let Some(&(corner, _)) =
                                    corners.iter().find(|(_, r)| r.contains(p))
                                {
                                    guard.drag_key = Some(key.clone());
                                    guard.drag_mode = DragMode::Scale;
                                    guard.drag_scale_corner = corner;
                                    guard.drag_origin_layout = layout.clone();
                                    guard.drag_fixed_pos = opposite_corner_pos(chip, corner);
                                    guard.selected_key = Some(key.clone());
                                } else if move_rect.contains(p) {
                                    guard.drag_key = Some(key.clone());
                                    guard.drag_mode = DragMode::Move;
                                    guard.drag_origin_layout = layout.clone();
                                    guard.selected_key = Some(key.clone());
                                    guard.drag_grab_offset = p - chip.min;
                                }
                            }
                        }
                    }

                    if guard.drag_key.as_deref() == Some(key.as_str()) {
                        let mut next = guard
                            .draft
                            .get(&key)
                            .cloned()
                            .unwrap_or_else(|| guard.drag_origin_layout.clone());
                        match guard.drag_mode {
                            DragMode::Move => {
                                if let Some(p) = pointer {
                                    next = move_layout_follow_pointer(
                                        p,
                                        guard.drag_grab_offset,
                                        Vec2::new(base.x * next.scale, base.y * next.scale),
                                        screen,
                                        canvas,
                                        grid,
                                        &display_c.id,
                                        next.scale,
                                    );
                                }
                            }
                            DragMode::Scale => {
                                if let Some(p) = pointer {
                                    next = scale_layout_follow_pointer(
                                        guard.drag_fixed_pos,
                                        guard.drag_scale_corner,
                                        p,
                                        base,
                                        screen,
                                        canvas,
                                        grid,
                                        &display_c.id,
                                    );
                                }
                            }
                            DragMode::None => {}
                        }
                        next.display = display_c.id.clone();
                        let chip_now = Vec2::new(base.x * next.scale, base.y * next.scale);
                        next = clamp_layout_in_canvas(next, chip_now, screen, canvas);
                        guard.draft.insert(key.clone(), next);

                        if !lmb {
                            guard.drag_key = None;
                            guard.drag_mode = DragMode::None;
                        }
                    }
                }

                // 本帧结束后更新全局左键边沿（所有条目共用）
                if let Ok(mut g) = host_c.lock() {
                    g.lmb_prev = lmb;
                }

                // 选中条目旁跟随「重置大小」
                let selected_chip = host_c.lock().ok().and_then(|g| {
                    let key = g.selected_key.clone()?;
                    let layout = g.draft.get(&key)?.clone();
                    let (pid, cid) = key.rsplit_once('.')?;
                    let label = items
                        .iter()
                        .find(|(p, c, _, _)| p == pid && c == cid)
                        .map(|(_, _, l, _)| l.as_str())
                        .unwrap_or(cid);
                    let base = base_size_points(cid, label);
                    let chip_size = Vec2::new(base.x * layout.scale, base.y * layout.scale);
                    let chip_pos = Pos2::new(
                        screen.min.x + layout.x * screen.width(),
                        screen.min.y + layout.y * screen.height(),
                    );
                    Some(Rect::from_min_size(chip_pos, chip_size))
                });
                if let Some(chip) = selected_chip {
                    const RS: f32 = 28.0;
                    const RS_GAP: f32 = 8.0;
                    let mut btn = Rect::from_min_size(
                        Pos2::new(chip.center().x - RS * 0.5, chip.min.y - RS_GAP - RS),
                        Vec2::splat(RS),
                    );
                    // 上方放不下则改到下方
                    if btn.min.y < canvas.min.y {
                        btn = Rect::from_min_size(
                            Pos2::new(chip.center().x - RS * 0.5, chip.max.y + RS_GAP),
                            Vec2::splat(RS),
                        );
                    }
                    // 夹进画布，避免出界
                    let max_x = (canvas.max.x - RS).max(canvas.min.x);
                    let max_y = (canvas.max.y - RS).max(canvas.min.y);
                    let origin = Pos2::new(
                        btn.min.x.clamp(canvas.min.x, max_x),
                        btn.min.y.clamp(canvas.min.y, max_y),
                    );
                    btn = Rect::from_min_size(origin, Vec2::splat(RS));

                    let resp = ui
                        .interact(btn, ui.id().with("hud_ed_reset_size"), Sense::click())
                        .on_hover_text(&reset_size_l);
                    let hovered = resp.hovered();
                    ui.painter().rect_filled(
                        btn,
                        6.0,
                        if hovered {
                            Color32::from_rgb(58, 66, 78)
                        } else {
                            Color32::from_rgb(48, 54, 64)
                        },
                    );
                    draw_reset_size_icon(
                        ui.painter(),
                        btn.center(),
                        Color32::from_rgb(220, 224, 230),
                    );
                    if resp.clicked() {
                        if let Ok(mut g) = host_c.lock() {
                            g.reset_selected_scale_to_one();
                        }
                    }
                }

                // 顶栏：提示在上，重置 / 取消 / 应用居中（距边缘有留白）
                ui.scope_builder(egui::UiBuilder::new().max_rect(bar), |ui| {
                    ui.vertical_centered(|ui| {
                        ui.add_space(4.0);
                        let hint_galley = ui.painter().layout_no_wrap(
                            hint_l.clone(),
                            FontId::proportional(11.0),
                            chrome.hint,
                        );
                        let hint_size = hint_galley.size();
                        let hint_pad = Vec2::new(10.0, 4.0);
                        let (hint_rect, _) = ui.allocate_exact_size(
                            hint_size + hint_pad * 2.0,
                            Sense::hover(),
                        );
                        ui.painter().rect_filled(
                            hint_rect,
                            6.0,
                            chrome.hint_bg,
                        );
                        ui.painter().galley(
                            hint_rect.min + hint_pad,
                            hint_galley,
                            chrome.hint,
                        );
                        ui.add_space(2.0);
                        ui.horizontal(|ui| {
                            let total_w = 88.0 + 8.0 + 88.0 + 8.0 + 96.0;
                            let side = ((ui.available_width() - total_w) * 0.5).max(0.0);
                            ui.add_space(side);

                            if ui
                                .add_sized(
                                    [88.0, 28.0],
                                    egui::Button::new(
                                        RichText::new(&reset_l)
                                            .color(Color32::from_rgb(220, 224, 230)),
                                    )
                                    .fill(Color32::from_rgb(48, 54, 64)),
                                )
                                .clicked()
                            {
                                if let Ok(mut g) = host_c.lock() {
                                    g.reset_draft();
                                }
                            }
                            ui.add_space(8.0);
                            if ui
                                .add_sized(
                                    [88.0, 28.0],
                                    egui::Button::new(
                                        RichText::new(&cancel_l)
                                            .color(Color32::from_rgb(220, 224, 230)),
                                    )
                                    .fill(Color32::from_rgb(48, 54, 64)),
                                )
                                .clicked()
                            {
                                if let Ok(mut g) = host_c.lock() {
                                    g.request_cancel();
                                }
                                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
                                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                            }
                            ui.add_space(8.0);
                            if ui
                                .add_sized(
                                    [96.0, 28.0],
                                    egui::Button::new(
                                        RichText::new(&done_l).color(Color32::WHITE),
                                    )
                                    .fill(Color32::from_rgb(46, 120, 210)),
                                )
                                .clicked()
                            {
                                if let Ok(mut g) = host_c.lock() {
                                    g.request_finish();
                                }
                                ui.ctx().send_viewport_cmd(ViewportCommand::Visible(false));
                                ui.ctx().send_viewport_cmd(ViewportCommand::Close);
                                ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                            }
                        });
                    });
                });
            });
    });
}

fn corner_hit_rects(chip: Rect, hit: f32) -> [(ScaleCorner, Rect); 4] {
    [
        (
            ScaleCorner::Nw,
            Rect::from_min_size(chip.min, Vec2::splat(hit)),
        ),
        (
            ScaleCorner::Ne,
            Rect::from_min_size(
                Pos2::new(chip.max.x - hit, chip.min.y),
                Vec2::splat(hit),
            ),
        ),
        (
            ScaleCorner::Sw,
            Rect::from_min_size(
                Pos2::new(chip.min.x, chip.max.y - hit),
                Vec2::splat(hit),
            ),
        ),
        (
            ScaleCorner::Se,
            Rect::from_min_size(chip.max - Vec2::splat(hit), Vec2::splat(hit)),
        ),
    ]
}

/// 拖动角的对角（缩放时固定）。
fn opposite_corner_pos(chip: Rect, corner: ScaleCorner) -> Pos2 {
    match corner {
        ScaleCorner::Se => chip.left_top(),
        ScaleCorner::Sw => chip.right_top(),
        ScaleCorner::Ne => chip.left_bottom(),
        ScaleCorner::Nw => chip.right_bottom(),
    }
}

/// 拖动跟手：左上角跟随（减抓取偏移），并按格吸附。
fn move_layout_follow_pointer(
    pointer: Pos2,
    grab_offset: Vec2,
    chip: Vec2,
    screen: Rect,
    canvas: Rect,
    grid: f32,
    display_id: &str,
    scale: f32,
) -> HudSlotLayout {
    let target = pointer - grab_offset;
    let sx = canvas.min.x + ((target.x - canvas.min.x) / grid).round() * grid;
    let sy = canvas.min.y + ((target.y - canvas.min.y) / grid).round() * grid;
    let mut layout = HudSlotLayout::default();
    layout.display = display_id.to_string();
    layout.x = (sx - screen.min.x) / screen.width().max(1.0);
    layout.y = (sy - screen.min.y) / screen.height().max(1.0);
    layout.scale = scale;
    clamp_layout_in_canvas(layout, chip, screen, canvas)
}

/// 拖角跟手：对角固定，尺寸按指针投影取格，自由角落在网格上。
fn scale_layout_follow_pointer(
    fixed: Pos2,
    corner: ScaleCorner,
    pointer: Pos2,
    base: Vec2,
    screen: Rect,
    canvas: Rect,
    grid: f32,
    display_id: &str,
) -> HudSlotLayout {
    // 把指针映射到「从固定角沿等比对角线」的尺度
    let (sx, sy) = match corner {
        ScaleCorner::Se => (1.0, 1.0),
        ScaleCorner::Sw => (-1.0, 1.0),
        ScaleCorner::Ne => (1.0, -1.0),
        ScaleCorner::Nw => (-1.0, -1.0),
    };
    let vx = (pointer.x - fixed.x) * sx;
    let vy = (pointer.y - fixed.y) * sy;
    let denom = base.x * base.x + base.y * base.y;
    let raw_scale = ((vx * base.x + vy * base.y) / denom.max(1.0)).max(grid / base.x.max(1.0));
    let scale = snap_scale_to_grid(raw_scale, base, grid);
    let w = base.x * scale;
    let h = base.y * scale;

    let min = match corner {
        ScaleCorner::Se => Pos2::new(fixed.x, fixed.y),
        ScaleCorner::Sw => Pos2::new(fixed.x - w, fixed.y),
        ScaleCorner::Ne => Pos2::new(fixed.x, fixed.y - h),
        ScaleCorner::Nw => Pos2::new(fixed.x - w, fixed.y - h),
    };

    let mut layout = HudSlotLayout::default();
    layout.display = display_id.to_string();
    layout.x = (min.x - screen.min.x) / screen.width().max(1.0);
    layout.y = (min.y - screen.min.y) / screen.height().max(1.0);
    layout.scale = scale;
    layout = clamp_layout_in_canvas(layout, Vec2::new(w, h), screen, canvas);

    // 夹紧后尽量把对角拉回固定点
    let chip = Rect::from_min_size(
        Pos2::new(
            screen.min.x + layout.x * screen.width(),
            screen.min.y + layout.y * screen.height(),
        ),
        Vec2::new(base.x * layout.scale, base.y * layout.scale),
    );
    let fixed_now = opposite_corner_pos(chip, corner);
    let drift = fixed - fixed_now;
    if drift.length() > 0.5 {
        layout.x += drift.x / screen.width().max(1.0);
        layout.y += drift.y / screen.height().max(1.0);
        let chip_sz = Vec2::new(base.x * layout.scale, base.y * layout.scale);
        layout = clamp_layout_in_canvas(layout, chip_sz, screen, canvas);
    }
    layout
}

/// 把显示器 `rcWork` 按比例映射到编辑视口，与全屏截图对齐（避免 ppp/窗体取整误差）。
fn safe_rect_from_work(screen: Rect, display: &platform::DisplayInfo) -> Rect {
    let dw = display.width.max(1) as f32;
    let dh = display.height.max(1) as f32;
    let l = ((display.work_left - display.x) as f32 / dw).clamp(0.0, 1.0);
    let t = ((display.work_top - display.y) as f32 / dh).clamp(0.0, 1.0);
    let r = ((display.work_right - display.x) as f32 / dw).clamp(0.0, 1.0);
    let b = ((display.work_bottom - display.y) as f32 / dh).clamp(0.0, 1.0);
    Rect::from_min_max(
        Pos2::new(
            screen.min.x + screen.width() * l,
            screen.min.y + screen.height() * t,
        ),
        Pos2::new(
            screen.min.x + screen.width() * r.max(l),
            screen.min.y + screen.height() * b.max(t),
        ),
    )
}

/// 可布局安全区：任务栏侧再往内缩 1 格；四边向内贴齐网格。
/// 顶栏按钮为浮层，不从安全区扣高度。
fn layout_safe_rect(work: Rect, screen: Rect, grid: f32) -> Rect {
    let eps = 0.5;
    let g = grid.max(1.0);
    let mut r = work;
    // 任务栏（系统栏）在哪一侧，安全边界就往内再扩满 1 格
    if work.min.x > screen.min.x + eps {
        r.min.x += g;
    }
    if work.min.y > screen.min.y + eps {
        r.min.y += g;
    }
    if work.max.x < screen.max.x - eps {
        r.max.x -= g;
    }
    if work.max.y < screen.max.y - eps {
        r.max.y -= g;
    }
    snap_rect_inward_to_grid(r, g, screen.min)
}

/// 矩形四边向内吸附到相对 `origin` 的网格线，保证内部是完整格子。
fn snap_rect_inward_to_grid(r: Rect, grid: f32, origin: Pos2) -> Rect {
    let g = grid.max(1.0);
    let min_x = origin.x + ((r.min.x - origin.x) / g).ceil() * g;
    let min_y = origin.y + ((r.min.y - origin.y) / g).ceil() * g;
    let max_x = origin.x + ((r.max.x - origin.x) / g).floor() * g;
    let max_y = origin.y + ((r.max.y - origin.y) / g).floor() * g;
    Rect::from_min_max(
        Pos2::new(min_x.min(max_x), min_y.min(max_y)),
        Pos2::new(max_x.max(min_x), max_y.max(min_y)),
    )
}

/// 「重置大小」图标：常见的逆时针环形箭头（重置）。
fn draw_reset_size_icon(painter: &egui::Painter, center: Pos2, color: Color32) {
    let r = 6.5;
    let stroke = Stroke::new(1.75, color);
    // 约 280° 开口环，箭头在开口处
    let start = -std::f32::consts::FRAC_PI_2 + 0.45;
    let end = start + std::f32::consts::TAU * 0.78;
    let steps = 20;
    let mut prev = Pos2::new(center.x + r * start.cos(), center.y + r * start.sin());
    for i in 1..=steps {
        let t = i as f32 / steps as f32;
        let a = start + (end - start) * t;
        let p = Pos2::new(center.x + r * a.cos(), center.y + r * a.sin());
        painter.line_segment([prev, p], stroke);
        prev = p;
    }
    // 箭头沿切线方向
    let tip = prev;
    let dir = Vec2::new(-end.sin(), end.cos());
    let base = tip - dir * 5.0;
    let n = Vec2::new(-dir.y, dir.x) * 3.0;
    painter.add(egui::Shape::convex_polygon(
        vec![tip, base + n, base - n],
        color,
        Stroke::NONE,
    ));
}

fn paint_unsafe_dim(painter: &egui::Painter, screen: Rect, safe: Rect) {
    let dim = Color32::from_rgba_unmultiplied(0, 0, 0, 110);
    // 上
    if safe.min.y > screen.min.y {
        painter.rect_filled(
            Rect::from_min_max(
                screen.min,
                Pos2::new(screen.max.x, safe.min.y),
            ),
            0.0,
            dim,
        );
    }
    // 下（任务栏）
    if safe.max.y < screen.max.y {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(screen.min.x, safe.max.y),
                screen.max,
            ),
            0.0,
            dim,
        );
    }
    // 左
    if safe.min.x > screen.min.x {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(screen.min.x, safe.min.y),
                Pos2::new(safe.min.x, safe.max.y),
            ),
            0.0,
            dim,
        );
    }
    // 右
    if safe.max.x < screen.max.x {
        painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(safe.max.x, safe.min.y),
                Pos2::new(screen.max.x, safe.max.y),
            ),
            0.0,
            dim,
        );
    }
}

/// 条目完全落在 canvas（可布局安全区）内。
fn clamp_layout_in_canvas(
    mut layout: HudSlotLayout,
    chip: Vec2,
    screen: Rect,
    canvas: Rect,
) -> HudSlotLayout {
    layout.scale = layout.scale.clamp(0.5, 3.0);
    let mut x = screen.min.x + layout.x * screen.width();
    let mut y = screen.min.y + layout.y * screen.height();
    let max_x = (canvas.max.x - chip.x).max(canvas.min.x);
    let max_y = (canvas.max.y - chip.y).max(canvas.min.y);
    x = x.clamp(canvas.min.x, max_x);
    y = y.clamp(canvas.min.y, max_y);
    layout.x = ((x - screen.min.x) / screen.width().max(1.0)).clamp(0.0, 1.0);
    layout.y = ((y - screen.min.y) / screen.height().max(1.0)).clamp(0.0, 1.0);
    layout
}

/// 缩放对齐网格：宽度每次只增减 **1 格**（等比高度随之变）。
fn snap_scale_to_grid(scale: f32, base: Vec2, grid: f32) -> f32 {
    let min_wu = ((0.5 * base.x) / grid).ceil().max(1.0);
    let max_wu = ((3.0 * base.x) / grid).floor().max(min_wu);
    let wu = ((base.x * scale) / grid).round().clamp(min_wu, max_wu);
    (wu * grid / base.x.max(1.0)).clamp(0.5, 3.0)
}

fn paint_grid(painter: &egui::Painter, rect: Rect, grid: f32, color: Color32) {
    let stroke = Stroke::new(1.0, color);
    let mut x = rect.min.x;
    while x <= rect.max.x + 0.5 {
        painter.line_segment(
            [Pos2::new(x, rect.min.y), Pos2::new(x, rect.max.y)],
            stroke,
        );
        x += grid;
    }
    let mut y = rect.min.y;
    while y <= rect.max.y + 0.5 {
        painter.line_segment(
            [Pos2::new(rect.min.x, y), Pos2::new(rect.max.x, y)],
            stroke,
        );
        y += grid;
    }
}

/// 截图平均亮度（相对亮度，0..=1）。
fn sample_image_luma(img: &ColorImage) -> f32 {
    let n = img.pixels.len().max(1);
    let step = (n / 5000).max(1);
    let mut sum = 0.0f64;
    let mut count = 0u64;
    for px in img.pixels.iter().step_by(step) {
        let r = f64::from(px.r()) / 255.0;
        let g = f64::from(px.g()) / 255.0;
        let b = f64::from(px.b()) / 255.0;
        sum += 0.2126 * r + 0.7152 * g + 0.0722 * b;
        count += 1;
    }
    (sum / count.max(1) as f64) as f32
}

/// 叠加编辑遮罩后的有效亮度，再选对比色。
fn layout_chrome(raw_luma: f32) -> LayoutChrome {
    // 与编辑器遮罩 `rgba(8,10,16,56)` 一致
    const OVERLAY_A: f32 = 56.0 / 255.0;
    let overlay_luma =
        (0.2126 * 8.0 + 0.7152 * 10.0 + 0.0722 * 16.0) / 255.0;
    let effective = (1.0 - OVERLAY_A) * raw_luma.clamp(0.0, 1.0) + OVERLAY_A * overlay_luma;
    if effective > 0.52 {
        // 浅色桌面 → 深色网格/提示
        LayoutChrome {
            grid: Color32::from_rgba_unmultiplied(18, 28, 44, 78),
            safe_border: Color32::from_rgba_unmultiplied(20, 70, 150, 210),
            hint: Color32::from_rgb(28, 34, 44),
            hint_bg: Color32::from_rgba_unmultiplied(255, 255, 255, 165),
            canvas_wash: Color32::from_rgba_unmultiplied(255, 255, 255, 22),
        }
    } else {
        // 深色桌面 → 浅色网格/提示
        LayoutChrome {
            grid: Color32::from_rgba_unmultiplied(210, 225, 255, 58),
            safe_border: Color32::from_rgb(120, 180, 255),
            hint: Color32::from_rgb(230, 234, 242),
            hint_bg: Color32::from_rgba_unmultiplied(8, 10, 16, 150),
            canvas_wash: Color32::from_rgba_unmultiplied(36, 44, 58, 24),
        }
    }
}

#[derive(Clone, Copy)]
struct LayoutChrome {
    grid: Color32,
    safe_border: Color32,
    hint: Color32,
    hint_bg: Color32,
    canvas_wash: Color32,
}

fn paint_dashed_rect(
    painter: &egui::Painter,
    rect: Rect,
    color: Color32,
    thickness: f32,
    dash: f32,
    gap: f32,
) {
    let stroke = Stroke::new(thickness, color);
    let segs = [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ];
    for (a, b) in segs {
        paint_dashed_segment(painter, a, b, stroke, dash, gap);
    }
}

fn paint_dashed_segment(
    painter: &egui::Painter,
    a: Pos2,
    b: Pos2,
    stroke: Stroke,
    dash: f32,
    gap: f32,
) {
    let delta = b - a;
    let len = delta.length();
    if len < 1.0 {
        return;
    }
    let dir = delta / len;
    let mut t = 0.0;
    while t < len {
        let t1 = (t + dash).min(len);
        painter.line_segment([a + dir * t, a + dir * t1], stroke);
        t += dash + gap;
    }
}

fn paint_chip(ui: &mut egui::Ui, rect: Rect, label: &str, scale: f32, editing: bool) {
    let fill = if editing {
        Color32::from_rgb(42, 105, 175)
    } else {
        Color32::from_rgb(45, 52, 64)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, CornerRadius::same(8), fill);
    painter.rect_stroke(
        rect.shrink(0.5),
        CornerRadius::same(8),
        Stroke::new(1.0, Color32::from_rgb(70, 78, 92)),
        egui::StrokeKind::Inside,
    );
    let font_size = (13.0 * scale).clamp(10.0, 36.0);
    painter.text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(font_size),
        Color32::from_rgb(245, 247, 250),
    );
}

fn slot_outer_points(
    display: &DisplayInfo,
    layout: &HudSlotLayout,
    ppp: f32,
    contrib_id: &str,
    label: &str,
) -> (Pos2, Vec2) {
    let base = base_size_points(contrib_id, label);
    let w = base.x * layout.scale;
    let h = base.y * layout.scale;
    let px = display.x as f32 + layout.x * display.width as f32;
    let py = display.y as f32 + layout.y * display.height as f32;
    (Pos2::new(px / ppp, py / ppp), Vec2::new(w, h))
}

fn resolve_display<'a>(displays: &'a [DisplayInfo], id: &str) -> Option<&'a DisplayInfo> {
    displays
        .iter()
        .find(|d| d.id == id)
        .or_else(|| displays.iter().find(|d| d.primary))
        .or_else(|| displays.first())
}

fn slot_label(id: &str, fallback: &str) -> String {
    match id {
        "clock" => {
            use std::time::{SystemTime, UNIX_EPOCH};
            let secs = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let m = (secs / 60) % 60;
            let s = secs % 60;
            format!("时钟 {m:02}:{s:02}")
        }
        "tip" => "DeskHud 演示".into(),
        _ => fallback.to_string(),
    }
}
