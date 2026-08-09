//! 右键菜单：设置 / 置顶 / 插件 / 插件布局 / 退出。

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use eframe::egui::{self, Align2, Color32, CornerRadius, FontId, Frame, Margin, Sense, Stroke, Vec2};
use deskhud_ui::{MessageKey, UiPreferences};

use crate::platform;

fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("deskhud_pet_menu")
}

const ROW_H: f32 = 34.0;
const SEP_H: f32 = 9.0;
const MENU_MIN_W: f32 = 160.0;
const MENU_MAX_CONTENT_W: f32 = 240.0;
const MENU_PAD_X: f32 = 8.0;
const MENU_PAD_Y: f32 = 8.0;
const TEXT_INSET: f32 = 10.0;
const DISMISS_GRACE: Duration = Duration::from_millis(280);
const LABEL_FONT: f32 = 13.5;
/// 菜单行数：设置 / 置顶 / 插件 / 插件布局 / 退出。
const MENU_ROWS: usize = 5;
/// 分隔线条数。
const MENU_SEPS: usize = 2;

mod menu_tone {
    use eframe::egui::{Color32, Context, Theme};

    pub fn sync_dark(ctx: &Context) -> bool {
        matches!(ctx.theme(), Theme::Dark)
    }

    pub fn fill(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(43, 45, 49)
        } else {
            Color32::from_rgb(248, 248, 252)
        }
    }

    pub fn text(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(232, 234, 237)
        } else {
            Color32::from_rgb(28, 28, 32)
        }
    }

    pub fn hover(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(52, 54, 60)
        } else {
            Color32::from_rgb(232, 236, 244)
        }
    }

    pub fn line(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(60, 64, 72)
        } else {
            Color32::from_rgb(220, 222, 230)
        }
    }

    pub fn check(dark: bool) -> Color32 {
        if dark {
            Color32::from_rgb(110, 168, 255)
        } else {
            Color32::from_rgb(46, 120, 210)
        }
    }
}

fn menu_height() -> f32 {
    // 与 Frame 上下边距一致；行间 item_spacing 在 draw 里清零，避免底项被裁
    MENU_PAD_Y * 2.0
        + MENU_ROWS as f32 * ROW_H
        + MENU_SEPS as f32 * SEP_H
        + 2.0
}

fn measure_text_width(ctx: &egui::Context, text: &str) -> f32 {
    let font = FontId::proportional(LABEL_FONT);
    ctx.fonts_mut(|f| {
        f.layout_no_wrap(text.to_string(), font, Color32::WHITE)
            .size()
            .x
    })
}

fn compute_menu_width(ctx: &egui::Context, labels: &[String]) -> f32 {
    let mut content = 64.0_f32;
    for label in labels {
        content = content.max(measure_text_width(ctx, label));
    }
    let content = content.min(MENU_MAX_CONTENT_W);
    (content + TEXT_INSET + MENU_PAD_X * 2.0)
        .clamp(MENU_MIN_W, MENU_MAX_CONTENT_W + TEXT_INSET + MENU_PAD_X * 2.0)
}

fn ellipsize(ctx: &egui::Context, text: &str, max_w: f32) -> String {
    if measure_text_width(ctx, text) <= max_w {
        return text.to_string();
    }
    let ell = "…";
    let ell_w = measure_text_width(ctx, ell);
    if ell_w >= max_w {
        return ell.to_string();
    }
    let chars: Vec<char> = text.chars().collect();
    let mut lo = 0usize;
    let mut hi = chars.len();
    while lo < hi {
        let mid = (lo + hi + 1) / 2;
        let cand: String = chars[..mid].iter().collect::<String>() + ell;
        if measure_text_width(ctx, &cand) <= max_w {
            lo = mid;
        } else {
            hi = mid - 1;
        }
    }
    chars[..lo].iter().collect::<String>() + ell
}

/// 与主壳共享的右键菜单状态。
#[derive(Clone)]
pub struct PetMenuHost {
    inner: Arc<Mutex<PetMenuState>>,
}

#[derive(Debug)]
pub struct PetMenuState {
    pub open: bool,
    pub focus_once: bool,
    pub opened_at: Instant,
    pub menu_hwnd: Option<isize>,
    pub pet_hwnd: Option<isize>,
    /// 设置窗 HWND：chrome 绑定时必须排除。
    pub settings_hwnd: Option<isize>,
    pub anchor: egui::Pos2,
    pub cursor: egui::Pos2,
    pub ppp: f32,
    pub menu_w: f32,
    pub width_ready: bool,
    pub locale_prefs: UiPreferences,
    /// 当前全局插件（HUD）是否启用。
    pub master_enabled: bool,
    /// 当前宠窗是否置顶。
    pub pet_topmost: bool,
    pub open_settings: bool,
    pub begin_hud_layout: bool,
    /// 点击后请求切换全局启用：`Some(new_enabled)`。
    pub toggle_master: Option<bool>,
    /// 点击后请求切换宠置顶：`Some(new_topmost)`。
    pub toggle_topmost: Option<bool>,
    pub quit: bool,
}

impl PetMenuHost {
    pub fn new(prefs: UiPreferences) -> Self {
        Self {
            inner: Arc::new(Mutex::new(PetMenuState {
                open: false,
                focus_once: false,
                opened_at: Instant::now(),
                menu_hwnd: None,
                pet_hwnd: None,
                settings_hwnd: None,
                anchor: egui::pos2(0.0, 0.0),
                cursor: egui::pos2(0.0, 0.0),
                ppp: 1.0,
                menu_w: MENU_MIN_W,
                width_ready: false,
                locale_prefs: prefs,
                master_enabled: true,
                pet_topmost: true,
                open_settings: false,
                begin_hud_layout: false,
                toggle_master: None,
                toggle_topmost: None,
                quit: false,
            })),
        }
    }

    pub fn lock(&self) -> std::sync::MutexGuard<'_, PetMenuState> {
        self.inner.lock().unwrap_or_else(|e| e.into_inner())
    }

    pub fn is_open(&self) -> bool {
        self.lock().open
    }

    pub fn dismiss(&self, ctx: &egui::Context) {
        if self.lock().open {
            self.close_viewport(ctx);
        }
    }

    pub fn open_at(
        &self,
        prefs: &UiPreferences,
        cursor_points: egui::Pos2,
        ppp: f32,
        master_enabled: bool,
        pet_topmost: bool,
    ) {
        let provisional_w = MENU_MIN_W.max(180.0);
        let (x, y) = platform::fit_popup_pos_points(
            (cursor_points.x, cursor_points.y),
            provisional_w,
            menu_height(),
            ppp,
        );
        let mut s = self.lock();
        s.open = true;
        s.focus_once = true;
        s.opened_at = Instant::now();
        s.menu_hwnd = None;
        s.cursor = cursor_points;
        s.ppp = ppp;
        s.menu_w = provisional_w;
        s.width_ready = false;
        s.anchor = egui::pos2(x, y);
        s.locale_prefs = prefs.clone();
        s.master_enabled = master_enabled;
        s.pet_topmost = pet_topmost;
        s.open_settings = false;
        s.begin_hud_layout = false;
        s.toggle_master = None;
        s.toggle_topmost = None;
        s.quit = false;
    }

    pub fn show(
        &self,
        ctx: &egui::Context,
        pet_hwnd: Option<isize>,
        settings_hwnd: Option<isize>,
    ) {
        if !self.lock().open {
            return;
        }
        self.lock().settings_hwnd = settings_hwnd;
        // 设置页打开且在草稿预览主题时，勿用已应用 prefs 覆盖全局主题
        if settings_hwnd.is_none() {
            let theme = self.lock().locale_prefs.shell.ui_theme;
            crate::theme::apply(ctx, theme);
        }

        self.lock().pet_hwnd = pet_hwnd;
        self.maybe_dismiss_outside(ctx, pet_hwnd);
        if !self.lock().open {
            return;
        }

        if !self.lock().width_ready {
            let (labels, ppp, cursor) = {
                let s = self.lock();
                let labels = vec![
                    s.locale_prefs.t(MessageKey::MenuSettings).to_string(),
                    s.locale_prefs.t(MessageKey::SettingsTopmost).to_string(),
                    s.locale_prefs.t(MessageKey::SettingsNavHud).to_string(),
                    s.locale_prefs.t(MessageKey::MenuHudLayout).to_string(),
                    s.locale_prefs.t(MessageKey::MenuQuit).to_string(),
                ];
                (labels, s.ppp, s.cursor)
            };
            let w = compute_menu_width(ctx, &labels);
            let mut s = self.lock();
            s.menu_w = w;
            s.width_ready = true;
            let (x, y) =
                platform::fit_popup_pos_points((cursor.x, cursor.y), w, menu_height(), ppp);
            s.anchor = egui::pos2(x, y);
        }

        let (anchor, focus_once, menu_w, topmost) = {
            let s = self.lock();
            (s.anchor, s.focus_once, s.menu_w, s.pet_topmost)
        };
        let height = menu_height();
        let shared = self.clone();
        let level = if topmost {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        let mut builder = egui::ViewportBuilder::default()
            .with_title("")
            .with_decorations(false)
            .with_title_shown(false)
            .with_titlebar_shown(false)
            .with_titlebar_buttons_shown(false)
            .with_position(anchor)
            .with_inner_size([menu_w, height])
            .with_transparent(false)
            .with_resizable(false)
            .with_taskbar(false)
            .with_active(true)
            .with_visible(true)
            .with_window_level(level);
        if topmost {
            builder = builder.with_always_on_top();
        }
        ctx.show_viewport_deferred(
            viewport_id(),
            builder,
            move |ui, _| shared.draw(ui),
        );

        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Decorations(false));
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::OuterPosition(anchor));
        ctx.send_viewport_cmd_to(
            viewport_id(),
            egui::ViewportCommand::InnerSize(egui::vec2(menu_w, height)),
        );
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::WindowLevel(level));
        if focus_once {
            self.lock().focus_once = false;
            ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
        }
    }

    fn maybe_dismiss_outside(&self, ctx: &egui::Context, pet_hwnd: Option<isize>) {
        let (opened_at, menu_hwnd) = {
            let s = self.lock();
            if !s.open {
                return;
            }
            (s.opened_at, s.menu_hwnd)
        };
        if opened_at.elapsed() < DISMISS_GRACE {
            return;
        }
        if platform::foreground_is_outside(pet_hwnd, menu_hwnd) {
            self.close_viewport(ctx);
        }
    }

    fn draw(&self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        // 设置页草稿主题预览优先；菜单仅在设置关闭时推主题
        if self.lock().settings_hwnd.is_none() {
            let theme = self.lock().locale_prefs.shell.ui_theme;
            crate::theme::apply(&ctx, theme);
        }
        {
            let mut s = self.lock();
            let pet = s.pet_hwnd;
            let settings = s.settings_hwnd;
            // 若误把设置窗当成菜单，立刻丢掉并还原装饰
            if let Some(h) = s.menu_hwnd {
                if settings == Some(h) || pet == Some(h) {
                    platform::release_popup_chrome(Some(h));
                    if settings == Some(h) {
                        platform::ensure_settings_chrome(h);
                    }
                    s.menu_hwnd = None;
                }
            }
            if s.menu_hwnd.is_none() {
                if let Some(h) = platform::foreground_hwnd() {
                    let forbidden = pet == Some(h) || settings == Some(h);
                    let size_ok = menu_hwnd_size_ok(h, s.menu_w, s.ppp);
                    if !forbidden && size_ok {
                        s.menu_hwnd = Some(h);
                    }
                }
            }
            if let Some(h) = s.menu_hwnd {
                // 每帧维持（同宠窗）：获焦时系统会画回 NC 白条
                let dark = menu_tone::sync_dark(&ctx);
                platform::ensure_acrylic_popup(h, pet, dark);
            }
        }

        if ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || ctx.input(|i| i.viewport().close_requested())
        {
            self.close_viewport(&ctx);
            return;
        }
        let focused = ctx.input(|i| i.viewport().focused).unwrap_or(true);
        if self.lock().opened_at.elapsed() >= DISMISS_GRACE && !focused {
            self.close_viewport(&ctx);
            return;
        }

        let dark = menu_tone::sync_dark(&ctx);
        let fill = menu_tone::fill(dark);
        ui.painter()
            .rect_filled(ui.max_rect(), CornerRadius::ZERO, fill);

        let mut close = false;
        egui::CentralPanel::default()
            .frame(
                Frame::NONE
                    .fill(fill)
                    .stroke(Stroke::NONE)
                    .inner_margin(Margin::symmetric(MENU_PAD_X as i8, MENU_PAD_Y as i8)),
            )
            .show(ui, |ui| {
                // 高度按固定行高计算，禁止 egui 默认行距把底项挤出视口
                ui.spacing_mut().item_spacing.y = 0.0;
                let mut s = self.lock();
                let settings = s.locale_prefs.t(MessageKey::MenuSettings).to_string();
                let topmost_l = s.locale_prefs.t(MessageKey::SettingsTopmost).to_string();
                let plugins = s.locale_prefs.t(MessageKey::SettingsNavHud).to_string();
                let layout = s.locale_prefs.t(MessageKey::MenuHudLayout).to_string();
                let quit = s.locale_prefs.t(MessageKey::MenuQuit).to_string();
                let max_text_w = (ui.available_width() - TEXT_INSET).max(24.0);
                let master_on = s.master_enabled;
                let topmost_on = s.pet_topmost;

                if action_row(ui, &ellipsize(&ctx, &settings, max_text_w), dark).clicked() {
                    s.open_settings = true;
                    close = true;
                }

                if check_row(ui, &ellipsize(&ctx, &topmost_l, max_text_w), topmost_on, dark)
                    .clicked()
                {
                    s.toggle_topmost = Some(!topmost_on);
                    close = true;
                }

                menu_separator(ui, dark);

                if check_row(ui, &ellipsize(&ctx, &plugins, max_text_w), master_on, dark)
                    .clicked()
                {
                    s.toggle_master = Some(!master_on);
                    close = true;
                }

                ui.add_enabled_ui(master_on, |ui| {
                    if action_row(ui, &ellipsize(&ctx, &layout, max_text_w), dark).clicked() {
                        s.begin_hud_layout = true;
                        close = true;
                    }
                });

                menu_separator(ui, dark);

                if action_row(ui, &ellipsize(&ctx, &quit, max_text_w), dark).clicked() {
                    s.quit = true;
                    close = true;
                }
            });

        if close {
            self.close_viewport(&ctx);
        }
    }

    fn close_viewport(&self, ctx: &egui::Context) {
        let mut s = self.lock();
        let menu_hwnd = s.menu_hwnd;
        s.open = false;
        s.menu_hwnd = None;
        s.width_ready = false;
        drop(s);
        platform::release_popup_chrome(menu_hwnd);
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::CancelClose);
        ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Visible(false));
    }
}

fn menu_hwnd_size_ok(hwnd: isize, menu_w_points: f32, ppp: f32) -> bool {
    let Some((l, t, r, b)) = platform::window_screen_rect(hwnd) else {
        return false;
    };
    let ppp = ppp.max(0.01);
    let expect_w = (menu_w_points * ppp).round();
    let expect_h = (menu_height() * ppp).round();
    let w = (r - l) as f32;
    let h = (b - t) as f32;
    // 允许边框/DPI 误差；设置窗远大于此
    (w - expect_w).abs() <= 48.0 && (h - expect_h).abs() <= 48.0
}

fn menu_separator(ui: &mut egui::Ui, dark: bool) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(ui.available_width(), SEP_H), Sense::hover());
    let y = rect.center().y;
    ui.painter().line_segment(
        [
            egui::pos2(rect.left() + 4.0, y),
            egui::pos2(rect.right() - 4.0, y),
        ],
        Stroke::new(1.0, menu_tone::line(dark)),
    );
}

fn action_row(ui: &mut egui::Ui, label: &str, dark: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), menu_tone::hover(dark));
    }
    ui.painter().text(
        egui::pos2(rect.left() + TEXT_INSET, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(LABEL_FONT),
        menu_tone::text(dark),
    );
    response
}

fn check_row(ui: &mut egui::Ui, label: &str, checked: bool, dark: bool) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(ui.available_width(), ROW_H), Sense::click());
    if response.hovered() {
        ui.painter()
            .rect_filled(rect, CornerRadius::same(6), menu_tone::hover(dark));
    }
    ui.painter().text(
        egui::pos2(rect.left() + TEXT_INSET, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(LABEL_FONT),
        menu_tone::text(dark),
    );
    if checked {
        ui.painter().text(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "✓",
            FontId::proportional(14.0),
            menu_tone::check(dark),
        );
    }
    response
}
