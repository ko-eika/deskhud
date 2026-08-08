//! 统一设置窗：侧栏（常规 / 宠物 / 插件）+ 右侧内容。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{
    self, Align2, Color32, ColorImage, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin,
    RichText, Sense, Stroke, TextureHandle, TextureOptions, Vec2,
};
use deskhud_host::{HudContribution, PetConfigOption, PetKindInfo, PluginInfo};
use deskhud_ui::{Locale, MessageKey, PetPickerMode, ShellPrefs, UiPreferences};

fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("deskhud_settings")
}

const SIDE_W: f32 = 168.0;
const CARD_GAP: f32 = 12.0;
const CARD_MIN_W: f32 = 156.0;
/// 允许卡片随容器拉宽，优先贴齐左右边缘。
const CARD_MAX_W: f32 = 280.0;
const CARD_PAD: f32 = 12.0;
/// 卡片底部文案区固定高度（标题 / 描述 / 尺寸）。
const CARD_TEXT_H: f32 = 78.0;
/// 内容区宽度稳定多久后才重算卡片（系统拖边改尺寸时 pointer 不可靠）。
const CARD_LAYOUT_SETTLE: Duration = Duration::from_millis(220);
/// 忽略滚动条出现等造成的微小宽度抖动，避免布局来回抖。
const CARD_LAYOUT_DEADBAND: f32 = 16.0;

/// 插件头与配置项共用的图标列，保证垂直对齐。
mod plugin_layout {
    /// 展开箭头列宽。
    pub const CHEV_W: f32 = 22.0;
    /// 箭头与插件图标间距。
    pub const CHEV_TO_ICON: f32 = 4.0;
    /// 插件 / 配置项图标边长（同尺寸才对齐）。
    pub const ICON: f32 = 32.0;
    /// 图标与文案间距。
    pub const ICON_TO_TEXT: f32 = 8.0;

    /// 图标列左边缘相对卡片内容左边缘的偏移。
    pub fn icon_left() -> f32 {
        CHEV_W + CHEV_TO_ICON
    }
}

mod tone {
    use eframe::egui::Color32;

    pub const BG: Color32 = Color32::from_rgb(246, 247, 249);
    pub const SIDE: Color32 = Color32::from_rgb(236, 238, 243);
    pub const CARD: Color32 = Color32::from_rgb(255, 255, 255);
    pub const TEXT: Color32 = Color32::from_rgb(28, 30, 36);
    pub const MUTED: Color32 = Color32::from_rgb(110, 114, 124);
    pub const LINE: Color32 = Color32::from_rgb(222, 225, 232);
    pub const ACCENT: Color32 = Color32::from_rgb(47, 110, 220);
    pub const ACCENT_HOVER: Color32 = Color32::from_rgb(70, 132, 236);
    pub const ACCENT_PRESS: Color32 = Color32::from_rgb(36, 90, 190);
    pub const ACCENT_SOFT: Color32 = Color32::from_rgb(232, 240, 255);
    pub const STAGE: Color32 = Color32::from_rgb(228, 232, 240);
    pub const SELECTED_RING: Color32 = Color32::from_rgb(70, 120, 220);
    pub const HOVER: Color32 = Color32::from_rgb(242, 244, 248);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Pet,
    Hud,
    General,
}

#[derive(Clone)]
pub struct SettingsHost {
    inner: Arc<Mutex<SettingsState>>,
}

pub struct SettingsState {
    pub open: bool,
    /// 编辑中的草稿。
    pub prefs: UiPreferences,
    /// 打开设置时 / 上次「应用」后的基准（「重置」恢复到此）。
    pub baseline: UiPreferences,
    pub tab: SettingsTab,
    pub pets: Vec<PetKindInfo>,
    /// 宠 id → 可配置项。
    pub pet_options: HashMap<String, Vec<PetConfigOption>>,
    pub plugins: Vec<PluginInfo>,
    pub hud_items: Vec<(&'static str, HudContribution)>,
    pub locale_dirty: bool,
    /// 打开后需要 Focus 一次。
    focus_once: bool,
    /// 打开后需要下发一次尺寸 / 位置。
    place_once: bool,
    /// 用户点了「应用」：主壳应把草稿写入运行态。
    pub apply_requested: bool,
    /// 关闭时几何已写入 `prefs`，待主壳落盘（取消时只同步几何）。
    pub pending_flush: bool,
    /// 取消关闭：丢弃草稿，勿把未应用的改动写入主壳。
    pub discard_draft: bool,
    /// 宠物卡片布局缓存（宽度变化后需稳定一段时间才重算）。
    card_layout: Option<(usize, f32, f32, f32)>,
    /// 当前布局对应的内容区宽。
    card_layout_for_w: f32,
    /// 最近观测到的内容区宽（拖边过程中持续变化）。
    card_observe_w: f32,
    /// `card_observe_w` 上次变化时间；稳定超过 `CARD_LAYOUT_SETTLE` 后应用。
    card_observe_since: Option<Instant>,
    /// 已为当前 settle 预约过一次重绘，避免每帧 request_repaint。
    card_settle_repaint_armed: bool,
    preview_textures: HashMap<String, TextureHandle>,
}

impl SettingsHost {
    pub fn new(prefs: UiPreferences) -> Self {
        Self {
            inner: Arc::new(Mutex::new(SettingsState {
                open: false,
                prefs: prefs.clone(),
                baseline: prefs,
                tab: SettingsTab::General,
                pets: Vec::new(),
                pet_options: HashMap::new(),
                plugins: Vec::new(),
                hud_items: Vec::new(),
                locale_dirty: false,
                focus_once: false,
                place_once: false,
                apply_requested: false,
                pending_flush: false,
                discard_draft: false,
                card_layout: None,
                card_layout_for_w: 0.0,
                card_observe_w: 0.0,
                card_observe_since: None,
                card_settle_repaint_armed: false,
                preview_textures: HashMap::new(),
            })),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, SettingsState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_open(&self) -> bool {
        self.lock().open
    }

    pub fn open(
        &self,
        prefs: &UiPreferences,
        pets: Vec<PetKindInfo>,
        pet_options: HashMap<String, Vec<PetConfigOption>>,
        plugins: Vec<PluginInfo>,
        hud_items: Vec<(&'static str, HudContribution)>,
        tab: SettingsTab,
    ) {
        let mut s = self.lock();
        s.open = true;
        s.prefs = prefs.clone();
        s.baseline = prefs.clone();
        s.pets = pets;
        s.pet_options = pet_options;
        s.plugins = plugins;
        s.hud_items = hud_items;
        s.tab = tab;
        s.focus_once = true;
        s.place_once = true;
        s.apply_requested = false;
        s.pending_flush = false;
        s.discard_draft = false;
        s.card_layout = None;
        s.card_layout_for_w = 0.0;
        s.card_observe_w = 0.0;
        s.card_observe_since = None;
        s.card_settle_repaint_armed = false;
        s.preview_textures.clear();
    }

    pub fn show(&self, ctx: &egui::Context, _pet_hwnd: Option<isize>) {
        let title = {
            let s = self.lock();
            if !s.open {
                return;
            }
            s.prefs.t(MessageKey::SettingsTitle).to_string()
        };

        let shared = self.clone();
        ctx.show_viewport_deferred(
            viewport_id(),
            egui::ViewportBuilder::default()
                .with_title(title.clone())
                .with_decorations(true)
                .with_transparent(false)
                .with_resizable(true)
                .with_min_inner_size([ShellPrefs::SETTINGS_MIN_W, ShellPrefs::SETTINGS_MIN_H])
                .with_taskbar(true)
                .with_visible(true)
                .with_icon(crate::icon()),
            move |ui, _| shared.draw(ui),
        );

        // 延迟视口首次 builder 标题不会随 locale 草稿更新；每帧同步
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Title(title));

        let (place_once, focus_once, size, pos) = {
            let s = self.lock();
            (
                s.place_once,
                s.focus_once,
                s.prefs.shell.settings_size(),
                s.prefs.shell.settings_pos(),
            )
        };
        if place_once {
            self.lock().place_once = false;
            ctx.send_viewport_cmd_to(
                viewport_id(),
                egui::ViewportCommand::MinInnerSize(egui::vec2(
                    ShellPrefs::SETTINGS_MIN_W,
                    ShellPrefs::SETTINGS_MIN_H,
                )),
            );
            ctx.send_viewport_cmd_to(
                viewport_id(),
                egui::ViewportCommand::InnerSize(egui::vec2(size[0], size[1])),
            );
            if let Some([x, y]) = pos {
                ctx.send_viewport_cmd_to(
                    viewport_id(),
                    egui::ViewportCommand::OuterPosition(egui::pos2(x, y)),
                );
            }
        }
        if focus_once {
            self.lock().focus_once = false;
            ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
        }
    }

    fn draw(&self, ui: &mut egui::Ui) {
        // clear_color 对子窗也是透明；必须铺满不透明底，否则发黑且点击异常
        let fill = tone::BG;
        ui.painter()
            .rect_filled(ui.max_rect(), CornerRadius::ZERO, fill);

        // 主壳为透明宠窗把全局 window_fill 设成透明；设置窗内控件/弹出层需不透明
        opaque_settings_visuals(ui);

        let mut close = false;
        let ctx = ui.ctx().clone();

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            close = true;
        }

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(fill).inner_margin(0.0))
            .show(ui, |ui| {
                egui::Panel::left("deskhud_settings_nav")
                    .exact_size(SIDE_W)
                    .resizable(false)
                    .frame(Frame::NONE.fill(tone::SIDE).inner_margin(Margin::symmetric(12, 16)))
                    .show(ui, |ui| {
                        self.draw_sidebar(ui);
                    });

                egui::Panel::bottom("deskhud_settings_footer")
                    .exact_size(56.0)
                    .resizable(false)
                    .frame(
                        Frame::NONE
                            .fill(fill)
                            .stroke(Stroke::new(1.0, tone::LINE))
                            .inner_margin(Margin::symmetric(20, 12)),
                    )
                    .show(ui, |ui| {
                        self.draw_footer(ui, &mut close);
                    });

                egui::CentralPanel::default()
                    .frame(Frame::NONE.fill(fill).inner_margin(Margin::symmetric(24, 18)))
                    .show(ui, |ui| {
                        // 预留滚动条宽度，避免条出现/消失导致卡片列宽抖动；不强制 AlwaysVisible（透明底会露黑条）
                        egui::ScrollArea::vertical()
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                let reserve = ui.spacing().scroll.bar_width
                                    + ui.spacing().scroll.bar_outer_margin * 2.0;
                                ui.set_width((ui.available_width() - reserve).max(0.0));
                                self.draw_content(ui);
                            });
                    });
            });

        let user_close = ctx.input(|i| i.viewport().close_requested());
        if close || user_close {
            self.capture_geometry(ui);
            self.close_viewport(&ctx, true);
        }
    }

    fn close_viewport(&self, ctx: &egui::Context, discard: bool) {
        let mut s = self.lock();
        s.open = false;
        s.focus_once = false;
        s.place_once = false;
        s.discard_draft = discard;
        s.pending_flush = true;
        drop(s);
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Visible(false));
    }

    fn capture_geometry(&self, ui: &egui::Ui) {
        let (inner, outer) = ui.ctx().input(|i| {
            let v = i.viewport();
            (v.inner_rect, v.outer_rect)
        });
        let Some(inner) = inner else {
            return;
        };
        let Some(outer) = outer else {
            return;
        };
        let w = inner.width();
        let h = inner.height();
        if w < 200.0 || h < 160.0 {
            return;
        }
        let mut s = self.lock();
        let cur = (
            s.prefs.shell.settings_width,
            s.prefs.shell.settings_height,
            s.prefs.shell.settings_pos_x,
            s.prefs.shell.settings_pos_y,
        );
        let changed = match (cur.0, cur.1, cur.2, cur.3) {
            (Some(cw), Some(ch), Some(cx), Some(cy)) => {
                (cw - w).abs() > 1.0
                    || (ch - h).abs() > 1.0
                    || (cx - outer.left()).abs() > 1.0
                    || (cy - outer.top()).abs() > 1.0
            }
            _ => true,
        };
        if changed {
            s.prefs
                .shell
                .set_settings_geometry(w, h, outer.left(), outer.top());
        }
    }

    fn draw_sidebar(&self, ui: &mut egui::Ui) {
        let (title, tab) = {
            let s = self.lock();
            (s.prefs.t(MessageKey::SettingsTitle).to_string(), s.tab)
        };

        ui.label(
            RichText::new(title)
                .size(18.0)
                .strong()
                .color(tone::TEXT),
        );
        ui.add_space(4.0);
        ui.label(RichText::new("DeskHud").size(11.5).color(tone::MUTED));
        ui.add_space(18.0);

        for (next, key) in [
            (SettingsTab::General, MessageKey::SettingsNavGeneral),
            (SettingsTab::Pet, MessageKey::SettingsNavPet),
            (SettingsTab::Hud, MessageKey::SettingsNavHud),
        ] {
            let label = self.lock().prefs.t(key).to_string();
            let selected = tab == next;
            if nav_item(ui, &label, selected).clicked() {
                let mut s = self.lock();
                s.tab = next;
                if next == SettingsTab::Pet {
                    s.card_layout = None;
                    s.card_layout_for_w = 0.0;
                    s.card_observe_w = 0.0;
                    s.card_observe_since = None;
                    s.card_settle_repaint_armed = false;
                }
            }
            ui.add_space(6.0);
        }
    }

    fn draw_footer(&self, ui: &mut egui::Ui, close: &mut bool) {
        let (reset_l, apply_l, cancel_l) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::ActionReset).to_string(),
                s.prefs.t(MessageKey::ActionApply).to_string(),
                s.prefs.t(MessageKey::ActionCancel).to_string(),
            )
        };
        // right_to_left：先画的在最右 → 视觉从左到右为 重置 / 应用 / 取消
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if footer_secondary_button(ui, &cancel_l).clicked() {
                *close = true;
            }
            ui.add_space(8.0);
            if footer_primary_button(ui, &apply_l).clicked() {
                let mut s = self.lock();
                s.baseline = s.prefs.clone();
                s.apply_requested = true;
            }
            ui.add_space(8.0);
            if footer_secondary_button(ui, &reset_l).clicked() {
                let mut s = self.lock();
                let mode = s.prefs.shell.pet_picker_mode;
                let geo = (
                    s.prefs.shell.settings_width,
                    s.prefs.shell.settings_height,
                    s.prefs.shell.settings_pos_x,
                    s.prefs.shell.settings_pos_y,
                );
                s.prefs = s.baseline.clone();
                s.prefs.shell.pet_picker_mode = mode;
                s.prefs.shell.settings_width = geo.0;
                s.prefs.shell.settings_height = geo.1;
                s.prefs.shell.settings_pos_x = geo.2;
                s.prefs.shell.settings_pos_y = geo.3;
                s.card_layout = None;
            }
        });
    }

    fn draw_content(&self, ui: &mut egui::Ui) {
        let tab = self.lock().tab;
        match tab {
            SettingsTab::General => self.draw_general_page(ui),
            SettingsTab::Pet => self.draw_pet_page(ui),
            SettingsTab::Hud => self.draw_hud_page(ui),
        }
    }

    fn draw_pet_page(&self, ui: &mut egui::Ui) {
        let (nav, intro, pets, active, size_key, selected_badge, mode, author_l, homepage_l) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsNavPet).to_string(),
                s.prefs.t(MessageKey::SettingsPetIntro).to_string(),
                s.pets.clone(),
                s.prefs.shell.active_pet_kind_id.clone(),
                s.prefs.t(MessageKey::SettingsPetWindowSize).to_string(),
                s.prefs.t(MessageKey::SettingsPetSelected).to_string(),
                s.prefs.shell.pet_picker_mode,
                s.prefs.t(MessageKey::MetaAuthor).to_string(),
                s.prefs.t(MessageKey::MetaHomepage).to_string(),
            )
        };

        // 标题单独一行；说明靠左、视图切换靠右，与下方内容左右对齐
        ui.label(
            RichText::new(&nav)
                .size(22.0)
                .strong()
                .color(tone::TEXT),
        );
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.set_height(30.0);
            ui.add(
                egui::Label::new(RichText::new(&intro).size(13.0).color(tone::MUTED)).truncate(),
            );
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(m) = view_mode_icon_group(ui, mode) {
                    let mut s = self.lock();
                    s.prefs.shell.pet_picker_mode = m;
                    s.card_layout = None;
                }
            });
        });
        ui.add_space(14.0);

        let ctx = ui.ctx().clone();
        {
            let mut s = self.lock();
            for pet in &pets {
                let _ = ensure_preview_texture(&ctx, &mut s.preview_textures, pet);
            }
        }
        let textures = self.lock().preview_textures.clone();
        let mode = self.lock().prefs.shell.pet_picker_mode;

        let mut pick: Option<String> = None;
        match mode {
            PetPickerMode::Grid => {
                let avail_w = ui.available_width();
                let (cols, card_w, card_h, preview_side) = {
                    let mut s = self.lock();
                    resolve_card_layout(&mut s, avail_w)
                };
                {
                    let mut s = self.lock();
                    if s.card_observe_since.is_some() && !s.card_settle_repaint_armed {
                        s.card_settle_repaint_armed = true;
                        ui.ctx().request_repaint_after(
                            CARD_LAYOUT_SETTLE + Duration::from_millis(16),
                        );
                    }
                }

                let mut idx = 0usize;
                while idx < pets.len() {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = CARD_GAP;
                        for col in 0..cols {
                            let i = idx + col;
                            if i >= pets.len() {
                                break;
                            }
                            let pet = &pets[i];
                            let selected = pet.id == active;
                            let size_label = format!(
                                "{}  {:.0}×{:.0}",
                                size_key, pet.window_width, pet.window_height
                            );
                            let author_label = format!("{}  {}", author_l, pet.author);
                            let tex = textures.get(&pet_preview_key(pet.id));
                            let resp = pet_preview_card(
                                ui,
                                card_w,
                                card_h,
                                preview_side,
                                pet.display_name,
                                pet.description,
                                pet.id,
                                &author_label,
                                pet.author,
                                &author_l,
                                &size_label,
                                &selected_badge,
                                selected,
                                tex,
                                pet.homepage,
                                &homepage_l,
                            );
                            if resp.clicked() {
                                pick = Some(pet.id.to_string());
                            }
                        }
                    });
                    idx += cols;
                    if idx < pets.len() {
                        ui.add_space(CARD_GAP);
                    }
                }
            }
            PetPickerMode::List => {
                for (i, pet) in pets.iter().enumerate() {
                    if i > 0 {
                        ui.add_space(8.0);
                    }
                    let selected = pet.id == active;
                    let size_label = format!(
                        "{}  {:.0}×{:.0}",
                        size_key, pet.window_width, pet.window_height
                    );
                    let author_label = format!("{}  {}", author_l, pet.author);
                    let tex = textures.get(&pet_preview_key(pet.id));
                    let resp = pet_list_row(
                        ui,
                        pet.display_name,
                        pet.description,
                        pet.id,
                        &author_label,
                        pet.author,
                        &author_l,
                        &size_label,
                        &selected_badge,
                        selected,
                        tex,
                        pet.homepage,
                        &homepage_l,
                    );
                    if resp.clicked() {
                        pick = Some(pet.id.to_string());
                    }
                }
            }
        }

        if let Some(id) = pick {
            let mut s = self.lock();
            if let Some(pet) = s.pets.iter().find(|p| p.id == id) {
                let (w, h) = (pet.window_width, pet.window_height);
                s.prefs.shell.active_pet_kind_id = id;
                s.prefs.shell.apply_pet_window_size(w, h);
            }
        }

        ui.add_space(16.0);
        self.draw_active_pet_options(ui);
    }

    fn draw_active_pet_options(&self, ui: &mut egui::Ui) {
        let (active_id, options, options_title) = {
            let s = self.lock();
            let id = s.prefs.shell.active_pet_kind_id.clone();
            let opts = s.pet_options.get(&id).cloned().unwrap_or_default();
            let title = s.prefs.t(MessageKey::SettingsPetOptions).to_string();
            (id, opts, title)
        };
        if options.is_empty() {
            return;
        }

        ui.label(
            RichText::new(&options_title)
                .size(16.0)
                .strong()
                .color(tone::TEXT),
        );
        ui.add_space(10.0);
        section_card(ui, |ui| {
            for (i, opt) in options.iter().enumerate() {
                if i > 0 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                }
                let mut on = self
                    .lock()
                    .prefs
                    .pet
                    .get_option(&active_id, opt.key, opt.default);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(opt.label)
                                .size(13.5)
                                .strong()
                                .color(tone::TEXT),
                        );
                        ui.label(
                            RichText::new(opt.description)
                                .size(12.0)
                                .color(tone::MUTED),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if toggle_switch(ui, &mut on).changed() {
                            self.lock()
                                .prefs
                                .pet
                                .set_option(&active_id, opt.key, on);
                        }
                    });
                });
            }
        });
    }

    fn draw_hud_page(&self, ui: &mut egui::Ui) {
        let (nav, intro, plugins, items, empty, author_l, enabled_suffix, homepage_l) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsNavHud).to_string(),
                s.prefs.t(MessageKey::HudSettingsIntro).to_string(),
                s.plugins.clone(),
                s.hud_items.clone(),
                s.prefs.t(MessageKey::HudSettingsEmpty).to_string(),
                s.prefs.t(MessageKey::MetaAuthor).to_string(),
                s.prefs.t(MessageKey::HudItemsEnabled).to_string(),
                s.prefs.t(MessageKey::MetaHomepage).to_string(),
            )
        };
        page_header(ui, &nav, &intro);
        ui.add_space(16.0);

        if items.is_empty() {
            empty_hint(ui, &empty);
            return;
        }

        let ctx = ui.ctx().clone();
        {
            let mut s = self.lock();
            for plugin in &plugins {
                let _ = ensure_bytes_texture(
                    &ctx,
                    &mut s.preview_textures,
                    &plugin_icon_key(plugin.id),
                    plugin.icon_png,
                );
            }
            for (pid, c) in &items {
                let _ = ensure_bytes_texture(
                    &ctx,
                    &mut s.preview_textures,
                    &hud_item_icon_key(pid, c.id),
                    c.icon_png,
                );
            }
        }
        let textures = self.lock().preview_textures.clone();

        let mut any_plugin = false;
        for plugin in &plugins {
            let contribs: Vec<_> = items
                .iter()
                .filter(|(pid, _)| *pid == plugin.id)
                .map(|(_, c)| c.clone())
                .collect();
            if contribs.is_empty() {
                continue;
            }
            any_plugin = true;

            let (mut plugin_on, open_default) = {
                let s = self.lock();
                (s.prefs.hud.is_plugin_enabled(plugin.id), true)
            };
            let enabled_n = {
                let s = self.lock();
                contribs
                    .iter()
                    .filter(|c| s.prefs.hud.is_enabled(plugin.id, c.id, c.default_enabled))
                    .count()
            };

            ui.add_space(4.0);
            section_card(ui, |ui| {
                let open_id = ui.make_persistent_id(("hud_plugin_open", plugin.id));
                let mut open = ui.data_mut(|d| *d.get_temp_mut_or(open_id, open_default));
                let plugin_icon = textures.get(&plugin_icon_key(plugin.id));

                ui.horizontal(|ui| {
                    let toggle_reserve = 50.0;
                    let left_w = (ui.available_width() - toggle_reserve).max(120.0);
                    let title = format!("{} ｜ {}", plugin.display_name, plugin.description);
                    let meta = format!(
                        "{}  ·  {} {}  ·  {}/{} {}",
                        plugin.id,
                        author_l,
                        plugin.author,
                        enabled_n,
                        contribs.len(),
                        enabled_suffix
                    );
                    let title_color = if plugin_on {
                        tone::TEXT
                    } else {
                        tone::MUTED
                    };
                    let mut left = plugin_header_hit(
                        ui,
                        left_w,
                        open,
                        plugin.display_name,
                        plugin_icon,
                        &title,
                        &meta,
                        title_color,
                    );
                    left = attach_pack_tooltip(
                        left,
                        plugin.display_name,
                        plugin.description,
                        plugin.id,
                        &author_l,
                        plugin.author,
                        None,
                        plugin.homepage,
                        &homepage_l,
                    );
                    if left.clicked() {
                        open = !open;
                    }

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if toggle_switch(ui, &mut plugin_on).changed() {
                            self.lock()
                                .prefs
                                .hud
                                .set_plugin_enabled(plugin.id, plugin_on);
                        }
                    });
                });

                ui.data_mut(|d| *d.get_temp_mut_or(open_id, open_default) = open);

                if open {
                    ui.add_space(8.0);
                    ui.add_enabled_ui(plugin_on, |ui| {
                        for (i, c) in contribs.iter().enumerate() {
                            if i > 0 {
                                ui.add_space(4.0);
                                // 分隔线从图标列起，强化「挂在插件下」的层级
                                let full = ui.available_width();
                                let (sep_rect, _) = ui.allocate_exact_size(
                                    Vec2::new(full, 1.0),
                                    Sense::hover(),
                                );
                                let line = egui::Rect::from_min_max(
                                    egui::pos2(
                                        sep_rect.left() + plugin_layout::icon_left(),
                                        sep_rect.center().y,
                                    ),
                                    egui::pos2(sep_rect.right(), sep_rect.center().y + 1.0),
                                );
                                ui.painter().rect_filled(line, 0.0, tone::LINE);
                                ui.add_space(4.0);
                            }
                            let mut on = self.lock().prefs.hud.is_enabled(
                                plugin.id,
                                c.id,
                                c.default_enabled,
                            );
                            let item_id = format!("{}.{}", plugin.id, c.id);
                            let item_icon =
                                textures.get(&hud_item_icon_key(plugin.id, c.id));
                            ui.horizontal(|ui| {
                                ui.with_layout(
                                    Layout::left_to_right(egui::Align::Center),
                                    |ui| {
                                        // 与插件图标左边缘对齐
                                        ui.add_space(plugin_layout::icon_left());
                                        hud_item_icon(ui, item_icon);
                                        ui.add_space(plugin_layout::ICON_TO_TEXT);
                                        ui.vertical(|ui| {
                                            ui.label(
                                                RichText::new(c.label)
                                                    .size(13.5)
                                                    .color(tone::TEXT),
                                            );
                                            ui.label(
                                                RichText::new(&item_id)
                                                    .size(11.0)
                                                    .color(tone::MUTED),
                                            );
                                        });
                                    },
                                );
                                ui.with_layout(
                                    Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if toggle_switch(ui, &mut on).changed() {
                                            self.lock().prefs.hud.set_enabled(
                                                plugin.id,
                                                c.id,
                                                on,
                                            );
                                        }
                                    },
                                );
                            });
                        }
                    });
                    if !plugin_on {
                        ui.add_space(6.0);
                        ui.horizontal(|ui| {
                            ui.add_space(plugin_layout::icon_left());
                            ui.label(
                                RichText::new("插件已关闭，条目暂时不会显示")
                                    .size(11.5)
                                    .color(tone::MUTED),
                            );
                        });
                    }
                }
            });
            ui.add_space(8.0);
        }

        if !any_plugin {
            empty_hint(ui, &empty);
        }
    }

    fn draw_general_page(&self, ui: &mut egui::Ui) {
        let nav = self.lock().prefs.t(MessageKey::SettingsNavGeneral).to_string();
        page_header(ui, &nav, "");
        ui.add_space(16.0);

        let (locale_l, zh, en, locale, topmost_l, topmost_hint, topmost) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsLocale).to_string(),
                s.prefs.t(MessageKey::OptLocaleZh).to_string(),
                s.prefs.t(MessageKey::OptLocaleEn).to_string(),
                s.prefs.locale,
                s.prefs.t(MessageKey::SettingsTopmost).to_string(),
                s.prefs.t(MessageKey::SettingsTopmostHint).to_string(),
                s.prefs.shell.pet_topmost,
            )
        };

        let mut locale = locale;
        let mut topmost = topmost;

        section_card(ui, |ui| {
            ui.label(
                RichText::new(locale_l)
                    .size(13.5)
                    .strong()
                    .color(tone::TEXT),
            );
            ui.add_space(10.0);
            let combo_w = ui.available_width();
            style_locale_combo(ui);
            // `height` = 弹层 ScrollArea 最大高度（不是按钮高度）
            egui::ComboBox::from_id_salt("settings_locale")
                .width(combo_w)
                .height(200.0)
                // 与插件展开箭头同款描边 chevron（非默认实心三角）
                .icon(locale_combo_chevron)
                .popup_style(egui::style::StyleModifier::new(|style| {
                    style.visuals.window_fill = tone::CARD;
                    style.visuals.panel_fill = tone::CARD;
                    style.visuals.extreme_bg_color = tone::CARD;
                    style.visuals.faint_bg_color = Color32::from_rgb(248, 249, 252);
                    style.visuals.window_stroke = Stroke::new(1.0, tone::LINE);
                    style.visuals.popup_shadow = egui::Shadow {
                        offset: [0, 4],
                        blur: 14,
                        spread: 0,
                        color: Color32::from_black_alpha(28),
                    };
                    let stroke = Stroke::new(1.0, Color32::TRANSPARENT);
                    for w in [
                        &mut style.visuals.widgets.inactive,
                        &mut style.visuals.widgets.hovered,
                        &mut style.visuals.widgets.active,
                        &mut style.visuals.widgets.open,
                    ] {
                        w.weak_bg_fill = Color32::TRANSPARENT;
                        w.bg_fill = Color32::TRANSPARENT;
                        w.bg_stroke = stroke;
                    }
                    style.visuals.selection.bg_fill = tone::ACCENT_SOFT;
                    style.visuals.selection.stroke = Stroke::new(1.0, tone::SELECTED_RING);
                }))
                .selected_text(
                    RichText::new(match locale {
                        Locale::ZhCn => zh.as_str(),
                        Locale::En => en.as_str(),
                    })
                    .size(13.5)
                    .color(tone::TEXT),
                )
                .show_ui(ui, |ui| {
                    ui.set_min_width((combo_w - 8.0).max(120.0));
                    ui.spacing_mut().item_spacing.y = 0.0;
                    ui.add_space(8.0);
                    locale_combo_option(ui, &mut locale, Locale::ZhCn, &zh);
                    ui.add_space(4.0);
                    locale_combo_option(ui, &mut locale, Locale::En, &en);
                    ui.add_space(8.0);
                });
        });

        ui.add_space(12.0);

        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&topmost_l)
                            .size(13.5)
                            .strong()
                            .color(tone::TEXT),
                    );
                    ui.label(RichText::new(&topmost_hint).size(12.0).color(tone::MUTED));
                });
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    toggle_switch(ui, &mut topmost);
                });
            });
        });

        let mut s = self.lock();
        if s.prefs.locale != locale {
            s.prefs.locale = locale;
            s.locale_dirty = true;
        }
        s.prefs.shell.pet_topmost = topmost;
    }
}

fn ensure_preview_texture(
    ctx: &egui::Context,
    cache: &mut HashMap<String, TextureHandle>,
    pet: &PetKindInfo,
) -> Option<TextureHandle> {
    ensure_bytes_texture(
        ctx,
        cache,
        &pet_preview_key(pet.id),
        pet.preview_png,
    )
}

fn ensure_bytes_texture(
    ctx: &egui::Context,
    cache: &mut HashMap<String, TextureHandle>,
    key: &str,
    bytes: Option<&[u8]>,
) -> Option<TextureHandle> {
    if let Some(tex) = cache.get(key) {
        return Some(tex.clone());
    }
    let bytes = bytes?;
    let image = image::load_from_memory(bytes).ok()?.into_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, image.as_raw());
    let tex = ctx.load_texture(key.to_string(), color, TextureOptions::LINEAR);
    cache.insert(key.to_string(), tex.clone());
    Some(tex)
}

fn pet_preview_key(pet_id: &str) -> String {
    format!("pet_preview_{pet_id}")
}

fn plugin_icon_key(plugin_id: &str) -> String {
    format!("icon:plugin:{plugin_id}")
}

fn hud_item_icon_key(plugin_id: &str, contrib_id: &str) -> String {
    format!("icon:hud:{plugin_id}.{contrib_id}")
}

fn page_header(ui: &mut egui::Ui, title: &str, intro: &str) {
    ui.label(
        RichText::new(title)
            .size(22.0)
            .strong()
            .color(tone::TEXT),
    );
    if !intro.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(intro).size(13.0).color(tone::MUTED));
    }
}

fn empty_hint(ui: &mut egui::Ui, text: &str) {
    section_card(ui, |ui| {
        ui.label(RichText::new(text).color(tone::MUTED));
    });
}

/// 设置窗独立不透明视觉：避免继承宠窗全局透明 `window_fill` 导致 Combo 弹出层透底。
fn opaque_settings_visuals(ui: &mut egui::Ui) {
    let v = ui.visuals_mut();
    v.window_fill = tone::CARD;
    v.panel_fill = tone::BG;
    v.extreme_bg_color = tone::CARD;
    v.window_stroke = Stroke::new(1.0, tone::LINE);
    v.popup_shadow = egui::Shadow {
        offset: [0, 3],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(36),
    };
    // Tooltip / 弹出层也走不透明卡底
    v.widgets.noninteractive.bg_fill = tone::CARD;
    v.widgets.inactive.bg_fill = tone::CARD;
}

/// 语言下拉：闭合态加大内边距；弹层白底细边框。
fn style_locale_combo(ui: &mut egui::Ui) {
    // 闭合按钮文字内边距（ComboBox 读 spacing.button_padding）
    ui.spacing_mut().button_padding = egui::vec2(14.0, 11.0);
    ui.spacing_mut().interact_size.y = 40.0;

    let stroke = Stroke::new(1.0, tone::LINE);
    let soft = Color32::from_rgb(252, 253, 255);
    let w = &mut ui.visuals_mut().widgets;
    for state in [
        &mut w.inactive,
        &mut w.hovered,
        &mut w.active,
        &mut w.open,
    ] {
        state.bg_fill = soft;
        state.weak_bg_fill = soft;
        state.bg_stroke = stroke;
        state.corner_radius = CornerRadius::same(8);
        state.expansion = 0.0;
    }
    w.hovered.bg_fill = tone::HOVER;
    w.hovered.weak_bg_fill = tone::HOVER;
    w.hovered.bg_stroke = Stroke::new(1.0, Color32::from_rgb(200, 208, 222));
    w.open.bg_fill = tone::CARD;
    w.open.weak_bg_fill = tone::CARD;
    w.open.bg_stroke = Stroke::new(1.0, tone::SELECTED_RING);
    w.active.bg_fill = tone::ACCENT_SOFT;
    w.active.weak_bg_fill = tone::ACCENT_SOFT;
}

/// 下拉选项：固定行高 + 左右内边距，避免文字贴边显得塌。
fn locale_combo_option(ui: &mut egui::Ui, locale: &mut Locale, value: Locale, label: &str) {
    let selected = *locale == value;
    let height = 40.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let fill = if selected {
        tone::ACCENT_SOFT
    } else if response.hovered() {
        tone::HOVER
    } else {
        Color32::TRANSPARENT
    };
    if fill.a() > 0 {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), fill);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
        if selected {
            tone::ACCENT
        } else {
            tone::TEXT
        },
    );
    if response.clicked() {
        *locale = value;
    }
}

fn section_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE
        .fill(tone::CARD)
        .stroke(Stroke::new(1.0, tone::LINE))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
}

/// iOS / Material 风格开关（替代 checkbox）。
fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired = Vec2::new(42.0, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(response.id, *on);
    let track_fill = Color32::from_rgb(
        egui::lerp(tone::STAGE.r() as f32..=tone::ACCENT.r() as f32, t) as u8,
        egui::lerp(tone::STAGE.g() as f32..=tone::ACCENT.g() as f32, t) as u8,
        egui::lerp(tone::STAGE.b() as f32..=tone::ACCENT.b() as f32, t) as u8,
    );
    let radius = rect.height() * 0.5;
    ui.painter().rect(
        rect,
        CornerRadius::same(radius as u8),
        track_fill,
        Stroke::NONE,
        egui::StrokeKind::Inside,
    );
    let knob_r = radius - 3.0;
    let knob_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), knob_r.max(4.0), Color32::WHITE);
    if response.hovered() {
        ui.painter().circle_stroke(
            egui::pos2(knob_x, rect.center().y),
            knob_r.max(4.0),
            Stroke::new(1.0, Color32::from_black_alpha(20)),
        );
    }
    response
}

/// 插件头可点区域：箭头 + 图标 + 限宽截断文案（无 Label，避免选中光标抢走点击）。
fn plugin_header_hit(
    ui: &mut egui::Ui,
    width: f32,
    open: bool,
    badge_name: &str,
    icon: Option<&TextureHandle>,
    title: &str,
    meta: &str,
    title_color: Color32,
) -> egui::Response {
    let height = 40.0;
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let response = response.on_hover_cursor(CursorIcon::PointingHand);

    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), tone::HOVER);
    }

    let chev = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - 16.0),
        Vec2::new(plugin_layout::CHEV_W, 32.0),
    );
    paint_expand_chevron(ui, response.id.with("chev"), chev, open, response.hovered());

    let badge = egui::Rect::from_min_size(
        egui::pos2(rect.left() + plugin_layout::icon_left(), rect.center().y - plugin_layout::ICON * 0.5),
        Vec2::splat(plugin_layout::ICON),
    );
    paint_plugin_icon(ui, badge, icon, badge_name);

    let text_left = badge.right() + plugin_layout::ICON_TO_TEXT;
    let text_right = rect.right() - 4.0;
    let max_w = (text_right - text_left).max(40.0);
    let title_font = FontId::proportional(14.0);
    let meta_font = FontId::proportional(11.5);
    let title_draw = truncate_ui_text(ui, title, title_font.clone(), max_w);
    let meta_draw = truncate_ui_text(ui, meta, meta_font.clone(), max_w);

    let cy = rect.center().y;
    ui.painter().text(
        egui::pos2(text_left, cy - 8.0),
        Align2::LEFT_CENTER,
        title_draw,
        title_font,
        title_color,
    );
    ui.painter().text(
        egui::pos2(text_left, cy + 9.0),
        Align2::LEFT_CENTER,
        meta_draw,
        meta_font,
        tone::MUTED,
    );

    response
}

fn paint_expand_chevron(
    ui: &mut egui::Ui,
    anim_id: egui::Id,
    rect: egui::Rect,
    open: bool,
    hovered: bool,
) {
    // 闭合指向右，展开旋转 90° 向下（插件列表）
    paint_stroke_chevron(
        ui,
        anim_id,
        rect,
        open,
        hovered,
        0.0,
        std::f32::consts::FRAC_PI_2,
    );
}

/// 语言下拉：与插件同款描边 chevron；闭合向下、展开向上。
fn locale_combo_chevron(
    ui: &egui::Ui,
    rect: egui::Rect,
    _visuals: &egui::style::WidgetVisuals,
    open: bool,
) {
    let hovered = ui.rect_contains_pointer(rect.expand(8.0));
    paint_stroke_chevron(
        ui,
        ui.id().with("locale_chev"),
        rect,
        open,
        hovered,
        std::f32::consts::FRAC_PI_2,
        std::f32::consts::PI,
    );
}

/// 描边 V 形箭头；`base_angle` 为闭合角，`open` 时再加 `open_delta`。
fn paint_stroke_chevron(
    ui: &egui::Ui,
    anim_id: egui::Id,
    rect: egui::Rect,
    open: bool,
    hovered: bool,
    base_angle: f32,
    open_delta: f32,
) {
    let t = ui.ctx().animate_bool_responsive(anim_id, open);
    let c = rect.center();
    let angle = base_angle + t * open_delta;
    let arm = 4.2;
    let stroke = Stroke::new(1.6, if hovered { tone::TEXT } else { tone::MUTED });
    let rot = |dx: f32, dy: f32| {
        let (s, co) = angle.sin_cos();
        egui::pos2(c.x + dx * co - dy * s, c.y + dx * s + dy * co)
    };
    let p1 = rot(-1.5, -arm);
    let p2 = rot(arm * 0.55, 0.0);
    let p3 = rot(-1.5, arm);
    let painter = ui.painter();
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
}

fn paint_plugin_icon(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    icon: Option<&TextureHandle>,
    fallback_name: &str,
) {
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), tone::ACCENT_SOFT);
    if let Some(tex) = icon {
        let pad = 3.0;
        let inner = rect.shrink(pad);
        ui.painter().image(
            tex.id(),
            inner,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    // 默认：首字徽章
    ui.painter().rect_stroke(
        rect,
        CornerRadius::same(10),
        Stroke::new(1.0, Color32::from_rgb(200, 214, 240)),
        egui::StrokeKind::Inside,
    );
    let ch = fallback_name
        .chars()
        .next()
        .map(|c| c.to_string())
        .unwrap_or_else(|| "P".into());
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        ch,
        FontId::proportional(14.0),
        tone::ACCENT,
    );
}

/// HUD 条目图标：包内图优先，否则程序默认图标（尺寸与插件图标对齐）。
fn hud_item_icon(ui: &mut egui::Ui, icon: Option<&TextureHandle>) {
    let size = Vec2::splat(plugin_layout::ICON);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), tone::STAGE);
    if let Some(tex) = icon {
        let inner = rect.shrink(3.0);
        ui.painter().image(
            tex.id(),
            inner,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            Color32::WHITE,
        );
        return;
    }
    let c = rect.center();
    let stroke = Stroke::new(1.5, tone::MUTED);
    ui.painter().rect_stroke(
        egui::Rect::from_center_size(c, Vec2::new(14.0, 10.0)),
        CornerRadius::same(2),
        stroke,
        egui::StrokeKind::Outside,
    );
    ui.painter().line_segment(
        [c + Vec2::new(-5.0, -1.5), c + Vec2::new(5.0, -1.5)],
        stroke,
    );
    ui.painter().line_segment(
        [c + Vec2::new(-5.0, 2.0), c + Vec2::new(2.5, 2.0)],
        stroke,
    );
}

/// 包信息悬浮卡（不透明底，避免继承宠窗透明样式）。
fn attach_pack_tooltip(
    response: egui::Response,
    name: &str,
    description: &str,
    id: &str,
    author_label: &str,
    author: &str,
    extra: Option<&str>,
    homepage: Option<&str>,
    homepage_label: &str,
) -> egui::Response {
    let name = name.to_string();
    let description = description.to_string();
    let id = id.to_string();
    let author_line = format!("{author_label}  {author}");
    let extra = extra.map(str::to_string);
    let homepage = homepage.map(str::to_string);
    let homepage_label = homepage_label.to_string();
    response.on_hover_ui(|ui| {
        opaque_tooltip_visuals(ui);
        ui.set_max_width(300.0);
        Frame::NONE
            .fill(tone::CARD)
            .stroke(Stroke::new(1.0, tone::LINE))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(
                    RichText::new(&name)
                        .size(14.0)
                        .strong()
                        .color(tone::TEXT),
                );
                if !description.is_empty() {
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(&description)
                            .size(12.5)
                            .color(tone::MUTED),
                    );
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new(&id).size(12.0).color(tone::TEXT));
                ui.label(
                    RichText::new(&author_line)
                        .size(11.5)
                        .color(tone::MUTED),
                );
                if let Some(ex) = &extra {
                    ui.label(RichText::new(ex).size(11.5).color(tone::MUTED));
                }
                if let Some(url) = &homepage {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&homepage_label)
                                .size(11.0)
                                .color(tone::MUTED),
                        );
                        ui.hyperlink_to(RichText::new(url).size(11.0), url);
                    });
                }
            });
    })
}

fn opaque_tooltip_visuals(ui: &mut egui::Ui) {
    let v = ui.visuals_mut();
    v.window_fill = tone::CARD;
    v.panel_fill = tone::CARD;
    v.extreme_bg_color = tone::CARD;
    v.override_text_color = Some(tone::TEXT);
}

fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
    let bg = if selected {
        tone::CARD
    } else if response.hovered() {
        tone::HOVER
    } else {
        Color32::TRANSPARENT
    };
    if bg.a() > 0 {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(8), bg);
    }
    if selected {
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.top() + 8.0),
            Vec2::new(3.0, rect.height() - 16.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), tone::ACCENT);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        if selected { tone::ACCENT } else { tone::TEXT },
    );
    response
}

fn resolve_card_layout(
    s: &mut SettingsState,
    avail_w: f32,
) -> (usize, f32, f32, f32) {
    if s.card_layout.is_none() {
        let layout = pet_card_layout(avail_w);
        s.card_layout = Some(layout);
        s.card_layout_for_w = avail_w;
        s.card_observe_w = avail_w;
        s.card_observe_since = None;
        s.card_settle_repaint_armed = false;
        return layout;
    }

    if (avail_w - s.card_observe_w).abs() > CARD_LAYOUT_DEADBAND {
        // 仍在改尺寸：只记观察值，不立刻改布局
        s.card_observe_w = avail_w;
        s.card_observe_since = Some(Instant::now());
        s.card_settle_repaint_armed = false;
    } else if let Some(since) = s.card_observe_since {
        if since.elapsed() >= CARD_LAYOUT_SETTLE
            && (s.card_observe_w - s.card_layout_for_w).abs() > CARD_LAYOUT_DEADBAND
        {
            let layout = pet_card_layout(s.card_observe_w);
            s.card_layout = Some(layout);
            s.card_layout_for_w = s.card_observe_w;
            s.card_observe_since = None;
            s.card_settle_repaint_armed = false;
            return layout;
        }
    }

    s.card_layout
        .unwrap_or_else(|| pet_card_layout(avail_w))
}

fn pet_card_layout(available_w: f32) -> (usize, f32, f32, f32) {
    let avail = available_w.max(CARD_MIN_W);
    // 取仍 ≥ MIN 的最大列数，均分宽度以贴齐左右
    let mut cols = 1usize;
    for c in 2usize..=5 {
        let w = (avail - CARD_GAP * (c - 1) as f32) / c as f32;
        if w < CARD_MIN_W {
            break;
        }
        cols = c;
    }
    let raw_w = (avail - CARD_GAP * (cols - 1) as f32) / cols as f32;
    let card_w = if cols == 1 {
        raw_w.min(CARD_MAX_W).max(CARD_MIN_W)
    } else {
        raw_w.max(CARD_MIN_W)
    };
    // 预览区 1:1：边长 = 卡片内宽 - 左右 padding（扣 2px 描边收缩）
    let preview_side = (card_w - 2.0 - CARD_PAD * 2.0).max(96.0);
    let card_h = 2.0 + CARD_PAD + preview_side + CARD_PAD + CARD_TEXT_H + CARD_PAD;
    (cols, card_w, card_h, preview_side)
}

/// 网格 / 列表纯图标分段按钮；点选返回新模式。
fn view_mode_icon_group(ui: &mut egui::Ui, mode: PetPickerMode) -> Option<PetPickerMode> {
    const H: f32 = 30.0;
    const CELL: f32 = 34.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(CELL * 2.0, H), Sense::hover());
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        Color32::from_rgb(232, 234, 239),
        Stroke::new(1.0, tone::LINE),
        egui::StrokeKind::Inside,
    );

    let left = egui::Rect::from_min_size(rect.min, Vec2::new(CELL, H));
    let right = egui::Rect::from_min_size(
        egui::pos2(rect.left() + CELL, rect.top()),
        Vec2::new(CELL, H),
    );

    let grid_sel = mode == PetPickerMode::Grid;
    let list_sel = mode == PetPickerMode::List;

    let grid_r = ui.interact(left, ui.id().with("pet_view_grid"), Sense::click());
    let list_r = ui.interact(right, ui.id().with("pet_view_list"), Sense::click());

    if grid_sel {
        ui.painter().rect(
            left.shrink(1.5),
            CornerRadius {
                nw: 7,
                ne: 0,
                sw: 7,
                se: 0,
            },
            tone::CARD,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    } else if grid_r.hovered() {
        ui.painter().rect_filled(
            left.shrink(1.5),
            CornerRadius {
                nw: 7,
                ne: 0,
                sw: 7,
                se: 0,
            },
            Color32::from_rgb(242, 244, 248),
        );
    }
    if list_sel {
        ui.painter().rect(
            right.shrink(1.5),
            CornerRadius {
                nw: 0,
                ne: 7,
                sw: 0,
                se: 7,
            },
            tone::CARD,
            Stroke::NONE,
            egui::StrokeKind::Inside,
        );
    } else if list_r.hovered() {
        ui.painter().rect_filled(
            right.shrink(1.5),
            CornerRadius {
                nw: 0,
                ne: 7,
                sw: 0,
                se: 7,
            },
            Color32::from_rgb(242, 244, 248),
        );
    }

    ui.painter().line_segment(
        [
            egui::pos2(rect.center().x, rect.top() + 6.0),
            egui::pos2(rect.center().x, rect.bottom() - 6.0),
        ],
        Stroke::new(1.0, Color32::from_rgb(210, 214, 222)),
    );

    draw_grid_icon(
        ui.painter(),
        left.center(),
        if grid_sel {
            tone::ACCENT
        } else {
            tone::MUTED
        },
    );
    draw_list_icon(
        ui.painter(),
        right.center(),
        if list_sel {
            tone::ACCENT
        } else {
            tone::MUTED
        },
    );

    if grid_r.clicked() && !grid_sel {
        Some(PetPickerMode::Grid)
    } else if list_r.clicked() && !list_sel {
        Some(PetPickerMode::List)
    } else {
        None
    }
}

fn draw_grid_icon(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let s = 3.2;
    let g = 2.2;
    let origin = center - Vec2::new(s + g * 0.5, s + g * 0.5);
    for row in 0..2 {
        for col in 0..2 {
            let p = origin + Vec2::new(col as f32 * (s + g), row as f32 * (s + g));
            painter.rect_filled(
                egui::Rect::from_min_size(p, Vec2::splat(s)),
                CornerRadius::same(1),
                color,
            );
        }
    }
}

fn draw_list_icon(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let w = 11.0;
    let left = center.x - w * 0.5;
    for i in 0..3 {
        let y = center.y - 5.0 + i as f32 * 5.0;
        painter.rect_filled(
            egui::Rect::from_min_size(egui::pos2(left, y - 0.9), Vec2::new(w, 1.8)),
            CornerRadius::same(1),
            color,
        );
    }
}

fn paint_preview_cover(ui: &mut egui::Ui, stage: egui::Rect, tex: &TextureHandle) {
    let clip = stage.intersect(ui.clip_rect());
    if !clip.is_positive() {
        return;
    }
    let size = tex.size_vec2();
    if size.x <= 0.0 || size.y <= 0.0 {
        return;
    }
    // 短边对齐外框（cover）：等比放大填满正方形，再居中，超出裁切
    let scale = (stage.width() / size.x).max(stage.height() / size.y);
    let img = size * scale;
    let img_rect = egui::Rect::from_center_size(stage.center(), img);
    ui.painter().with_clip_rect(clip).image(
        tex.id(),
        img_rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
}

fn footer_primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let size = Vec2::new(88.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        tone::ACCENT_PRESS
    } else if response.hovered() {
        tone::ACCENT_HOVER
    } else {
        tone::ACCENT
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        Color32::WHITE,
    );
    response
}

fn footer_secondary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let size = Vec2::new(88.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        Color32::from_rgb(220, 223, 230)
    } else if response.hovered() {
        Color32::from_rgb(242, 244, 248)
    } else {
        Color32::from_rgb(236, 238, 242)
    };
    ui.painter()
        .rect_filled(rect, CornerRadius::same(8), fill);
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        tone::TEXT,
    );
    response
}

fn pet_list_row(
    ui: &mut egui::Ui,
    name: &str,
    description: &str,
    id: &str,
    author_label: &str,
    author: &str,
    author_prefix: &str,
    size_label: &str,
    selected_badge: &str,
    selected: bool,
    preview: Option<&TextureHandle>,
    homepage: Option<&str>,
    homepage_label: &str,
) -> egui::Response {
    // 四行信息块高度；预览框边长与之相等 → 正方形且与信息等高
    const TEXT_LINES: f32 = 4.0;
    const LINE_H: f32 = 18.0;
    let text_block_h = LINE_H * TEXT_LINES;
    let thumb_side = text_block_h;
    let height = CARD_PAD * 2.0 + thumb_side;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let response = attach_pack_tooltip(
        response,
        name,
        description,
        id,
        author_prefix,
        author,
        Some(size_label),
        homepage,
        homepage_label,
    );
    let bg = if selected {
        tone::ACCENT_SOFT
    } else if response.hovered() {
        Color32::from_rgb(252, 253, 255)
    } else {
        tone::CARD
    };
    let stroke = if selected {
        Stroke::new(1.5, tone::SELECTED_RING)
    } else {
        Stroke::new(1.0, tone::LINE)
    };
    let draw = rect.shrink(0.5);
    ui.painter().rect(
        draw,
        CornerRadius::same(10),
        bg,
        stroke,
        egui::StrokeKind::Inside,
    );

    let thumb = egui::Rect::from_min_size(
        egui::pos2(draw.left() + CARD_PAD, draw.top() + CARD_PAD),
        Vec2::splat(thumb_side),
    );
    ui.painter()
        .rect_filled(thumb, CornerRadius::same(8), tone::STAGE);
    if let Some(tex) = preview {
        paint_preview_cover(ui, thumb, tex);
    }

    let text_left = thumb.right() + CARD_PAD;
    let text_right = draw.right() - CARD_PAD;
    let text_top = draw.top() + CARD_PAD;
    let name_max = if selected {
        let badge_w = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                selected_badge.to_string(),
                FontId::proportional(12.0),
                Color32::WHITE,
            )
            .size()
            .x
        }) + 8.0;
        (text_right - text_left - badge_w).max(40.0)
    } else {
        (text_right - text_left).max(40.0)
    };
    let name_draw = truncate_ui_text(ui, name, FontId::proportional(15.0), name_max);
    let desc_draw = truncate_ui_text(
        ui,
        description,
        FontId::proportional(12.0),
        (text_right - text_left).max(40.0),
    );
    let author_draw = truncate_ui_text(
        ui,
        author_label,
        FontId::proportional(11.5),
        (text_right - text_left).max(40.0),
    );
    let size_draw = truncate_ui_text(
        ui,
        size_label,
        FontId::proportional(11.0),
        (text_right - text_left).max(40.0),
    );

    ui.painter().text(
        egui::pos2(text_left, text_top),
        Align2::LEFT_TOP,
        name_draw,
        FontId::proportional(15.0),
        tone::TEXT,
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + LINE_H),
        Align2::LEFT_TOP,
        desc_draw,
        FontId::proportional(12.0),
        tone::MUTED,
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + LINE_H * 2.0),
        Align2::LEFT_TOP,
        author_draw,
        FontId::proportional(11.5),
        tone::MUTED,
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + LINE_H * 3.0),
        Align2::LEFT_TOP,
        size_draw,
        FontId::proportional(11.0),
        tone::MUTED,
    );
    if selected {
        // 相对整行内容区（与预览框同高）垂直居中，而非贴在名称行顶
        ui.painter().text(
            egui::pos2(text_right, thumb.center().y),
            Align2::RIGHT_CENTER,
            selected_badge,
            FontId::proportional(12.0),
            tone::ACCENT,
        );
    }
    response
}

fn pet_preview_card(
    ui: &mut egui::Ui,
    card_w: f32,
    card_h: f32,
    preview_side: f32,
    name: &str,
    description: &str,
    id: &str,
    author_label: &str,
    author: &str,
    author_prefix: &str,
    size_label: &str,
    selected_badge: &str,
    selected: bool,
    preview: Option<&TextureHandle>,
    homepage: Option<&str>,
    homepage_label: &str,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(card_w, card_h), Sense::click());
    let response = attach_pack_tooltip(
        response,
        name,
        description,
        id,
        author_prefix,
        author,
        Some(size_label),
        homepage,
        homepage_label,
    );
    let draw = rect.shrink(1.0);

    let bg = if selected {
        tone::ACCENT_SOFT
    } else if response.hovered() {
        Color32::from_rgb(252, 253, 255)
    } else {
        tone::CARD
    };
    let stroke = if selected {
        Stroke::new(1.5, tone::SELECTED_RING)
    } else if response.hovered() {
        Stroke::new(1.2, Color32::from_rgb(190, 198, 214))
    } else {
        Stroke::new(1.0, tone::LINE)
    };
    ui.painter().rect(
        draw,
        CornerRadius::same(12),
        bg,
        stroke,
        egui::StrokeKind::Inside,
    );

    let side = preview_side
        .min(draw.width() - CARD_PAD * 2.0)
        .max(1.0);
    let stage = egui::Rect::from_center_size(
        egui::pos2(draw.center().x, draw.top() + CARD_PAD + side * 0.5),
        Vec2::splat(side),
    );
    ui.painter()
        .rect_filled(stage, CornerRadius::same(10), tone::STAGE);

    if let Some(tex) = preview {
        paint_preview_cover(ui, stage, tex);
    } else {
        ui.painter().text(
            stage.center(),
            Align2::CENTER_CENTER,
            "—",
            FontId::proportional(28.0),
            tone::MUTED,
        );
    }

    let text_left = draw.left() + CARD_PAD;
    let text_right = draw.right() - CARD_PAD;
    let text_top = stage.bottom() + CARD_PAD;
    let text_bottom = draw.bottom() - CARD_PAD;
    let name_draw = truncate_ui_text(
        ui,
        name,
        FontId::proportional(14.5),
        (text_right - text_left).max(40.0),
    );
    let desc_draw = truncate_ui_text(
        ui,
        description,
        FontId::proportional(12.0),
        (text_right - text_left).max(40.0),
    );
    let author_draw = truncate_ui_text(
        ui,
        author_label,
        FontId::proportional(11.5),
        (text_right - text_left).max(40.0),
    );
    let size_max = if selected {
        let badge_w = ui.fonts_mut(|f| {
            f.layout_no_wrap(
                selected_badge.to_string(),
                FontId::proportional(11.5),
                Color32::WHITE,
            )
            .size()
            .x
        }) + 8.0;
        (text_right - text_left - badge_w).max(40.0)
    } else {
        (text_right - text_left).max(40.0)
    };
    let size_draw = truncate_ui_text(ui, size_label, FontId::proportional(11.5), size_max);

    ui.painter().text(
        egui::pos2(text_left, text_top),
        Align2::LEFT_TOP,
        name_draw,
        FontId::proportional(14.5),
        tone::TEXT,
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + 20.0),
        Align2::LEFT_TOP,
        desc_draw,
        FontId::proportional(12.0),
        tone::MUTED,
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + 38.0),
        Align2::LEFT_TOP,
        author_draw,
        FontId::proportional(11.5),
        tone::MUTED,
    );
    ui.painter().text(
        egui::pos2(text_left, text_bottom),
        Align2::LEFT_BOTTOM,
        size_draw,
        FontId::proportional(11.5),
        tone::MUTED,
    );
    if selected {
        ui.painter().text(
            egui::pos2(text_right, text_bottom),
            Align2::RIGHT_BOTTOM,
            selected_badge,
            FontId::proportional(11.5),
            tone::ACCENT,
        );
    }
    response
}

fn truncate_ui_text(ui: &egui::Ui, text: &str, font: FontId, max_w: f32) -> String {
    if text.is_empty() || max_w <= 8.0 {
        return String::new();
    }
    let full = ui.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font.clone(), Color32::WHITE)
            .size()
            .x
    });
    if full <= max_w {
        return text.to_string();
    }
    // 按宽度比例估算截断点，再最多微调一次，避免每帧二分测字
    let chars: Vec<char> = text.chars().collect();
    let mut keep = ((chars.len() as f32) * (max_w / full) * 0.92)
        .floor()
        .max(1.0) as usize;
    keep = keep.min(chars.len());
    let ell = '…';
    let mut cand: String = chars[..keep].iter().collect();
    cand.push(ell);
    let w = ui.fonts_mut(|f| {
        f.layout_no_wrap(cand.clone(), font.clone(), Color32::WHITE)
            .size()
            .x
    });
    if w > max_w && keep > 1 {
        keep = (keep * 4 / 5).max(1);
        cand = chars[..keep].iter().collect();
        cand.push(ell);
    }
    cand
}
