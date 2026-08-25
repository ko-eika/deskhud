//! 可复用的独立菜单窗口，支持勾选项和悬浮二级菜单。

#![cfg_attr(target_os = "macos", allow(dead_code))]

pub(crate) mod controller;
pub(crate) mod placement;

pub(crate) use controller::MenuController;

use egui::{Align2, Color32, Context, FontId, RawInput, Sense, Stroke, Vec2};
use winit::{
    dpi::PhysicalPosition,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::WindowId,
};

use crate::runtime::{
    viewport::{UserEvent, Viewport},
    viewport_config::ViewportConfig,
};
use crate::views::ViewOutput;

const MENU_LEFT_ICON_WIDTH: f32 = 24.0;
const MENU_RIGHT_ICON_WIDTH: f32 = 24.0;
const MENU_PADDING: f32 = 8.0;
const MENU_TEXT_GAP: f32 = 8.0;
const MENU_ITEM_GAP: f32 = MENU_PADDING;
const MENU_ITEM_HEIGHT: f32 = 28.0;
const MENU_TITLE_HEIGHT: f32 = 24.0;
const MENU_SEPARATOR_HEIGHT: f32 = MENU_ITEM_GAP * 2.0 + 1.0;
const MENU_FONT_SIZE: f32 = 14.0;

/// 菜单窗口的原生配置。
pub(crate) struct MenuConfig {
    /// 窗口标题及 egui 菜单标题。
    pub(crate) title: &'static str,
    /// 是否在菜单内容顶部显示标题。
    pub(crate) show_title: bool,
    /// 菜单初始逻辑尺寸。
    pub(crate) size: [f64; 2],
    /// 菜单使用的 egui 视口标识。
    pub(crate) egui_id: egui::ViewportId,
    /// 是否显示系统装饰。
    pub(crate) decorations: bool,
    /// 是否使用透明窗口背景。
    pub(crate) transparent: bool,
    /// 是否从任务栏隐藏。
    pub(crate) skip_taskbar: bool,
    /// 是否保持窗口置顶。
    pub(crate) always_on_top: bool,
    /// 无边框窗口是否保留系统阴影。
    pub(crate) undecorated_shadow: bool,
    pub(crate) x11_popup: bool,
    /// 打开时是否请求窗口焦点。
    pub(crate) focus_on_show: bool,
    /// 是否因窗口失去焦点而自动关闭。
    pub(crate) close_on_focus_loss: bool,
}

impl Default for MenuConfig {
    fn default() -> Self {
        Self {
            title: "Menu",
            show_title: false,
            size: [210.0, 180.0],
            egui_id: egui::ViewportId::from_hash_of("menu"),
            decorations: false,
            transparent: false,
            skip_taskbar: true,
            always_on_top: true,
            undecorated_shadow: true,
            x11_popup: false,
            focus_on_show: true,
            close_on_focus_loss: true,
        }
    }
}

/// 菜单中的一项。
pub(crate) struct MenuItem {
    /// 菜单项稳定标识，用于转换为业务动作。
    pub(crate) id: &'static str,
    /// 菜单项显示文本。
    pub(crate) label: &'static str,
    /// 是否显示选中状态。
    pub(crate) checked: bool,
    /// 是否允许点击。
    pub(crate) enabled: bool,
    /// 是否在该项前绘制分隔线。
    pub(crate) separator_before: bool,
    /// 可选的子菜单定义。
    pub(crate) submenu: Option<Box<MenuDefinition>>,
}

impl MenuItem {
    pub(crate) const fn new(id: &'static str, label: &'static str) -> Self {
        Self {
            id,
            label,
            checked: false,
            enabled: true,
            separator_before: false,
            submenu: None,
        }
    }

    pub(crate) const fn checkable(id: &'static str, label: &'static str, checked: bool) -> Self {
        Self {
            id,
            label,
            checked,
            enabled: true,
            separator_before: false,
            submenu: None,
        }
    }

    pub(crate) const fn with_separator_before(mut self) -> Self {
        self.separator_before = true;
        self
    }

    pub(crate) const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    pub(crate) fn with_submenu(mut self, submenu: MenuDefinition) -> Self {
        self.submenu = Some(Box::new(submenu));
        self
    }
}

/// 一次菜单展示所需的完整定义。
pub(crate) struct MenuDefinition {
    /// 当前层级的菜单项列表。
    pub(crate) items: Vec<MenuItem>,
}

impl MenuDefinition {
    pub(crate) fn new(items: Vec<MenuItem>) -> Self {
        Self { items }
    }
}

pub(crate) struct MenuOutput {
    /// 通用视口输出。
    pub(crate) viewport: crate::runtime::viewport::ViewportOutput,
    /// 本帧被点击的菜单项。
    pub(crate) selected_item: Option<String>,
    /// 请求打开的子菜单索引。
    pub(crate) open_submenu: Option<usize>,
    /// 当前悬浮菜单项索引。
    pub(crate) hovered_item: Option<usize>,
    /// 当前子菜单触发区域。
    pub(crate) submenu_anchor: Option<[f32; 3]>,
}

pub(crate) struct MenuWindow {
    /// 通用视口运行时。
    viewport: Viewport,
    /// 菜单窗口标题。
    title: &'static str,
    /// 是否绘制菜单标题。
    show_title: bool,
    /// 打开时是否获取焦点。
    focus_on_show: bool,
}

impl MenuWindow {
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        config: MenuConfig,
        proxy: &EventLoopProxy<UserEvent>,
    ) -> Self {
        let viewport_config = ViewportConfig {
            title: config.title,
            size: config.size,
            egui_id: config.egui_id,
            decorations: config.decorations,
            transparent: config.transparent,
            resizable: false,
            drag_anywhere: false,
            skip_taskbar: config.skip_taskbar,
            visible: false,
            always_on_top: config.always_on_top,
            undecorated_shadow: config.undecorated_shadow,
            x11_popup: config.x11_popup,
        };
        Self {
            viewport: Viewport::new(event_loop, viewport_config, proxy),
            title: config.title,
            show_title: config.show_title,
            focus_on_show: config.focus_on_show,
        }
    }

    pub(crate) fn open(&mut self, anchor: PhysicalPosition<i32>, definition: &MenuDefinition) {
        let _ = definition;
        self.set_visible(true);
        self.set_outer_position(placement::choose_position(self.viewport.window(), anchor));
    }

    pub(crate) fn open_at(&mut self, position: PhysicalPosition<i32>, definition: &MenuDefinition) {
        let _ = definition;
        self.set_visible(true);
        self.set_outer_position(position);
    }

    fn set_visible(&mut self, visible: bool) {
        if self.focus_on_show {
            self.viewport.set_visible(visible);
        } else {
            self.viewport.set_visible_without_focus(visible);
        }
    }

    pub(crate) fn close(&mut self) {
        self.set_visible(false);
    }
    pub(crate) fn is_visible(&self) -> bool {
        self.viewport.is_visible()
    }
    pub(crate) fn window_id(&self) -> WindowId {
        self.viewport.window_id()
    }

    pub(crate) fn window_handle(&self) -> std::sync::Arc<winit::window::Window> {
        self.viewport.window_handle()
    }

    pub(crate) fn handle_event(&mut self, event: &WindowEvent, close_on_focus_loss: bool) {
        if matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed)
            || (close_on_focus_loss && matches!(event, WindowEvent::Focused(false)))
        {
            self.close();
        } else if !matches!(event, WindowEvent::RedrawRequested) {
            self.viewport.handle_event(event);
        }
    }

    pub(crate) fn render(
        &mut self,
        definition: &MenuDefinition,
        highlighted_submenu: Option<usize>,
    ) -> MenuOutput {
        let viewport = self.viewport.render(|context, raw_input| {
            run(
                context,
                raw_input,
                definition,
                self.title,
                self.show_title,
                highlighted_submenu,
            )
        });
        let selected_item = viewport.selected_menu_item.clone();
        MenuOutput {
            open_submenu: viewport.open_submenu,
            hovered_item: viewport.hovered_item,
            submenu_anchor: viewport.submenu_anchor,
            viewport,
            selected_item,
        }
    }

    pub(crate) fn set_outer_position(&self, position: PhysicalPosition<i32>) {
        if self.viewport.window().outer_position().ok() != Some(position) {
            self.viewport.request_outer_position(position);
        }
    }

    pub(crate) fn outer_size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.viewport.window().outer_size()
    }

    pub(crate) fn window(&self) -> &winit::window::Window {
        self.viewport.window()
    }

    pub(crate) fn scale_factor(&self) -> f64 {
        self.viewport.window().scale_factor()
    }
    pub(crate) fn destroy(&mut self) {
        self.viewport.destroy();
    }
}

fn run(
    context: &Context,
    raw_input: RawInput,
    definition: &MenuDefinition,
    title: &str,
    show_title: bool,
    highlighted_submenu: Option<usize>,
) -> ViewOutput {
    let mut output = ViewOutput::default();
    let mut hovered_item = None;
    let mut target_size = [0.0, 0.0];
    let full_output = context.run_ui(raw_input, |ctx| {
        let layout = menu_layout(ctx, definition, title, show_title);
        let frame =
            egui::Frame::menu(&ctx.style()).inner_margin(egui::Margin::same(MENU_PADDING as i8));
        let frame_margin = frame.total_margin();
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.set_min_size(Vec2::new(layout.content_width, layout.content_height));
            ui.spacing_mut().item_spacing = Vec2::ZERO;
            if show_title {
                menu_title(ui, title);
                menu_separator(ui);
            }
            for (index, item) in definition.items.iter().enumerate() {
                if item.separator_before {
                    menu_separator(ui);
                } else if index > 0 {
                    menu_gap(ui);
                }
                let response = menu_item(
                    ui,
                    item.label,
                    item.checked,
                    item.enabled,
                    item.submenu.is_some(),
                    highlighted_submenu == Some(index),
                );
                if response.hovered() {
                    hovered_item = Some(index);
                }
                if response.hovered() && item.enabled && item.submenu.is_some() {
                    output.open_submenu = Some(index);
                    output.submenu_anchor = Some([
                        layout.content_width,
                        response.rect.top(),
                        response.rect.height(),
                    ]);
                }
                if response.clicked() && item.enabled && item.submenu.is_none() {
                    output.selected_menu_item = Some(item.id.to_owned());
                    output.should_close = true;
                }
            }
            target_size = [
                layout.content_width + frame_margin.left + frame_margin.right,
                layout.content_height + frame_margin.top + frame_margin.bottom,
            ];
        });
    });
    output.resize_to = Some(target_size);
    output.full_output = full_output;
    output.hovered_item = hovered_item;
    output
}

struct MenuLayout {
    content_width: f32,
    content_height: f32,
}

fn menu_layout(
    ctx: &Context,
    definition: &MenuDefinition,
    title: &str,
    show_title: bool,
) -> MenuLayout {
    let text_width = menu_text_width(ctx, definition, show_title.then_some(title));
    let content_width =
        text_width + MENU_LEFT_ICON_WIDTH + MENU_RIGHT_ICON_WIDTH + MENU_TEXT_GAP * 2.0;
    let separator_count = definition
        .items
        .iter()
        .filter(|item| item.separator_before)
        .count();
    let item_gap_count = definition
        .items
        .iter()
        .enumerate()
        .filter(|(index, item)| *index > 0 && !item.separator_before)
        .count();
    let title_height = if show_title {
        MENU_TITLE_HEIGHT + MENU_SEPARATOR_HEIGHT
    } else {
        0.0
    };
    let content_height = title_height
        + definition.items.len() as f32 * MENU_ITEM_HEIGHT
        + separator_count as f32 * MENU_SEPARATOR_HEIGHT
        + item_gap_count as f32 * MENU_ITEM_GAP;
    MenuLayout {
        content_width,
        content_height,
    }
}

fn menu_title(ui: &mut egui::Ui, title: &str) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_TITLE_HEIGHT),
        Sense::hover(),
    );
    ui.painter().text(
        rect.left_center() + Vec2::new(MENU_LEFT_ICON_WIDTH + MENU_TEXT_GAP, 0.0),
        Align2::LEFT_CENTER,
        title,
        FontId::proportional(MENU_FONT_SIZE),
        Color32::from_rgb(150, 150, 150),
    );
}

fn menu_separator(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_SEPARATOR_HEIGHT),
        Sense::hover(),
    );
    let y = rect.center().y;
    ui.painter().line_segment(
        [egui::pos2(rect.left(), y), egui::pos2(rect.right(), y)],
        Stroke::new(1.0, Color32::from_rgb(65, 65, 65)),
    );
}

fn menu_gap(ui: &mut egui::Ui) {
    ui.allocate_space(Vec2::new(ui.available_width(), MENU_ITEM_GAP));
}

fn menu_text_width(ctx: &Context, definition: &MenuDefinition, title: Option<&str>) -> f32 {
    let font_id = FontId::proportional(MENU_FONT_SIZE);
    ctx.fonts_mut(|fonts| {
        title
            .into_iter()
            .chain(definition.items.iter().map(|item| item.label))
            .map(|label| {
                fonts
                    .layout_no_wrap(label.to_owned(), font_id.clone(), Color32::WHITE)
                    .size()
                    .x
            })
            .fold(0.0, f32::max)
    })
}

fn menu_item(
    ui: &mut egui::Ui,
    text: &str,
    checked: bool,
    enabled: bool,
    has_submenu: bool,
    highlighted: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(
        Vec2::new(ui.available_width(), MENU_ITEM_HEIGHT),
        if enabled {
            Sense::click()
        } else {
            Sense::hover()
        },
    );
    if enabled && (response.hovered() || highlighted) {
        ui.painter()
            .rect_filled(rect, 4.0, ui.visuals().widgets.hovered.bg_fill);
    }
    if checked {
        let center = rect.left_center() + Vec2::new(MENU_LEFT_ICON_WIDTH * 0.5, 0.0);
        let middle = center + Vec2::new(-1.0, 4.0);
        ui.painter().line_segment(
            [center + Vec2::new(-5.0, 0.0), middle],
            Stroke::new(1.8, Color32::from_rgb(95, 190, 120)),
        );
        ui.painter().line_segment(
            [middle, center + Vec2::new(6.0, -5.0)],
            Stroke::new(1.8, Color32::from_rgb(95, 190, 120)),
        );
    }
    ui.painter().text(
        rect.left_center() + Vec2::new(MENU_LEFT_ICON_WIDTH + MENU_TEXT_GAP, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(MENU_FONT_SIZE),
        if enabled {
            Color32::from_rgb(225, 225, 225)
        } else {
            Color32::from_rgb(110, 110, 110)
        },
    );
    if has_submenu {
        ui.painter().text(
            rect.right_center() - Vec2::new(MENU_RIGHT_ICON_WIDTH * 0.5, 0.0),
            Align2::RIGHT_CENTER,
            "›",
            FontId::proportional(18.0),
            Color32::from_rgb(190, 190, 190),
        );
    }
    response
}
