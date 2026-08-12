//! 统一设置窗：侧栏（常规 / 宠物 / 插件）+ 右侧内容。

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use deskhud_engine::{HudContribution, PetConfigOption, PetKindInfo, PluginInfo};
use deskhud_ui::{CatalogStore, Locale, MessageKey, PetPickerMode, UiPreferences, UiTheme};
use egui::text::{CCursor, CCursorRange};
use egui::text_edit::TextEditState;
use egui::{
    self, Align2, Area, Color32, CornerRadius, CursorIcon, FontId, Frame, Layout, Margin, Order,
    RichText, Sense, Stroke, TextureHandle, TextureOptions, Vec2,
};

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
/// 常规页下拉统一宽度（按钮截断，避免长文案撑宽）。
const SETTINGS_COMBO_W: f32 = 200.0;
/// 下拉弹层最大高度（超出滚动）。
const SETTINGS_COMBO_POPUP_H: f32 = 240.0;

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

/// 设置窗色板：随当前 egui 深/浅模式切换（由草稿主题驱动）。
mod tone {
    use std::cell::Cell;

    use egui::{Color32, Context, Theme};

    thread_local! {
        static DARK: Cell<bool> = const { Cell::new(false) };
    }

    pub fn sync(ctx: &Context) {
        DARK.with(|c| c.set(matches!(ctx.theme(), Theme::Dark)));
    }

    fn dark() -> bool {
        DARK.with(|c| c.get())
    }

    pub fn bg() -> Color32 {
        if dark() {
            Color32::from_rgb(30, 31, 34)
        } else {
            Color32::from_rgb(246, 247, 249)
        }
    }
    pub fn side() -> Color32 {
        if dark() {
            Color32::from_rgb(37, 38, 43)
        } else {
            Color32::from_rgb(236, 238, 243)
        }
    }
    pub fn card() -> Color32 {
        if dark() {
            Color32::from_rgb(43, 45, 49)
        } else {
            Color32::from_rgb(255, 255, 255)
        }
    }
    pub fn text() -> Color32 {
        if dark() {
            Color32::from_rgb(232, 234, 237)
        } else {
            Color32::from_rgb(28, 30, 36)
        }
    }
    pub fn muted() -> Color32 {
        if dark() {
            Color32::from_rgb(154, 160, 166)
        } else {
            Color32::from_rgb(110, 114, 124)
        }
    }
    pub fn line() -> Color32 {
        if dark() {
            Color32::from_rgb(60, 64, 72)
        } else {
            Color32::from_rgb(222, 225, 232)
        }
    }
    pub fn accent() -> Color32 {
        Color32::from_rgb(47, 110, 220)
    }
    pub fn accent_hover() -> Color32 {
        Color32::from_rgb(70, 132, 236)
    }
    pub fn accent_press() -> Color32 {
        Color32::from_rgb(36, 90, 190)
    }
    pub fn accent_soft() -> Color32 {
        if dark() {
            Color32::from_rgb(40, 56, 88)
        } else {
            Color32::from_rgb(232, 240, 255)
        }
    }
    pub fn stage() -> Color32 {
        if dark() {
            Color32::from_rgb(70, 74, 82)
        } else {
            Color32::from_rgb(228, 232, 240)
        }
    }
    pub fn selected_ring() -> Color32 {
        Color32::from_rgb(70, 120, 220)
    }
    pub fn hover() -> Color32 {
        if dark() {
            Color32::from_rgb(52, 54, 60)
        } else {
            Color32::from_rgb(242, 244, 248)
        }
    }
    pub fn faint() -> Color32 {
        if dark() {
            Color32::from_rgb(48, 50, 56)
        } else {
            Color32::from_rgb(248, 249, 252)
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    Pet,
    Hud,
    General,
    About,
}

const APP_HOMEPAGE: &str = "https://github.com/ko-eika/deskhud";
/// 与根 `Cargo.toml` 中 `egui` 版本对齐；升级依赖时请同步。
const APP_EGUI_VERSION: &str = "0.36";

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
    /// 外壳 + 包文案（打开设置时从主壳拷贝；草稿语言切换即时生效）。
    pub catalogs: CatalogStore,
    pub locale_dirty: bool,
    /// 打开后需要 Focus 一次。
    /// 打开后需要下发一次尺寸 / 位置。
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
    /// 请求开始 HUD 布局编辑。
    pub hud_layout_begin: bool,
    /// 请求完成并写回草稿布局。
    /// 请求取消布局编辑。
    pub hud_layout_cancel: bool,
    /// 当前是否在布局编辑（UI 显示用）。
    pub hud_layout_editing: bool,
}

impl SettingsHost {
    pub(crate) fn begin_hud_layout_edit(&self) {
        let mut state = self.lock();
        state.hud_layout_begin = true;
    }
    /// Draw the existing settings surface inside a directly hosted egui root
    /// window. Window creation/visibility is owned by the native host.
    pub(crate) fn draw_native(&self, ui: &mut egui::Ui) {
        self.draw(ui);
    }

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
                catalogs: CatalogStore::new(),
                locale_dirty: false,
                apply_requested: false,
                pending_flush: false,
                discard_draft: false,
                card_layout: None,
                card_layout_for_w: 0.0,
                card_observe_w: 0.0,
                card_observe_since: None,
                card_settle_repaint_armed: false,
                preview_textures: HashMap::new(),
                hud_layout_begin: false,
                hud_layout_cancel: false,
                hud_layout_editing: false,
            })),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, SettingsState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn open(
        &self,
        prefs: &UiPreferences,
        pets: Vec<PetKindInfo>,
        pet_options: HashMap<String, Vec<PetConfigOption>>,
        plugins: Vec<PluginInfo>,
        hud_items: Vec<(&'static str, HudContribution)>,
        catalogs: CatalogStore,
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
        s.catalogs = catalogs;
        s.tab = tab;
        // 会话内窗口层级跟打开瞬间的已应用值；草稿置顶只影响「应用」后
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

    fn draw(&self, ui: &mut egui::Ui) {
        {
            let mut state = self.lock();
            if state.hud_layout_begin {
                state.hud_layout_begin = false;
                state.hud_layout_editing = true;
            }
        }
        // 草稿主题即时预览（取消时主壳会恢复已应用偏好）
        let draft_theme = self.lock().prefs.shell.ui_theme;
        crate::theme::apply(ui.ctx(), draft_theme);
        tone::sync(ui.ctx());

        // clear_color 对子窗也是透明；必须铺满不透明底，否则发黑且点击异常
        let fill = tone::bg();
        ui.painter()
            .rect_filled(ui.max_rect(), CornerRadius::ZERO, fill);

        // 持续记录窗口几何，避免关闭事件发生在最后一帧之后而丢失尺寸。
        self.capture_geometry(ui);

        // 主壳为透明宠窗把全局 window_fill 设成透明；设置窗内控件/弹出层需不透明
        opaque_settings_visuals(ui);

        let mut close = false;
        let mut discard_on_close = true;
        let ctx = ui.ctx().clone();

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            let editing = self.lock().hud_layout_editing;
            if editing {
                self.lock().hud_layout_cancel = true;
                self.lock().hud_layout_editing = false;
                ctx.request_repaint_of(egui::ViewportId::ROOT);
            } else {
                close = true;
                discard_on_close = true;
            }
        }

        egui::CentralPanel::default()
            .frame(Frame::NONE.fill(fill).inner_margin(0.0))
            .show(ui, |ui| {
                egui::Panel::left("deskhud_settings_nav")
                    .exact_size(SIDE_W)
                    .resizable(false)
                    .frame(
                        Frame::NONE
                            .fill(tone::side())
                            .inner_margin(Margin::symmetric(12, 16)),
                    )
                    .show(ui, |ui| {
                        self.draw_sidebar(ui);
                    });

                egui::Panel::bottom("deskhud_settings_footer")
                    .exact_size(56.0)
                    .resizable(false)
                    .frame(
                        Frame::NONE
                            .fill(fill)
                            .stroke(Stroke::new(1.0, tone::line()))
                            .inner_margin(Margin::symmetric(20, 12)),
                    )
                    .show(ui, |ui| {
                        self.draw_footer(ui, &mut close, &mut discard_on_close);
                    });

                egui::CentralPanel::default()
                    .frame(
                        Frame::NONE
                            .fill(fill)
                            .inner_margin(Margin::symmetric(24, 18)),
                    )
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
            // 窗口关闭 / Esc / 取消 → 丢弃草稿；应用 → 保留已提交内容
            let discard = if user_close && !close {
                true
            } else {
                discard_on_close
            };
            self.close_viewport(discard);
        }
    }

    fn close_viewport(&self, discard: bool) {
        let mut s = self.lock();
        s.open = false;
        s.discard_draft = discard;
        s.pending_flush = true;
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
            s.pending_flush = true;
        }
    }

    fn draw_sidebar(&self, ui: &mut egui::Ui) {
        let (title, tab) = {
            let s = self.lock();
            (s.prefs.t(MessageKey::SettingsTitle).to_string(), s.tab)
        };

        ui.label(RichText::new(title).size(18.0).strong().color(tone::text()));
        ui.add_space(4.0);
        ui.label(RichText::new("DeskHud").size(11.5).color(tone::muted()));
        ui.add_space(18.0);

        for (next, key) in [
            (SettingsTab::General, MessageKey::SettingsNavGeneral),
            (SettingsTab::Pet, MessageKey::SettingsNavPet),
            (SettingsTab::Hud, MessageKey::SettingsNavHud),
            (SettingsTab::About, MessageKey::SettingsNavAbout),
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

    fn draw_footer(&self, ui: &mut egui::Ui, close: &mut bool, discard_on_close: &mut bool) {
        let (reset_l, apply_l, cancel_l, dirty) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::ActionReset).to_string(),
                s.prefs.t(MessageKey::ActionApply).to_string(),
                s.prefs.t(MessageKey::ActionCancel).to_string(),
                settings_draft_dirty(&s.prefs, &s.baseline),
            )
        };
        // right_to_left：先画的在最右 → 视觉从左到右为 重置 / 应用 / 取消
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            if footer_secondary_button(ui, &cancel_l).clicked() {
                *close = true;
                *discard_on_close = true;
            }
            ui.add_space(8.0);
            ui.add_enabled_ui(dirty, |ui| {
                if footer_primary_button(ui, &apply_l).clicked() {
                    self.capture_geometry(ui);
                    let mut s = self.lock();
                    s.baseline = s.prefs.clone();
                    s.apply_requested = true;
                }
            });
            ui.add_space(8.0);
            ui.add_enabled_ui(dirty, |ui| {
                if footer_secondary_button(ui, &reset_l).clicked() {
                    let mut s = self.lock();
                    let mode = s.prefs.pet.picker_mode;
                    let geo = (
                        s.prefs.shell.settings_width,
                        s.prefs.shell.settings_height,
                        s.prefs.shell.settings_pos_x,
                        s.prefs.shell.settings_pos_y,
                    );
                    s.prefs = s.baseline.clone();
                    s.prefs.pet.picker_mode = mode;
                    s.prefs.shell.settings_width = geo.0;
                    s.prefs.shell.settings_height = geo.1;
                    s.prefs.shell.settings_pos_x = geo.2;
                    s.prefs.shell.settings_pos_y = geo.3;
                    s.card_layout = None;
                }
            });
        });
    }

    fn draw_content(&self, ui: &mut egui::Ui) {
        let tab = self.lock().tab;
        match tab {
            SettingsTab::General => self.draw_general_page(ui),
            SettingsTab::Pet => self.draw_pet_page(ui),
            SettingsTab::Hud => self.draw_hud_page(ui),
            SettingsTab::About => self.draw_about_page(ui),
        }
    }

    fn draw_about_page(&self, ui: &mut egui::Ui) {
        let (nav, intro, version_l, license_l, author_l, homepage_l, stack) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsNavAbout).to_string(),
                s.prefs.t(MessageKey::SettingsAboutIntro).to_string(),
                s.prefs.t(MessageKey::SettingsAboutVersion).to_string(),
                s.prefs.t(MessageKey::SettingsAboutLicense).to_string(),
                s.prefs.t(MessageKey::MetaAuthor).to_string(),
                s.prefs.t(MessageKey::MetaHomepage).to_string(),
                s.prefs.t(MessageKey::SettingsAboutStack).to_string(),
            )
        };
        page_header(ui, &nav, &intro);
        ui.add_space(16.0);

        // 版本 / 作者 / 许可证：编译期从本包 Cargo 元数据注入（继承 workspace.package）
        let version = env!("CARGO_PKG_VERSION");
        let author = env!("CARGO_PKG_AUTHORS");
        let license = env!("CARGO_PKG_LICENSE");

        section_card(ui, |ui| {
            ui.label(
                RichText::new("DeskHud")
                    .size(20.0)
                    .strong()
                    .color(tone::text()),
            );
            ui.add_space(6.0);
            ui.label(RichText::new(&stack).size(13.0).color(tone::muted()));
            ui.add_space(14.0);
            about_info_row(ui, &version_l, version);
            ui.add_space(8.0);
            about_info_row(ui, &author_l, author);
            ui.add_space(8.0);
            about_info_row(ui, &license_l, license);
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.set_min_height(22.0);
                ui.allocate_ui_with_layout(
                    Vec2::new(72.0, 22.0),
                    Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.label(RichText::new(&homepage_l).size(13.0).color(tone::muted()));
                    },
                );
                ui.hyperlink_to(
                    RichText::new(APP_HOMEPAGE).size(13.0).color(tone::accent()),
                    APP_HOMEPAGE,
                );
            });
        });

        ui.add_space(12.0);
        section_card(ui, |ui| {
            about_info_row(ui, "Rust", concat!(env!("CARGO_PKG_RUST_VERSION"), "+"));
            ui.add_space(8.0);
            about_info_row(ui, "egui / egui_glow", APP_EGUI_VERSION);
        });
    }

    fn draw_pet_page(&self, ui: &mut egui::Ui) {
        let (
            nav,
            intro,
            pets,
            active,
            size_key,
            selected_badge,
            mode,
            author_l,
            homepage_l,
            catalogs,
            locale,
        ) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsNavPet).to_string(),
                s.prefs.t(MessageKey::SettingsPetIntro).to_string(),
                s.pets.clone(),
                s.prefs.pet.kind.clone(),
                s.prefs.t(MessageKey::SettingsPetWindowSize).to_string(),
                s.prefs.t(MessageKey::SettingsPetSelected).to_string(),
                s.prefs.pet.picker_mode,
                s.prefs.t(MessageKey::MetaAuthor).to_string(),
                s.prefs.t(MessageKey::MetaHomepage).to_string(),
                s.catalogs.clone(),
                s.prefs.locale,
            )
        };

        // 标题 + 视图切换同一行；说明多行换行（含第三方风险提示，勿截断）
        ui.horizontal(|ui| {
            ui.set_height(30.0);
            ui.label(RichText::new(&nav).size(22.0).strong().color(tone::text()));
            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                if let Some(m) = view_mode_icon_group(ui, mode) {
                    let mut s = self.lock();
                    s.prefs.pet.picker_mode = m;
                    s.card_layout = None;
                }
            });
        });
        ui.add_space(6.0);
        ui.label(RichText::new(&intro).size(13.0).color(tone::muted()));
        ui.add_space(14.0);

        let ctx = ui.ctx().clone();
        let preview_edge = crate::image_decode::physical_raster_edge(
            crate::image_decode::PREVIEW_RASTER_EDGE,
            ctx.pixels_per_point(),
        );
        {
            let mut s = self.lock();
            for pet in &pets {
                let _ = ensure_preview_texture(&ctx, &mut s.preview_textures, pet);
            }
        }
        let textures = self.lock().preview_textures.clone();
        let mode = self.lock().prefs.pet.picker_mode;

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
                        ui.ctx()
                            .request_repaint_after(CARD_LAYOUT_SETTLE + Duration::from_millis(16));
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
                            let name = pack_field(
                                &catalogs,
                                locale,
                                pet.id,
                                "display_name",
                                pet.display_name,
                            );
                            let desc = pack_field(
                                &catalogs,
                                locale,
                                pet.id,
                                "description",
                                pet.description,
                            );
                            let size_label = format!(
                                "{}  {:.0}×{:.0}",
                                size_key, pet.window_width, pet.window_height
                            );
                            let author_label = format!("{}  {}", author_l, pet.author);
                            let tex = textures.get(&pet_preview_key(pet.id, preview_edge));
                            let resp = pet_preview_card(
                                ui,
                                card_w,
                                card_h,
                                preview_side,
                                &name,
                                &desc,
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
                    let name =
                        pack_field(&catalogs, locale, pet.id, "display_name", pet.display_name);
                    let desc =
                        pack_field(&catalogs, locale, pet.id, "description", pet.description);
                    let size_label = format!(
                        "{}  {:.0}×{:.0}",
                        size_key, pet.window_width, pet.window_height
                    );
                    let author_label = format!("{}  {}", author_l, pet.author);
                    let tex = textures.get(&pet_preview_key(pet.id, preview_edge));
                    let resp = pet_list_row(
                        ui,
                        &name,
                        &desc,
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
                s.prefs.pet.kind = id;
                s.prefs.pet.apply_window_size(w, h);
            }
        }

        ui.add_space(16.0);
        self.draw_active_pet_options(ui);
    }

    fn draw_active_pet_options(&self, ui: &mut egui::Ui) {
        let (active_id, options, options_title, catalogs, locale) = {
            let s = self.lock();
            let id = s.prefs.pet.kind.clone();
            let opts = s.pet_options.get(&id).cloned().unwrap_or_default();
            let title = s.prefs.t(MessageKey::SettingsPetOptions).to_string();
            (id, opts, title, s.catalogs.clone(), s.prefs.locale)
        };
        if options.is_empty() {
            return;
        }

        ui.label(
            RichText::new(&options_title)
                .size(16.0)
                .strong()
                .color(tone::text()),
        );
        ui.add_space(10.0);
        section_card(ui, |ui| {
            for (i, opt) in options.iter().enumerate() {
                if i > 0 {
                    ui.add_space(10.0);
                    ui.separator();
                    ui.add_space(10.0);
                }
                let label = pack_field(
                    &catalogs,
                    locale,
                    &active_id,
                    &format!("{}.label", opt.key),
                    opt.label,
                );
                let description = pack_field(
                    &catalogs,
                    locale,
                    &active_id,
                    &format!("{}.description", opt.key),
                    opt.description,
                );
                let mut on = self
                    .lock()
                    .prefs
                    .pet
                    .get_option(&active_id, opt.key, opt.default);
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&label)
                                .size(13.5)
                                .strong()
                                .color(tone::text()),
                        );
                        ui.label(RichText::new(&description).size(12.0).color(tone::muted()));
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if toggle_switch(ui, &mut on).changed() {
                            self.lock().prefs.pet.set_option(&active_id, opt.key, on);
                        }
                    });
                });
            }
        });
    }

    fn draw_hud_page(&self, ui: &mut egui::Ui) {
        let (
            nav,
            intro,
            plugins,
            items,
            empty,
            author_l,
            enabled_suffix,
            homepage_l,
            disabled_hint,
            master_l,
            master_on_hint,
            master_off_hint,
            catalogs,
            locale,
        ) = {
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
                s.prefs.t(MessageKey::HudPluginDisabledHint).to_string(),
                s.prefs.t(MessageKey::HudMasterEnable).to_string(),
                s.prefs.t(MessageKey::HudMasterEnableHint).to_string(),
                s.prefs.t(MessageKey::HudMasterDisabledHint).to_string(),
                s.catalogs.clone(),
                s.prefs.locale,
            )
        };
        {
            let (editing, edit_l, editing_hint, mut master_on) = {
                let s = self.lock();
                (
                    s.hud_layout_editing,
                    s.prefs.t(MessageKey::HudLayoutEdit).to_string(),
                    s.prefs.t(MessageKey::HudLayoutEditingHint).to_string(),
                    s.prefs.hud.is_master_enabled(),
                )
            };
            ui.label(RichText::new(&nav).size(22.0).strong().color(tone::text()));
            if !intro.is_empty() {
                ui.add_space(6.0);
                ui.label(RichText::new(&intro).size(13.0).color(tone::muted()));
            }
            ui.add_space(12.0);
            section_card(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.vertical(|ui| {
                        ui.label(
                            RichText::new(&master_l)
                                .size(14.0)
                                .strong()
                                .color(tone::text()),
                        );
                        ui.label(
                            RichText::new(if master_on {
                                master_on_hint.as_str()
                            } else {
                                master_off_hint.as_str()
                            })
                            .size(12.0)
                            .color(tone::muted()),
                        );
                    });
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if toggle_switch(ui, &mut master_on).changed() {
                            self.lock().prefs.hud.set_master_enabled(master_on);
                        }
                    });
                });
            });
            ui.add_space(10.0);
            if editing {
                ui.label(RichText::new(editing_hint).size(12.5).color(tone::muted()));
            } else if !items.is_empty() {
                ui.add_enabled_ui(master_on, |ui| {
                    if hud_layout_action_button(ui, &edit_l).clicked() {
                        self.lock().hud_layout_begin = true;
                        ui.ctx().request_repaint_of(egui::ViewportId::ROOT);
                    }
                });
            }
            ui.add_space(14.0);

            if items.is_empty() {
                empty_hint(ui, &empty);
                return;
            }

            let ctx = ui.ctx().clone();
            let icon_edge = crate::image_decode::physical_raster_edge(
                crate::image_decode::ICON_RASTER_EDGE,
                ctx.pixels_per_point(),
            );
            {
                let mut s = self.lock();
                for plugin in &plugins {
                    let _ = ensure_bytes_texture(
                        &ctx,
                        &mut s.preview_textures,
                        &plugin_icon_key(plugin.id, icon_edge),
                        plugin.icon,
                        icon_edge,
                    );
                }
                for (pid, c) in &items {
                    let _ = ensure_bytes_texture(
                        &ctx,
                        &mut s.preview_textures,
                        &hud_item_icon_key(pid, c.id, icon_edge),
                        c.icon,
                        icon_edge,
                    );
                }
            }
            let textures = self.lock().preview_textures.clone();

            // 总开关关闭时：布局入口与各插件调整一并禁用
            ui.add_enabled_ui(master_on, |ui| {
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
                        let plugin_icon = textures.get(&plugin_icon_key(plugin.id, icon_edge));

                        let plugin_name = pack_field(
                            &catalogs,
                            locale,
                            plugin.id,
                            "display_name",
                            plugin.display_name,
                        );
                        let plugin_desc = pack_field(
                            &catalogs,
                            locale,
                            plugin.id,
                            "description",
                            plugin.description,
                        );
                        ui.horizontal(|ui| {
                            let toggle_reserve = 52.0;
                            let left_w = (ui.available_width() - toggle_reserve).max(120.0);
                            let title = format!("{} ｜ {}", plugin_name, plugin_desc);
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
                                tone::text()
                            } else {
                                tone::muted()
                            };
                            let mut left = plugin_header_hit(
                                ui,
                                left_w,
                                open,
                                &plugin_name,
                                plugin_icon,
                                &title,
                                &meta,
                                title_color,
                            );
                            left = attach_pack_tooltip(
                                left,
                                &plugin_name,
                                &plugin_desc,
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
                                        ui.painter().rect_filled(line, 0.0, tone::line());
                                        ui.add_space(4.0);
                                    }
                                    let mut on = self.lock().prefs.hud.is_enabled(
                                        plugin.id,
                                        c.id,
                                        c.default_enabled,
                                    );
                                    let item_id = format!("{}.{}", plugin.id, c.id);
                                    let item_label = pack_field(
                                        &catalogs,
                                        locale,
                                        plugin.id,
                                        &format!("{}.label", c.id),
                                        c.label,
                                    );
                                    let item_icon = textures
                                        .get(&hud_item_icon_key(plugin.id, c.id, icon_edge));
                                    ui.horizontal(|ui| {
                                        ui.with_layout(
                                            Layout::left_to_right(egui::Align::Center),
                                            |ui| {
                                                ui.add_space(plugin_layout::icon_left());
                                                hud_item_icon(ui, item_icon);
                                                ui.add_space(plugin_layout::ICON_TO_TEXT);
                                                ui.vertical(|ui| {
                                                    ui.label(
                                                        RichText::new(&item_label)
                                                            .size(13.5)
                                                            .color(tone::text()),
                                                    );
                                                    ui.label(
                                                        RichText::new(&item_id)
                                                            .size(11.0)
                                                            .color(tone::muted()),
                                                    );
                                                });
                                            },
                                        );
                                        ui.with_layout(
                                            Layout::right_to_left(egui::Align::Center),
                                            |ui| {
                                                if toggle_switch(ui, &mut on).changed() {
                                                    self.lock()
                                                        .prefs
                                                        .hud
                                                        .set_enabled(plugin.id, c.id, on);
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
                                        RichText::new(&disabled_hint)
                                            .size(11.5)
                                            .color(tone::muted()),
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
            });
        }
    }

    fn draw_general_page(&self, ui: &mut egui::Ui) {
        let nav = self
            .lock()
            .prefs
            .t(MessageKey::SettingsNavGeneral)
            .to_string();
        page_header(ui, &nav, "");
        ui.add_space(16.0);

        let (topmost_l, topmost_hint, mut topmost) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsTopmost).to_string(),
                s.prefs.t(MessageKey::SettingsTopmostHint).to_string(),
                s.prefs.shell.topmost,
            )
        };
        section_card(ui, |ui| {
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        RichText::new(&topmost_l)
                            .size(13.5)
                            .strong()
                            .color(tone::text()),
                    );
                    ui.label(RichText::new(&topmost_hint).size(12.0).color(tone::muted()));
                });
                ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                    toggle_switch(ui, &mut topmost);
                });
            });
        });
        self.lock().prefs.shell.topmost = topmost;
        ui.add_space(16.0);

        let (
            locale_l,
            zh,
            en,
            locale,
            theme_l,
            theme_light,
            theme_dark,
            theme_system,
            theme,
            font_l,
            font_family_l,
            font_style_l,
            font_size_l,
            font_preview,
            font_id,
            font_family,
            font_style,
            font_size,
            locale_enum,
        ) = {
            let s = self.lock();
            (
                s.prefs.t(MessageKey::SettingsLocale).to_string(),
                s.prefs.t(MessageKey::OptLocaleZh).to_string(),
                s.prefs.t(MessageKey::OptLocaleEn).to_string(),
                s.prefs.locale,
                s.prefs.t(MessageKey::SettingsTheme).to_string(),
                s.prefs.t(MessageKey::OptThemeLight).to_string(),
                s.prefs.t(MessageKey::OptThemeDark).to_string(),
                s.prefs.t(MessageKey::OptThemeSystem).to_string(),
                s.prefs.shell.ui_theme,
                s.prefs.t(MessageKey::SettingsUiFont).to_string(),
                s.prefs.t(MessageKey::SettingsUiFontFamily).to_string(),
                s.prefs.t(MessageKey::SettingsUiFontStyle).to_string(),
                s.prefs.t(MessageKey::SettingsUiFontSize).to_string(),
                s.prefs.t(MessageKey::SettingsUiFontPreview).to_string(),
                s.prefs.shell.ui_font_id.clone(),
                s.prefs.shell.ui_font_family.clone(),
                crate::fonts::normalize_style_name(&s.prefs.shell.ui_font_style),
                s.prefs.shell.ui_font_size,
                s.prefs.locale,
            )
        };

        let mut locale = locale;
        let mut theme = theme;
        let families = crate::fonts::list_font_families();
        let mut family_key = if families.iter().any(|f| f.family_key == font_family) {
            font_family
        } else {
            crate::fonts::family_key_for_font_id(&families, &font_id)
        };
        let mut font_style = font_style;
        let mut font_size = font_size;
        if let Some(fam) = families.iter().find(|f| f.family_key == family_key) {
            let styles = fam.style_names();
            if !styles
                .iter()
                .any(|s| crate::fonts::normalize_style_name(s) == font_style)
            {
                font_style = styles.first().cloned().unwrap_or_else(|| "Regular".into());
            }
        }
        let family_label = crate::fonts::label_for_family(&families, &family_key);
        let style_disp = match locale_enum {
            Locale::ZhCn => crate::fonts::style_label_zh(&font_style),
            Locale::En => crate::fonts::style_label_en(&font_style),
        };

        section_card(ui, |ui| {
            setting_row(ui, &theme_l, |ui| {
                let theme_text = match theme {
                    UiTheme::Light => theme_light.as_str(),
                    UiTheme::Dark => theme_dark.as_str(),
                    UiTheme::System => theme_system.as_str(),
                };
                settings_combo(
                    ui,
                    "settings_theme",
                    SETTINGS_COMBO_W,
                    theme_text,
                    match theme {
                        UiTheme::System => 0,
                        UiTheme::Light => 1,
                        UiTheme::Dark => 2,
                    },
                    3,
                    |ui| {
                        theme_combo_option(ui, &mut theme, UiTheme::System, &theme_system);
                        ui.add_space(2.0);
                        theme_combo_option(ui, &mut theme, UiTheme::Light, &theme_light);
                        ui.add_space(2.0);
                        theme_combo_option(ui, &mut theme, UiTheme::Dark, &theme_dark);
                    },
                );
            });
        });

        ui.add_space(12.0);

        section_card(ui, |ui| {
            setting_row(ui, &locale_l, |ui| {
                let locale_text = match locale {
                    Locale::ZhCn => zh.as_str(),
                    Locale::En => en.as_str(),
                };
                settings_combo(
                    ui,
                    "settings_locale",
                    SETTINGS_COMBO_W,
                    locale_text,
                    match locale {
                        Locale::ZhCn => 0,
                        Locale::En => 1,
                    },
                    2,
                    |ui| {
                        locale_combo_option(ui, &mut locale, Locale::ZhCn, &zh);
                        ui.add_space(2.0);
                        locale_combo_option(ui, &mut locale, Locale::En, &en);
                    },
                );
            });
        });

        ui.add_space(12.0);

        section_card(ui, |ui| {
            ui.label(
                RichText::new(&font_l)
                    .size(13.5)
                    .strong()
                    .color(tone::text()),
            );
            ui.add_space(8.0);
            // 不缩进：标题下横线，圈出配置项区域
            settings_full_rule(ui);
            ui.add_space(8.0);

            nested_settings_block(ui, |ui| {
                setting_row_divided(ui, &font_family_l, true, |ui| {
                    let opts: Vec<(String, String)> = families
                        .iter()
                        .map(|f| (f.family_key.clone(), f.label.clone()))
                        .collect();
                    searchable_combo(
                        ui,
                        "settings_ui_font_family",
                        SETTINGS_COMBO_W,
                        &family_label,
                        &opts,
                        |key, label, q| {
                            if q.is_empty() {
                                return false;
                            }
                            let q = q.to_lowercase();
                            if label.to_lowercase().starts_with(&q) {
                                return true;
                            }
                            families
                                .iter()
                                .find(|f| f.family_key == key)
                                .is_some_and(|f| {
                                    f.search_terms
                                        .iter()
                                        .any(|term| term.to_lowercase().starts_with(&q))
                                })
                        },
                        &mut family_key,
                    );
                });

                let style_names: Vec<String> = families
                    .iter()
                    .find(|f| f.family_key == family_key)
                    .map(|f| f.style_names())
                    .unwrap_or_else(|| vec!["Regular".into()]);
                setting_row_divided(ui, &font_style_l, true, |ui| {
                    let style_idx = style_names
                        .iter()
                        .position(|st| st == &font_style)
                        .unwrap_or(0);
                    settings_combo(
                        ui,
                        "settings_ui_font_style",
                        SETTINGS_COMBO_W,
                        &style_disp,
                        style_idx,
                        style_names.len(),
                        |ui| {
                            for (i, st) in style_names.iter().enumerate() {
                                if i > 0 {
                                    ui.add_space(2.0);
                                }
                                let label = match locale_enum {
                                    Locale::ZhCn => crate::fonts::style_label_zh(st),
                                    Locale::En => crate::fonts::style_label_en(st),
                                };
                                string_combo_option(ui, &mut font_style, st, &label);
                            }
                        },
                    );
                });

                // 最后一项底部不再画行内横线
                setting_row_divided(ui, &font_size_l, false, |ui| {
                    size_searchable_combo(
                        ui,
                        "settings_ui_font_size",
                        SETTINGS_COMBO_W,
                        &mut font_size,
                    );
                });
            });

            ui.add_space(8.0);
            // 不缩进：配置区与预览之间的横线
            settings_full_rule(ui);
            ui.add_space(10.0);
            ui.label(
                RichText::new(&font_preview)
                    .size(font_size.clamp(11.0, 18.0))
                    .color(tone::text()),
            );
        });

        if let Some(fam) = families.iter().find(|f| f.family_key == family_key) {
            let styles = fam.style_names();
            if !styles
                .iter()
                .any(|s| crate::fonts::normalize_style_name(s) == font_style)
            {
                font_style = styles.first().cloned().unwrap_or_else(|| "Regular".into());
            }
        }
        let resolved_id = crate::fonts::resolve_font_id(&families, &family_key, &font_style);

        let mut s = self.lock();
        if s.prefs.locale != locale {
            s.prefs.locale = locale;
            s.locale_dirty = true;
        }
        s.prefs.shell.ui_theme = theme;
        let font_changed = s.prefs.shell.ui_font_id != resolved_id
            || s.prefs.shell.ui_font_family != family_key
            || crate::fonts::normalize_style_name(&s.prefs.shell.ui_font_style) != font_style
            || (s.prefs.shell.ui_font_size - font_size).abs() > 0.01;
        s.prefs.shell.ui_font_id = resolved_id.clone();
        s.prefs.shell.ui_font_family = family_key;
        s.prefs.shell.ui_font_style = font_style;
        s.prefs.shell.ui_font_size = font_size;
        if font_changed {
            crate::fonts::configure_typography(ui.ctx(), &resolved_id, font_size);
        }
    }
}

fn setting_row(ui: &mut egui::Ui, label: &str, add: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.set_height(40.0);
        ui.allocate_ui_with_layout(
            Vec2::new(72.0, 40.0),
            Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(13.0).color(tone::text()));
            },
        );
        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            add(ui);
        });
    });
}

/// 组合配置子区：左右缩进，形成层次。
fn nested_settings_block(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE
        .inner_margin(Margin::symmetric(14, 0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add(ui);
        });
}

/// 与卡片同宽的分隔线（不随子项缩进）。
fn settings_full_rule(ui: &mut egui::Ui) {
    let stroke = Stroke::new(1.0, tone::line());
    let y = ui.cursor().min.y;
    let x0 = ui.max_rect().left();
    let x1 = ui.max_rect().right();
    ui.painter().hline(x0..=x1, y, stroke);
    ui.add_space(1.0);
}

/// 子配置行；`divider_below` 时在下方画分隔线。
fn setting_row_divided(
    ui: &mut egui::Ui,
    label: &str,
    divider_below: bool,
    add: impl FnOnce(&mut egui::Ui),
) {
    setting_row(ui, label, add);
    if divider_below {
        ui.add_space(4.0);
        let stroke = Stroke::new(1.0, tone::line());
        let y = ui.cursor().min.y;
        let x0 = ui.max_rect().left();
        let x1 = ui.max_rect().right();
        ui.painter().hline(x0..=x1, y, stroke);
        ui.add_space(9.0);
    }
}

fn theme_combo_option(ui: &mut egui::Ui, selected: &mut UiTheme, value: UiTheme, label: &str) {
    let mut cur = format!("{selected:?}");
    font_combo_option(ui, &mut cur, &format!("{value:?}"), label);
    if cur == format!("{value:?}") {
        *selected = value;
    }
}

fn string_combo_option(ui: &mut egui::Ui, selected: &mut String, value: &str, label: &str) {
    let mut cur = selected.clone();
    font_combo_option(ui, &mut cur, value, label);
    if cur == value {
        *selected = value.to_string();
    }
}

/// 统一普通下拉：定宽截断 + 同款 chevron；条目少时贴合内容高度，超限才固定限高并滚动。
fn settings_combo(
    ui: &mut egui::Ui,
    id_salt: &'static str,
    width: f32,
    selected_text: impl Into<egui::WidgetText>,
    selected_index: usize,
    item_count: usize,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    style_locale_combo(ui);
    let button_id = ui.make_persistent_id(id_salt);
    let area_id = button_id.with("area");
    let open_id = button_id.with("open");
    let prev_sel_id = button_id.with("prev_sel");
    let scroll_pending_id = button_id.with("scroll_pending");
    let mut is_open = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(open_id, || false));

    // 选中项相对上一帧变了 → 关闭（选项在弹层内点选后，下一帧生效）
    let prev_sel = ui.data(|d| d.get_temp::<usize>(prev_sel_id));
    if is_open {
        if let Some(prev) = prev_sel {
            if prev != selected_index {
                is_open = false;
            }
        }
    }
    ui.data_mut(|d| d.insert_temp(prev_sel_id, selected_index));

    let height = ui.spacing().interact_size.y.max(40.0);
    let (rect, bar_resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let bar_w = bar_resp.rect.width();
    const LIST_PAD_Y: f32 = 8.0;
    let content_h = combo_list_content_height(item_count, LIST_PAD_Y);
    let popup_h = content_h.min(SETTINGS_COMBO_POPUP_H);
    let needs_scroll = content_h > SETTINGS_COMBO_POPUP_H + 0.5;

    let visuals = if is_open {
        ui.visuals().widgets.open.clone()
    } else {
        ui.style().interact(&bar_resp).clone()
    };
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );

    let pad = ui.spacing().button_padding;
    let icon_size = Vec2::splat(ui.spacing().icon_width);
    let inner = rect.shrink2(pad);
    let icon_rect = Align2::RIGHT_CENTER.align_size_within_rect(icon_size, inner);
    let text_rect = egui::Rect::from_min_max(
        inner.min,
        egui::pos2(icon_rect.left() - ui.spacing().icon_spacing, inner.max.y),
    );

    let selected_text = selected_text.into();
    let shown = truncate_to_width(ui, selected_text.text(), text_rect.width(), 13.5);
    ui.painter().text(
        egui::pos2(text_rect.left(), text_rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        FontId::proportional(13.5),
        tone::text(),
    );
    settings_combo_chevron(ui, id_salt, icon_rect, is_open);

    let mut just_opened = false;
    if !is_open && bar_resp.clicked() {
        is_open = true;
        just_opened = true;
        if needs_scroll {
            // 多帧重试：首帧 content 高度未就绪时 offset 会被夹成 0
            ui.data_mut(|d| {
                d.insert_temp(
                    scroll_pending_id,
                    (
                        combo_scroll_offset_idx(selected_index, SETTINGS_COMBO_POPUP_H, LIST_PAD_Y),
                        4u8,
                    ),
                );
            });
        }
    } else if is_open && bar_resp.clicked() {
        is_open = false;
    }

    if !is_open {
        ui.data_mut(|d| d.remove::<(f32, u8)>(scroll_pending_id));
    }

    if is_open {
        let popup_pos = egui::pos2(bar_resp.rect.left(), bar_resp.rect.bottom() + 2.0);
        let jump = if needs_scroll {
            ui.data_mut(|d| match d.get_temp::<(f32, u8)>(scroll_pending_id) {
                Some((off, left)) if left > 0 => {
                    let next = left - 1;
                    if next == 0 {
                        d.remove::<(f32, u8)>(scroll_pending_id);
                    } else {
                        d.insert_temp(scroll_pending_id, (off, next));
                    }
                    Some(off)
                }
                _ => None,
            })
        } else {
            None
        };
        let area_inner = Area::new(area_id)
            .order(Order::Foreground)
            .fixed_pos(popup_pos)
            .default_size(Vec2::new(bar_w, popup_h))
            .sense(Sense::click())
            .show(ui.ctx(), |ui| {
                settings_combo_popup_style().apply(ui.style_mut());
                Frame::NONE
                    .fill(tone::card())
                    .stroke(Stroke::new(1.0, tone::line()))
                    .corner_radius(CornerRadius::same(8))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 14,
                        spread: 0,
                        color: Color32::from_black_alpha(28),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(bar_w);
                        ui.set_max_width(bar_w);
                        ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Truncate);
                        if needs_scroll {
                            let mut scroll = egui::ScrollArea::vertical()
                                .id_salt((id_salt, "scroll"))
                                .max_height(SETTINGS_COMBO_POPUP_H)
                                .animated(false)
                                .auto_shrink([false, true])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                );
                            if let Some(off) = jump {
                                scroll = scroll.vertical_scroll_offset(off);
                            }
                            scroll.show(ui, |ui| {
                                ui.set_width(bar_w);
                                ui.set_min_width(bar_w);
                                ui.set_max_width(bar_w);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add_space(LIST_PAD_Y);
                                add_contents(ui);
                                ui.add_space(LIST_PAD_Y);
                            });
                        } else {
                            // 未超限：不用 ScrollArea，避免出现空滚动条
                            ui.set_width(bar_w);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add_space(LIST_PAD_Y);
                            add_contents(ui);
                            ui.add_space(LIST_PAD_Y);
                        }
                    });
            });

        let click_away = !just_opened
            && ui.input(|i| i.pointer.any_click())
            && bar_resp.clicked_elsewhere()
            && area_inner.response.clicked_elsewhere();
        if click_away {
            is_open = false;
        }
    }

    ui.data_mut(|d| d.insert_temp(open_id, is_open));
}

/// 下拉列表内容高度（含上下内边距；未封顶）。
fn combo_list_content_height(item_count: usize, pad_y: f32) -> f32 {
    const ROW_H: f32 = 36.0;
    const ROW_GAP: f32 = 2.0;
    if item_count == 0 {
        return pad_y * 2.0;
    }
    pad_y * 2.0 + item_count as f32 * ROW_H + item_count.saturating_sub(1) as f32 * ROW_GAP
}

/// 将第 `idx` 项滚到弹层可视区中部附近。
fn combo_scroll_offset_idx(idx: usize, list_h: f32, pad_y: f32) -> f32 {
    const ROW_H: f32 = 36.0;
    const ROW_GAP: f32 = 2.0;
    let y = pad_y + idx as f32 * (ROW_H + ROW_GAP);
    (y - (list_h - ROW_H) * 0.5).max(0.0)
}

/// 可搜索下拉（前缀补齐）：点开全选；输入替换选区；列表不过滤，跳到最佳前缀匹配并补齐且选中后缀。
fn searchable_combo(
    ui: &mut egui::Ui,
    id_salt: &str,
    width: f32,
    selected_label: &str,
    opts: &[(String, String)],
    mut is_prefix: impl FnMut(&str, &str, &str) -> bool,
    selected_key: &mut String,
) {
    let button_id = ui.make_persistent_id(id_salt);
    let area_id = button_id.with("area");
    let open_id = button_id.with("open");
    let text_id = button_id.with("text");
    let edit_id = button_id.with("edit");
    let hi_id = button_id.with("highlight");
    let typed_id = button_id.with("typed_len");
    let suppress_id = button_id.with("suppress_match");
    let ime_id = button_id.with("ime_composing");
    let scroll_pending_id = button_id.with("scroll_pending");

    let mut is_open = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(open_id, || false));
    let mut text = ui.data_mut(|d| d.get_temp_mut_or_insert_with(text_id, String::new).clone());
    let mut highlight = ui.data_mut(|d| {
        d.get_temp_mut_or_insert_with(hi_id, || selected_key.clone())
            .clone()
    });
    let mut typed_len = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(typed_id, || 0usize));
    let mut suppress_match = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(suppress_id, || false));
    let mut ime_composing = ui.data_mut(|d| *d.get_temp_mut_or_insert_with(ime_id, || false));
    // 多帧 offset 跳转；勿用 scroll_to_me（长列表首帧高度未稳时会反复居中狂滚）

    // 中文等 IME：预编辑期间禁止补齐/改选区，否则会打断候选窗
    let ime_event_this_frame = update_ime_composing(ui, &mut ime_composing);

    style_locale_combo(ui);
    let height = ui.spacing().interact_size.y.max(40.0);
    let (rect, bar_resp) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let bar_w = bar_resp.rect.width();
    const ROW_GAP: f32 = 2.0;
    const LIST_PAD_Y: f32 = 6.0;
    let content_h = combo_list_content_height(opts.len(), LIST_PAD_Y);
    let popup_h = content_h.min(SETTINGS_COMBO_POPUP_H);
    let needs_scroll = content_h > SETTINGS_COMBO_POPUP_H + 0.5;

    let visuals = if is_open {
        ui.visuals().widgets.open.clone()
    } else {
        ui.style().interact(&bar_resp).clone()
    };
    ui.painter().rect(
        rect,
        visuals.corner_radius,
        visuals.weak_bg_fill,
        visuals.bg_stroke,
        egui::StrokeKind::Inside,
    );

    let pad = ui.spacing().button_padding;
    let icon_size = Vec2::splat(ui.spacing().icon_width);
    let inner = rect.shrink2(pad);
    let icon_rect = Align2::RIGHT_CENTER.align_size_within_rect(icon_size, inner);
    let text_rect = egui::Rect::from_min_max(
        inner.min,
        egui::pos2(icon_rect.left() - ui.spacing().icon_spacing, inner.max.y),
    );

    let mut just_opened = false;

    if !is_open && bar_resp.clicked() {
        text = selected_label.to_string();
        highlight = selected_key.clone();
        typed_len = text.chars().count();
        suppress_match = false;
        is_open = true;
        just_opened = true;
        if needs_scroll {
            ui.data_mut(|d| {
                d.insert_temp(
                    scroll_pending_id,
                    (
                        combo_scroll_offset(opts, &highlight, SETTINGS_COMBO_POPUP_H),
                        4u8,
                    ),
                );
            });
        }
    } else if is_open
        && bar_resp.clicked()
        && ui.rect_contains_pointer(icon_rect.expand(6.0))
        && !ui.rect_contains_pointer(text_rect)
    {
        is_open = false;
    }

    if is_open {
        if just_opened {
            ui.ctx().data_mut(|d| d.remove::<TextEditState>(edit_id));
        }
        let mut edit_text = text.clone();
        let edit_resp = ui.put(
            text_rect,
            egui::TextEdit::singleline(&mut edit_text)
                .id(edit_id)
                .frame(Frame::NONE)
                .desired_width(text_rect.width())
                .text_color(tone::text()),
        );

        if edit_resp.changed() {
            let typed = edit_text;
            if ime_composing {
                // 预编辑串由 TextEdit/IME 管理；只同步缓冲，不动光标与补齐
                text = typed;
            } else {
                // 退格/删除：去掉补齐选区后不再自动匹配，直到再次输入字符
                let deleting = ui.input(|i| {
                    i.key_pressed(egui::Key::Backspace) || i.key_pressed(egui::Key::Delete)
                });
                if deleting {
                    suppress_match = true;
                    text = typed;
                    typed_len = text.chars().count();
                    set_text_edit_selection(ui.ctx(), edit_id, typed_len, typed_len);
                } else {
                    suppress_match = false;
                    if typed.is_empty() {
                        text = String::new();
                        typed_len = 0;
                    } else if let Some((key, label)) =
                        opts.iter().find(|(k, l)| is_prefix(k, l, &typed))
                    {
                        let label_lower = label.to_lowercase();
                        let typed_lower = typed.to_lowercase();
                        let hi_changed = *key != highlight;
                        text = label.clone();
                        highlight = key.clone();
                        if label_lower.starts_with(&typed_lower) {
                            typed_len = typed.chars().count();
                        } else {
                            // 别名命中：整词选中，继续输入将整体替换
                            typed_len = 0;
                        }
                        set_text_edit_selection(ui.ctx(), edit_id, typed_len, text.chars().count());
                        if hi_changed && needs_scroll {
                            ui.data_mut(|d| {
                                d.insert_temp(
                                    scroll_pending_id,
                                    (
                                        combo_scroll_offset(
                                            opts,
                                            &highlight,
                                            SETTINGS_COMBO_POPUP_H,
                                        ),
                                        4u8,
                                    ),
                                );
                            });
                        }
                    } else {
                        text = typed;
                        typed_len = text.chars().count();
                    }
                }
            }
        }

        if just_opened {
            edit_resp.request_focus();
            // 点开全选，下一次输入直接替换
            set_text_edit_selection(ui.ctx(), edit_id, 0, text.chars().count());
        }

        // Enter 上屏候选时勿关闭下拉
        if !ime_composing && !ime_event_this_frame && ui.input(|i| i.key_pressed(egui::Key::Enter))
        {
            *selected_key = highlight.clone();
            is_open = false;
        }
    } else {
        let shown = truncate_to_width(ui, selected_label, text_rect.width(), 13.5);
        ui.painter().text(
            egui::pos2(text_rect.left(), text_rect.center().y),
            Align2::LEFT_CENTER,
            shown,
            FontId::proportional(13.5),
            tone::text(),
        );
    }

    settings_combo_chevron(ui, id_salt, icon_rect, is_open);

    if is_open {
        let before = selected_key.clone();
        let popup_pos = egui::pos2(bar_resp.rect.left(), bar_resp.rect.bottom() + 2.0);
        let hi = highlight.clone();
        let jump = if needs_scroll {
            ui.data_mut(|d| match d.get_temp::<(f32, u8)>(scroll_pending_id) {
                Some((off, left)) if left > 0 => {
                    let next = left - 1;
                    if next == 0 {
                        d.remove::<(f32, u8)>(scroll_pending_id);
                    } else {
                        d.insert_temp(scroll_pending_id, (off, next));
                    }
                    Some(off)
                }
                _ => None,
            })
        } else {
            None
        };

        let area_inner = Area::new(area_id)
            .order(Order::Foreground)
            .fixed_pos(popup_pos)
            .default_size(Vec2::new(bar_w, popup_h))
            .sense(Sense::click())
            .show(ui.ctx(), |ui| {
                settings_combo_popup_style().apply(ui.style_mut());
                Frame::NONE
                    .fill(tone::card())
                    .stroke(Stroke::new(1.0, tone::line()))
                    .corner_radius(CornerRadius::same(8))
                    .shadow(egui::Shadow {
                        offset: [0, 4],
                        blur: 14,
                        spread: 0,
                        color: Color32::from_black_alpha(28),
                    })
                    .show(ui, |ui| {
                        ui.set_min_width(bar_w);
                        ui.set_max_width(bar_w);
                        if needs_scroll {
                            let mut scroll = egui::ScrollArea::vertical()
                                .id_salt((id_salt, "scroll"))
                                .max_height(SETTINGS_COMBO_POPUP_H)
                                .animated(false)
                                .auto_shrink([false, true])
                                .scroll_bar_visibility(
                                    egui::scroll_area::ScrollBarVisibility::VisibleWhenNeeded,
                                );
                            if let Some(off) = jump {
                                scroll = scroll.vertical_scroll_offset(off);
                            }
                            scroll.show(ui, |ui| {
                                ui.set_width(bar_w);
                                ui.spacing_mut().item_spacing.y = 0.0;
                                ui.add_space(LIST_PAD_Y);
                                for (i, (key, label)) in opts.iter().enumerate() {
                                    if i > 0 {
                                        ui.add_space(ROW_GAP);
                                    }
                                    let active = *key == hi;
                                    let resp = combo_option_row(ui, active, label);
                                    if resp.clicked() {
                                        *selected_key = key.clone();
                                    }
                                }
                                ui.add_space(LIST_PAD_Y);
                            });
                        } else {
                            ui.set_width(bar_w);
                            ui.spacing_mut().item_spacing.y = 0.0;
                            ui.add_space(LIST_PAD_Y);
                            for (i, (key, label)) in opts.iter().enumerate() {
                                if i > 0 {
                                    ui.add_space(ROW_GAP);
                                }
                                let active = *key == hi;
                                let resp = combo_option_row(ui, active, label);
                                if resp.clicked() {
                                    *selected_key = key.clone();
                                }
                            }
                            ui.add_space(LIST_PAD_Y);
                        }
                    });
            });

        let click_away = !just_opened
            && ui.input(|i| i.pointer.any_click())
            && bar_resp.clicked_elsewhere()
            && area_inner.response.clicked_elsewhere();
        if click_away {
            // 失焦时提交当前高亮项
            *selected_key = highlight.clone();
            is_open = false;
        } else if *selected_key != before {
            is_open = false;
        }
    }

    if is_open {
        ui.data_mut(|d| {
            d.insert_temp(text_id, text.clone());
            d.insert_temp(hi_id, highlight.clone());
            d.insert_temp(typed_id, typed_len);
            d.insert_temp(suppress_id, suppress_match);
            d.insert_temp(ime_id, ime_composing);
        });
    } else {
        ui.data_mut(|d| {
            d.insert_temp(text_id, String::new());
            d.insert_temp(hi_id, selected_key.clone());
            d.insert_temp(typed_id, 0usize);
            d.insert_temp(suppress_id, false);
            d.insert_temp(ime_id, false);
            d.remove::<(f32, u8)>(scroll_pending_id);
        });
    }
    ui.data_mut(|d| d.insert_temp(open_id, is_open));
}

/// 根据本帧 `ImeEvent` 更新组合态；返回是否收到任意 IME 事件。
fn update_ime_composing(ui: &egui::Ui, composing: &mut bool) -> bool {
    let mut saw = false;
    ui.input(|i| {
        for ev in &i.events {
            let egui::Event::Ime(ime) = ev else {
                continue;
            };
            saw = true;
            match ime {
                egui::ImeEvent::Preedit { text, .. } => {
                    *composing = !text.is_empty();
                }
                egui::ImeEvent::Commit(_) => {
                    *composing = false;
                }
                _ => {}
            }
        }
    });
    saw
}

/// 将高亮项滚到弹层可视区中部附近（一次性 offset，无动画）。
fn combo_scroll_offset(opts: &[(String, String)], highlight: &str, list_h: f32) -> f32 {
    const ROW_H: f32 = 36.0;
    const ROW_GAP: f32 = 2.0;
    const LIST_PAD_Y: f32 = 6.0;
    let Some(idx) = opts.iter().position(|(k, _)| k == highlight) else {
        return 0.0;
    };
    let y = LIST_PAD_Y + idx as f32 * (ROW_H + ROW_GAP);
    (y - (list_h - ROW_H) * 0.5).max(0.0)
}

fn size_searchable_combo(ui: &mut egui::Ui, id_salt: &str, width: f32, font_size: &mut f32) {
    let opts: Vec<(String, String)> = crate::fonts::FONT_SIZE_OPTIONS
        .iter()
        .map(|&sz| {
            let s = format!("{sz:.0}");
            (s.clone(), s)
        })
        .collect();
    let selected_label = format!("{:.0}", *font_size);
    let mut key = selected_label.clone();
    searchable_combo(
        ui,
        id_salt,
        width,
        &selected_label,
        &opts,
        |_k, label, q| !q.is_empty() && label.to_lowercase().starts_with(&q.to_lowercase()),
        &mut key,
    );
    if let Ok(v) = key.parse::<f32>() {
        *font_size = v.clamp(10.0, 22.0);
    }
}

fn set_text_edit_selection(ctx: &egui::Context, edit_id: egui::Id, start: usize, end: usize) {
    let mut state = TextEditState::load(ctx, edit_id).unwrap_or_default();
    state.cursor.set_char_range(Some(CCursorRange::two(
        CCursor::new(start),
        CCursor::new(end),
    )));
    state.store(ctx, edit_id);
}

fn combo_option_row(ui: &mut egui::Ui, active: bool, label: &str) -> egui::Response {
    let height = 36.0;
    let pad_x = 14.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let fill = if active {
        tone::accent_soft()
    } else if response.hovered() {
        tone::hover()
    } else {
        Color32::TRANSPARENT
    };
    if fill.a() > 0 {
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    }
    let text_max_w = (rect.width() - pad_x * 2.0).max(0.0);
    let shown = truncate_to_width(ui, label, text_max_w, 13.0);
    ui.painter().with_clip_rect(rect).text(
        egui::pos2(rect.left() + pad_x, rect.center().y),
        Align2::LEFT_CENTER,
        shown,
        FontId::proportional(13.0),
        if active { tone::accent() } else { tone::text() },
    );
    response
}

fn truncate_to_width(ui: &egui::Ui, text: &str, max_w: f32, size: f32) -> String {
    let galley =
        ui.painter()
            .layout_no_wrap(text.to_string(), FontId::proportional(size), tone::text());
    if galley.size().x <= max_w || max_w <= 0.0 {
        return text.to_string();
    }
    let mut chars: Vec<char> = text.chars().collect();
    let mut out = text.to_string();
    while chars.len() > 1 {
        chars.pop();
        let candidate: String = chars.iter().collect::<String>() + "…";
        let g = ui.painter().layout_no_wrap(
            candidate.clone(),
            FontId::proportional(size),
            tone::text(),
        );
        if g.size().x <= max_w {
            out = candidate;
            break;
        }
    }
    out
}

fn settings_combo_popup_style() -> egui::style::StyleModifier {
    egui::style::StyleModifier::new(|style| {
        style.visuals.window_fill = tone::card();
        style.visuals.panel_fill = tone::card();
        style.visuals.extreme_bg_color = tone::card();
        style.visuals.faint_bg_color = tone::faint();
        style.visuals.window_stroke = Stroke::new(1.0, tone::line());
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
        style.visuals.selection.bg_fill = tone::accent_soft();
        style.visuals.selection.stroke = Stroke::new(1.0, tone::selected_ring());
    })
}

fn font_combo_option(
    ui: &mut egui::Ui,
    selected_id: &mut String,
    value: &str,
    label: &str,
) -> egui::Response {
    let response = combo_option_row(ui, selected_id == value, label);
    if response.clicked() {
        *selected_id = value.to_string();
    }
    response
}

fn ensure_preview_texture(
    ctx: &egui::Context,
    cache: &mut HashMap<String, TextureHandle>,
    pet: &PetKindInfo,
) -> Option<TextureHandle> {
    let edge = crate::image_decode::physical_raster_edge(
        crate::image_decode::PREVIEW_RASTER_EDGE,
        ctx.pixels_per_point(),
    );
    ensure_bytes_texture(
        ctx,
        cache,
        &pet_preview_key(pet.id, edge),
        pet.preview,
        edge,
    )
}

fn ensure_bytes_texture(
    ctx: &egui::Context,
    cache: &mut HashMap<String, TextureHandle>,
    key: &str,
    bytes: Option<&[u8]>,
    max_edge: u32,
) -> Option<TextureHandle> {
    if let Some(tex) = cache.get(key) {
        return Some(tex.clone());
    }
    let bytes = bytes?;
    let color = crate::image_decode::decode_to_color_image(bytes, max_edge)?;
    let tex = ctx.load_texture(key.to_string(), color, TextureOptions::LINEAR);
    cache.insert(key.to_string(), tex.clone());
    Some(tex)
}

fn pet_preview_key(pet_id: &str, edge: u32) -> String {
    format!("pet_preview_{pet_id}@{edge}")
}

fn plugin_icon_key(plugin_id: &str, edge: u32) -> String {
    format!("icon:plugin:{plugin_id}@{edge}")
}

fn hud_item_icon_key(plugin_id: &str, contrib_id: &str, edge: u32) -> String {
    format!("icon:hud:{plugin_id}.{contrib_id}@{edge}")
}

fn page_header(ui: &mut egui::Ui, title: &str, intro: &str) {
    ui.label(RichText::new(title).size(22.0).strong().color(tone::text()));
    if !intro.is_empty() {
        ui.add_space(6.0);
        ui.label(RichText::new(intro).size(13.0).color(tone::muted()));
    }
}

fn empty_hint(ui: &mut egui::Ui, text: &str) {
    section_card(ui, |ui| {
        ui.label(RichText::new(text).color(tone::muted()));
    });
}

/// 设置窗独立不透明视觉：避免继承宠窗全局透明 `window_fill` 导致 Combo 弹出层透底。
fn opaque_settings_visuals(ui: &mut egui::Ui) {
    let dark = matches!(ui.ctx().theme(), egui::Theme::Dark);
    let v = ui.visuals_mut();
    v.dark_mode = dark;
    v.window_fill = tone::card();
    v.panel_fill = tone::bg();
    v.extreme_bg_color = tone::card();
    v.faint_bg_color = tone::faint();
    v.window_stroke = Stroke::new(1.0, tone::line());
    v.popup_shadow = egui::Shadow {
        offset: [0, 3],
        blur: 10,
        spread: 0,
        color: Color32::from_black_alpha(if dark { 80 } else { 36 }),
    };
    v.override_text_color = Some(tone::text());
    // Tooltip / 弹出层 / TextEdit 走主题色
    let stroke = Stroke::new(1.0, tone::line());
    for w in [
        &mut v.widgets.noninteractive,
        &mut v.widgets.inactive,
        &mut v.widgets.hovered,
        &mut v.widgets.active,
        &mut v.widgets.open,
    ] {
        w.bg_fill = tone::card();
        w.weak_bg_fill = tone::faint();
        w.bg_stroke = stroke;
        w.fg_stroke = Stroke::new(1.0, tone::text());
    }
    v.widgets.hovered.bg_fill = tone::hover();
    v.widgets.hovered.weak_bg_fill = tone::hover();
    v.selection.bg_fill = tone::accent_soft();
    v.selection.stroke = Stroke::new(1.0, tone::selected_ring());
}

/// 语言 / 字体下拉：闭合态加大内边距；填充随主题。
fn style_locale_combo(ui: &mut egui::Ui) {
    // 闭合按钮文字内边距（ComboBox 读 spacing.button_padding）
    ui.spacing_mut().button_padding = egui::vec2(14.0, 11.0);
    ui.spacing_mut().interact_size.y = 40.0;

    let stroke = Stroke::new(1.0, tone::line());
    let fill = tone::faint();
    let w = &mut ui.visuals_mut().widgets;
    for state in [&mut w.inactive, &mut w.hovered, &mut w.active, &mut w.open] {
        state.bg_fill = fill;
        state.weak_bg_fill = fill;
        state.bg_stroke = stroke;
        state.fg_stroke = Stroke::new(1.0, tone::text());
        state.corner_radius = CornerRadius::same(8);
        state.expansion = 0.0;
    }
    w.hovered.bg_fill = tone::hover();
    w.hovered.weak_bg_fill = tone::hover();
    w.hovered.bg_stroke = Stroke::new(1.0, tone::line());
    w.open.bg_fill = tone::card();
    w.open.weak_bg_fill = tone::card();
    w.open.bg_stroke = Stroke::new(1.0, tone::selected_ring());
    w.active.bg_fill = tone::accent_soft();
    w.active.weak_bg_fill = tone::accent_soft();
}

/// 下拉选项：固定行高 + 左右内边距，避免文字贴边显得塌。
fn locale_combo_option(ui: &mut egui::Ui, locale: &mut Locale, value: Locale, label: &str) {
    let selected = *locale == value;
    let height = 36.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), height), Sense::click());
    let fill = if selected {
        tone::accent_soft()
    } else if response.hovered() {
        tone::hover()
    } else {
        Color32::TRANSPARENT
    };
    if fill.a() > 0 {
        ui.painter().rect_filled(rect, CornerRadius::same(6), fill);
    }
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(13.5),
        if selected {
            tone::accent()
        } else {
            tone::text()
        },
    );
    if response.clicked() {
        *locale = value;
    }
}

fn section_card(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    Frame::NONE
        .fill(tone::card())
        .stroke(Stroke::new(1.0, tone::line()))
        .corner_radius(CornerRadius::same(12))
        .inner_margin(Margin::symmetric(16, 14))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
            add_contents(ui);
        });
}

fn about_info_row(ui: &mut egui::Ui, label: &str, value: &str) {
    ui.horizontal(|ui| {
        ui.set_min_height(22.0);
        ui.allocate_ui_with_layout(
            Vec2::new(72.0, 22.0),
            Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.label(RichText::new(label).size(13.0).color(tone::muted()));
            },
        );
        ui.label(RichText::new(value).size(13.0).color(tone::text()));
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
        egui::lerp(tone::stage().r() as f32..=tone::accent().r() as f32, t) as u8,
        egui::lerp(tone::stage().g() as f32..=tone::accent().g() as f32, t) as u8,
        egui::lerp(tone::stage().b() as f32..=tone::accent().b() as f32, t) as u8,
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
    ui.painter().circle_filled(
        egui::pos2(knob_x, rect.center().y),
        knob_r.max(4.0),
        Color32::WHITE,
    );
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
/// 包动态键：`{pack_id}.{field}`，缺失时回退 host 元数据。
fn pack_field(
    catalogs: &CatalogStore,
    locale: Locale,
    pack_id: &str,
    field: &str,
    fallback: &str,
) -> String {
    catalogs
        .t(locale, &format!("{pack_id}.{field}"), fallback)
        .to_string()
}

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
            .rect_filled(rect, CornerRadius::same(8), tone::hover());
    }

    let chev = egui::Rect::from_min_size(
        egui::pos2(rect.left(), rect.center().y - 16.0),
        Vec2::new(plugin_layout::CHEV_W, 32.0),
    );
    paint_expand_chevron(ui, response.id.with("chev"), chev, open, response.hovered());

    let badge = egui::Rect::from_min_size(
        egui::pos2(
            rect.left() + plugin_layout::icon_left(),
            rect.center().y - plugin_layout::ICON * 0.5,
        ),
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
        tone::muted(),
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

/// 设置下拉箭头：按 salt 独立动画 id，避免多个 Combo 抢同一帧动画。
fn settings_combo_chevron(ui: &egui::Ui, salt: &str, rect: egui::Rect, open: bool) {
    let hovered = ui.rect_contains_pointer(rect.expand(8.0));
    paint_stroke_chevron(
        ui,
        egui::Id::new(("settings_combo_chev", salt)),
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
    let stroke = Stroke::new(1.6, if hovered { tone::text() } else { tone::muted() });
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
        .rect_filled(rect, CornerRadius::same(10), tone::accent_soft());
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
        tone::accent(),
    );
}

/// HUD 条目图标：包内图优先，否则程序默认图标（尺寸与插件图标对齐）。
fn hud_item_icon(ui: &mut egui::Ui, icon: Option<&TextureHandle>) {
    let size = Vec2::splat(plugin_layout::ICON);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    ui.painter()
        .rect_filled(rect, CornerRadius::same(10), tone::stage());
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
    let stroke = Stroke::new(1.5, tone::muted());
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
    ui.painter()
        .line_segment([c + Vec2::new(-5.0, 2.0), c + Vec2::new(2.5, 2.0)], stroke);
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
            .fill(tone::card())
            .stroke(Stroke::new(1.0, tone::line()))
            .corner_radius(CornerRadius::same(10))
            .inner_margin(Margin::symmetric(12, 10))
            .show(ui, |ui| {
                ui.label(RichText::new(&name).size(14.0).strong().color(tone::text()));
                if !description.is_empty() {
                    ui.add_space(4.0);
                    ui.label(RichText::new(&description).size(12.5).color(tone::muted()));
                }
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(6.0);
                ui.label(RichText::new(&id).size(12.0).color(tone::text()));
                ui.label(RichText::new(&author_line).size(11.5).color(tone::muted()));
                if let Some(ex) = &extra {
                    ui.label(RichText::new(ex).size(11.5).color(tone::muted()));
                }
                if let Some(url) = &homepage {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(&homepage_label)
                                .size(11.0)
                                .color(tone::muted()),
                        );
                        ui.hyperlink_to(RichText::new(url).size(11.0), url);
                    });
                }
            });
    })
}

fn opaque_tooltip_visuals(ui: &mut egui::Ui) {
    let v = ui.visuals_mut();
    v.window_fill = tone::card();
    v.panel_fill = tone::card();
    v.extreme_bg_color = tone::card();
    v.override_text_color = Some(tone::text());
}

fn nav_item(ui: &mut egui::Ui, label: &str, selected: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), 36.0), Sense::click());
    let bg = if selected {
        tone::card()
    } else if response.hovered() {
        tone::hover()
    } else {
        Color32::TRANSPARENT
    };
    if bg.a() > 0 {
        ui.painter().rect_filled(rect, CornerRadius::same(8), bg);
    }
    if selected {
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.left(), rect.top() + 8.0),
            Vec2::new(3.0, rect.height() - 16.0),
        );
        ui.painter()
            .rect_filled(bar, CornerRadius::same(2), tone::accent());
    }
    ui.painter().text(
        egui::pos2(rect.left() + 14.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        if selected {
            tone::accent()
        } else {
            tone::text()
        },
    );
    response
}

fn resolve_card_layout(s: &mut SettingsState, avail_w: f32) -> (usize, f32, f32, f32) {
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

    s.card_layout.unwrap_or_else(|| pet_card_layout(avail_w))
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
    const H: f32 = 28.0;
    const CELL: f32 = 32.0;
    let (rect, _) = ui.allocate_exact_size(Vec2::new(CELL * 2.0, H), Sense::hover());

    // 一体轨道：单一描边，避免双线缝隙
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        tone::faint(),
        Stroke::new(1.0, tone::line()),
        egui::StrokeKind::Inside,
    );

    let inner = rect.shrink(1.0);
    let mid_x = inner.center().x;
    let left = egui::Rect::from_min_max(inner.min, egui::pos2(mid_x, inner.bottom()));
    let right = egui::Rect::from_min_max(egui::pos2(mid_x, inner.top()), inner.max);

    let grid_sel = mode == PetPickerMode::Grid;
    let list_sel = mode == PetPickerMode::List;

    let grid_r = ui.interact(left, ui.id().with("pet_view_grid"), Sense::click());
    let list_r = ui.interact(right, ui.id().with("pet_view_list"), Sense::click());

    if grid_sel {
        ui.painter().rect_filled(
            left,
            CornerRadius {
                nw: 7,
                ne: 0,
                sw: 7,
                se: 0,
            },
            tone::card(),
        );
    } else if grid_r.hovered() {
        ui.painter().rect_filled(
            left,
            CornerRadius {
                nw: 7,
                ne: 0,
                sw: 7,
                se: 0,
            },
            tone::hover(),
        );
    }
    if list_sel {
        ui.painter().rect_filled(
            right,
            CornerRadius {
                nw: 0,
                ne: 7,
                sw: 0,
                se: 7,
            },
            tone::card(),
        );
    } else if list_r.hovered() {
        ui.painter().rect_filled(
            right,
            CornerRadius {
                nw: 0,
                ne: 7,
                sw: 0,
                se: 7,
            },
            tone::hover(),
        );
    }

    ui.painter().line_segment(
        [
            egui::pos2(mid_x, inner.top() + 5.0),
            egui::pos2(mid_x, inner.bottom() - 5.0),
        ],
        Stroke::new(1.0, tone::line()),
    );

    draw_grid_icon(
        ui.painter(),
        left.center(),
        if grid_sel {
            tone::accent()
        } else {
            tone::muted()
        },
    );
    draw_list_icon(
        ui.painter(),
        right.center(),
        if list_sel {
            tone::accent()
        } else {
            tone::muted()
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

/// 草稿相对基准是否有可应用改动（忽略设置窗几何；几何随关窗落盘）。
fn settings_draft_dirty(draft: &UiPreferences, baseline: &UiPreferences) -> bool {
    let mut a = draft.clone();
    let mut b = baseline.clone();
    a.shell.settings_width = None;
    a.shell.settings_height = None;
    a.shell.settings_pos_x = None;
    a.shell.settings_pos_y = None;
    b.shell.settings_width = None;
    b.shell.settings_height = None;
    b.shell.settings_pos_x = None;
    b.shell.settings_pos_y = None;
    a != b
}

fn footer_primary_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let size = Vec2::new(88.0, 32.0);
    let (rect, response) = ui.allocate_exact_size(size, Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        tone::accent_press()
    } else if response.hovered() {
        tone::accent_hover()
    } else {
        tone::accent()
    };
    ui.painter().rect_filled(rect, CornerRadius::same(8), fill);
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
        tone::stage()
    } else if response.hovered() {
        tone::hover()
    } else {
        tone::faint()
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(8),
        fill,
        Stroke::new(1.0, tone::line()),
        egui::StrokeKind::Inside,
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        label,
        FontId::proportional(14.0),
        tone::text(),
    );
    response
}

/// 插件页「插件布局」：整行操作按钮（左图标 + 文案）。
fn hud_layout_action_button(ui: &mut egui::Ui, label: &str) -> egui::Response {
    let height = 40.0;
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, height), Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        tone::accent_soft()
    } else if response.hovered() {
        Color32::from_rgba_unmultiplied(
            tone::accent_soft().r(),
            tone::accent_soft().g(),
            tone::accent_soft().b(),
            180,
        )
    } else {
        tone::faint()
    };
    let stroke = if response.hovered() || response.is_pointer_button_down_on() {
        Stroke::new(1.0, tone::accent())
    } else {
        Stroke::new(1.0, tone::line())
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(10),
        fill,
        stroke,
        egui::StrokeKind::Inside,
    );
    let icon_c = egui::pos2(rect.left() + 22.0, rect.center().y);
    draw_layout_edit_icon(
        ui.painter(),
        icon_c,
        if response.hovered() {
            tone::accent()
        } else {
            tone::text()
        },
    );
    ui.painter().text(
        egui::pos2(rect.left() + 40.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(14.0),
        tone::text(),
    );
    response
}

fn draw_layout_edit_icon(painter: &egui::Painter, center: egui::Pos2, color: Color32) {
    let s = 5.0;
    let g = 2.5;
    let origin = center - Vec2::new(s + g * 0.5, s + g * 0.5);
    for row in 0..2 {
        for col in 0..2 {
            let p = origin + Vec2::new(col as f32 * (s + g), row as f32 * (s + g));
            painter.rect_stroke(
                egui::Rect::from_min_size(p, Vec2::splat(s)),
                1.0,
                Stroke::new(1.35, color),
                egui::StrokeKind::Outside,
            );
        }
    }
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
    // 四行信息：按实际行高 + 行距排布，预览框边长与信息块等高
    const NAME_SIZE: f32 = 15.0;
    const DESC_SIZE: f32 = 12.0;
    const META_SIZE: f32 = 11.5;
    const SIZE_SIZE: f32 = 11.0;
    const LINE_GAP: f32 = 4.0;
    let (h_name, h_desc, h_meta, h_size) = ui.fonts_mut(|f| {
        (
            f.row_height(&FontId::proportional(NAME_SIZE)),
            f.row_height(&FontId::proportional(DESC_SIZE)),
            f.row_height(&FontId::proportional(META_SIZE)),
            f.row_height(&FontId::proportional(SIZE_SIZE)),
        )
    });
    let text_block_h = h_name + h_desc + h_meta + h_size + LINE_GAP * 3.0;
    let thumb_side = text_block_h.max(72.0);
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
        tone::accent_soft()
    } else if response.hovered() {
        tone::hover()
    } else {
        tone::card()
    };
    let stroke = if selected {
        Stroke::new(1.5, tone::selected_ring())
    } else {
        Stroke::new(1.0, tone::line())
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
        .rect_filled(thumb, CornerRadius::same(8), tone::stage());
    if let Some(tex) = preview {
        paint_preview_cover(ui, thumb, tex);
    }

    let text_left = thumb.right() + CARD_PAD;
    let text_right = draw.right() - CARD_PAD;
    // 信息块在预览框高度内垂直居中
    let text_top = thumb.top() + ((thumb_side - text_block_h) * 0.5).max(0.0);
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
    let name_font = FontId::proportional(NAME_SIZE);
    let desc_font = FontId::proportional(DESC_SIZE);
    let meta_font = FontId::proportional(META_SIZE);
    let size_font = FontId::proportional(SIZE_SIZE);
    let name_draw = truncate_ui_text(ui, name, name_font.clone(), name_max);
    let desc_draw = truncate_ui_text(
        ui,
        description,
        desc_font.clone(),
        (text_right - text_left).max(40.0),
    );
    let author_draw = truncate_ui_text(
        ui,
        author_label,
        meta_font.clone(),
        (text_right - text_left).max(40.0),
    );
    let size_draw = truncate_ui_text(
        ui,
        size_label,
        size_font.clone(),
        (text_right - text_left).max(40.0),
    );

    let mut y = text_top;
    ui.painter().text(
        egui::pos2(text_left, y),
        Align2::LEFT_TOP,
        name_draw,
        name_font,
        tone::text(),
    );
    y += h_name + LINE_GAP;
    ui.painter().text(
        egui::pos2(text_left, y),
        Align2::LEFT_TOP,
        desc_draw,
        desc_font,
        tone::muted(),
    );
    y += h_desc + LINE_GAP;
    ui.painter().text(
        egui::pos2(text_left, y),
        Align2::LEFT_TOP,
        author_draw,
        meta_font,
        tone::muted(),
    );
    y += h_meta + LINE_GAP;
    ui.painter().text(
        egui::pos2(text_left, y),
        Align2::LEFT_TOP,
        size_draw,
        size_font,
        tone::muted(),
    );
    if selected {
        // 相对整行内容区（与预览框同高）垂直居中，而非贴在名称行顶
        ui.painter().text(
            egui::pos2(text_right, thumb.center().y),
            Align2::RIGHT_CENTER,
            selected_badge,
            FontId::proportional(12.0),
            tone::accent(),
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
        tone::accent_soft()
    } else if response.hovered() {
        tone::hover()
    } else {
        tone::card()
    };
    let stroke = if selected {
        Stroke::new(1.5, tone::selected_ring())
    } else if response.hovered() {
        Stroke::new(1.2, tone::line())
    } else {
        Stroke::new(1.0, tone::line())
    };
    ui.painter().rect(
        draw,
        CornerRadius::same(12),
        bg,
        stroke,
        egui::StrokeKind::Inside,
    );

    let side = preview_side.min(draw.width() - CARD_PAD * 2.0).max(1.0);
    let stage = egui::Rect::from_center_size(
        egui::pos2(draw.center().x, draw.top() + CARD_PAD + side * 0.5),
        Vec2::splat(side),
    );
    ui.painter()
        .rect_filled(stage, CornerRadius::same(10), tone::stage());

    if let Some(tex) = preview {
        paint_preview_cover(ui, stage, tex);
    } else {
        ui.painter().text(
            stage.center(),
            Align2::CENTER_CENTER,
            "—",
            FontId::proportional(28.0),
            tone::muted(),
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
        tone::text(),
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + 20.0),
        Align2::LEFT_TOP,
        desc_draw,
        FontId::proportional(12.0),
        tone::muted(),
    );
    ui.painter().text(
        egui::pos2(text_left, text_top + 38.0),
        Align2::LEFT_TOP,
        author_draw,
        FontId::proportional(11.5),
        tone::muted(),
    );
    ui.painter().text(
        egui::pos2(text_left, text_bottom),
        Align2::LEFT_BOTTOM,
        size_draw,
        FontId::proportional(11.5),
        tone::muted(),
    );
    if selected {
        ui.painter().text(
            egui::pos2(text_right, text_bottom),
            Align2::RIGHT_BOTTOM,
            selected_badge,
            FontId::proportional(11.5),
            tone::accent(),
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
