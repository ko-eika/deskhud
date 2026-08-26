//! 菜单树控制器。
#![cfg_attr(target_os = "macos", allow(dead_code))]
//!
//! 该模块负责独立菜单窗口的生命周期、N 级子菜单路径、悬浮状态、焦点过渡和定位。

use std::time::{Duration, Instant};

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoopProxy},
    window::{Window, WindowId},
};

use super::{MenuConfig, MenuDefinition, MenuWindow, placement};
use crate::runtime::viewport::UserEvent;
use deskhud_ui::UiTheme;

/// 可复用的菜单树窗口控制器。
pub(crate) struct MenuController {
    window: MenuWindow,
    proxy: EventLoopProxy<UserEvent>,
    track_focus_loss: bool,
    submenus: Vec<MenuWindow>,
    submenu_path: Vec<usize>,
    submenu_anchors: Vec<[f32; 3]>,
    submenu_sides: Vec<placement::SubmenuSide>,
    submenu_vertical_sides: Vec<placement::SubmenuVerticalSide>,
    focus_lost_at: Option<Instant>,
    root_anchor: Option<PhysicalPosition<i32>>,
}

impl MenuController {
    /// 创建一个菜单树控制器。
    pub(crate) fn new(
        event_loop: &ActiveEventLoop,
        config: MenuConfig,
        proxy: &EventLoopProxy<UserEvent>,
    ) -> Self {
        let track_focus_loss = config.close_on_focus_loss;
        Self {
            window: MenuWindow::new(event_loop, config, proxy),
            proxy: proxy.clone(),
            track_focus_loss,
            submenus: Vec::new(),
            submenu_path: Vec::new(),
            submenu_anchors: Vec::new(),
            submenu_sides: Vec::new(),
            submenu_vertical_sides: Vec::new(),
            focus_lost_at: None,
            root_anchor: None,
        }
    }

    /// 判断原生窗口是否属于当前菜单树。
    pub(crate) fn contains_window(&self, window_id: WindowId) -> bool {
        self.window.window_id() == window_id
            || self
                .submenus
                .iter()
                .any(|submenu| submenu.window_id() == window_id)
    }

    pub(crate) fn prewarm_submenus(&mut self, event_loop: &ActiveEventLoop) {
        for depth in 0..3 {
            self.submenus.push(MenuWindow::new(
                event_loop,
                MenuConfig {
                    title: "Menu submenu",
                    egui_id: egui::ViewportId::from_hash_of(("menu-submenu", depth)),
                    focus_on_show: false,
                    ..Default::default()
                },
                &self.proxy,
            ));
        }
    }

    /// 判断根菜单是否可见。
    pub(crate) fn window_id(&self) -> WindowId {
        self.window.window_id()
    }

    pub(crate) fn window_handles(&self) -> Vec<(WindowId, std::sync::Arc<Window>)> {
        let mut handles = vec![(self.window.window_id(), self.window.window_handle())];
        handles.extend(
            self.submenus
                .iter()
                .map(|submenu| (submenu.window_id(), submenu.window_handle())),
        );
        handles
    }

    pub(crate) fn is_visible(&self) -> bool {
        self.window.is_visible()
    }

    /// 在指定屏幕位置打开根菜单。
    pub(crate) fn open(&mut self, anchor: PhysicalPosition<i32>, definition: &MenuDefinition) {
        self.root_anchor = Some(anchor);
        self.window.open(anchor, definition);
    }

    /// 关闭整棵菜单树。
    pub(crate) fn close(&mut self) {
        self.window.close();
        self.close_submenus_from(0);
        self.focus_lost_at = None;
    }

    fn close_submenus_from(&mut self, depth: usize) {
        for submenu in self.submenus.iter_mut().skip(depth) {
            submenu.close();
        }
        self.submenu_path.truncate(depth);
        self.submenu_anchors.truncate(depth);
        self.submenu_sides.truncate(depth);
        self.submenu_vertical_sides.truncate(depth);
    }

    /// 将窗口事件路由到根菜单或对应层级的子菜单。
    pub(crate) fn handle_event(&mut self, window_id: WindowId, event: &WindowEvent) {
        if let Some(depth) = self
            .submenus
            .iter()
            .position(|submenu| submenu.window_id() == window_id)
        {
            let submenu_closed = if let Some(submenu) = self.submenus.get_mut(depth) {
                submenu.handle_event(event, false);
                matches!(event, WindowEvent::CloseRequested | WindowEvent::Destroyed)
                    && !submenu.is_visible()
            } else {
                false
            };
            self.track_focus_event(event);
            if submenu_closed {
                self.close_submenus_from(depth);
            }
        } else {
            self.window.handle_event(event, false);
            self.track_focus_event(event);
        }
        if !self.window.is_visible() {
            self.close();
        }
    }

    fn track_focus_event(&mut self, event: &WindowEvent) {
        if !self.track_focus_loss {
            return;
        }
        if matches!(event, WindowEvent::Focused(false)) {
            self.focus_lost_at = Some(Instant::now());
        } else if matches!(event, WindowEvent::Focused(true)) {
            self.focus_lost_at = None;
        }
    }

    /// 驱动菜单树的一帧渲染，并返回被点击的菜单项标识和关闭请求。
    pub(crate) fn render(
        &mut self,
        definition: &MenuDefinition,
        theme: UiTheme,
    ) -> (Option<String>, bool) {
        let root_output =
            self.window
                .render(&definition, self.submenu_path.first().copied(), theme);
        if root_output.hovered_item.is_some() {
            self.focus_lost_at = None;
        }
        let mut parent_output = root_output;
        let mut selected_item = parent_output.selected_item.clone();
        let mut should_close = parent_output.viewport.should_close;
        let mut depth = 0;

        if let (Some(anchor), Some([width, height])) =
            (self.root_anchor, parent_output.viewport.resize_to)
        {
            let scale = self.window.scale_factor();
            let size = PhysicalSize::new(
                (width as f64 * scale).round().max(1.0) as u32,
                (height as f64 * scale).round().max(1.0) as u32,
            );
            let position = placement::choose_position_for_size(self.window.window(), anchor, size);
            self.window.set_outer_position(position);
        }

        let root_scale = self.window.scale_factor();
        let mut parent_position = self
            .window
            .window()
            .outer_position()
            .unwrap_or(PhysicalPosition::new(0, 0));
        let mut parent_size = self.window.outer_size();
        if let (Some(anchor), Some([width, height])) =
            (self.root_anchor, parent_output.viewport.resize_to)
        {
            let size = PhysicalSize::new(
                (width as f64 * root_scale).round().max(1.0) as u32,
                (height as f64 * root_scale).round().max(1.0) as u32,
            );
            parent_position =
                placement::choose_position_for_size(self.window.window(), anchor, size);
            parent_size = size;
        }

        loop {
            if let Some(index) = parent_output.open_submenu {
                let Some(parent_definition) =
                    definition_at(definition, &self.submenu_path[..depth])
                else {
                    self.close_submenus_from(depth);
                    break;
                };
                let valid = parent_definition
                    .items
                    .get(index)
                    .is_some_and(|item| item.enabled && item.submenu.is_some());
                if !valid {
                    self.close_submenus_from(depth);
                    break;
                }
                let path_changed = self.submenu_path.get(depth).copied() != Some(index);
                if path_changed {
                    self.close_submenus_from(depth);
                    let vertical = if depth == 0 {
                        self.root_anchor
                            .filter(|anchor| parent_position.y < anchor.y)
                            .map_or(placement::SubmenuVerticalSide::Down, |_| {
                                placement::SubmenuVerticalSide::Up
                            })
                    } else {
                        self.submenu_vertical_sides[depth - 1]
                    };
                    self.submenu_path.push(index);
                    self.submenu_anchors
                        .push(parent_output.submenu_anchor.unwrap_or([0.0, 0.0, 0.0]));
                    self.submenu_sides.push(placement::SubmenuSide::Right);
                    self.submenu_vertical_sides.push(vertical);
                } else if let Some(anchor) = parent_output.submenu_anchor {
                    self.submenu_anchors[depth] = anchor;
                }
            } else if parent_output
                .hovered_item
                .is_some_and(|index| self.submenu_path.get(depth).copied() != Some(index))
                && self.submenu_path.len() > depth
            {
                self.close_submenus_from(depth);
                break;
            } else if self.submenu_path.len() <= depth {
                break;
            }

            let Some(parent_definition) = definition_at(definition, &self.submenu_path[..depth])
            else {
                self.close_submenus_from(depth);
                break;
            };
            let Some(submenu_definition) = parent_definition
                .items
                .get(self.submenu_path[depth])
                .and_then(|item| item.submenu.as_deref())
            else {
                self.close_submenus_from(depth);
                break;
            };
            self.ensure_submenu(depth);
            let anchor = self.submenu_anchors[depth];
            let scale = self.parent_window(depth).scale_factor();
            let trigger = PhysicalPosition::new(
                parent_position.x + parent_size.width as i32,
                parent_position.y + (anchor[1] as f64 * scale).round() as i32,
            );
            let (child_output, child_size) = {
                let submenu = &mut self.submenus[depth];
                if !submenu.is_visible() {
                    submenu.open_at(trigger, submenu_definition);
                }
                let child_output = submenu.render(
                    submenu_definition,
                    self.submenu_path.get(depth + 1).copied(),
                    theme,
                );
                let child_size = child_output
                    .viewport
                    .resize_to
                    .map(|[width, height]| {
                        PhysicalSize::new(
                            (width as f64 * submenu.scale_factor()).round().max(1.0) as u32,
                            (height as f64 * submenu.scale_factor()).round().max(1.0) as u32,
                        )
                    })
                    .unwrap_or_else(|| submenu.outer_size());
                (child_output, child_size)
            };
            let trigger_height = (anchor[2] as f64 * scale).round() as i32;
            let (position, side, vertical) = if depth == 0 {
                placement::choose_submenu_position_for_parent(
                    self.parent_window(depth),
                    parent_position,
                    parent_size,
                    trigger,
                    trigger_height,
                    child_size,
                    self.submenu_vertical_sides[depth],
                )
            } else {
                let side = self.submenu_sides[depth - 1];
                let (position, vertical) =
                    placement::choose_submenu_position_on_side_for_parent_with_vertical(
                        self.parent_window(depth),
                        parent_position,
                        parent_size,
                        trigger,
                        trigger_height,
                        child_size,
                        side,
                        self.submenu_vertical_sides[depth],
                    );
                (position, side, vertical)
            };
            self.submenu_sides[depth] = side;
            self.submenu_vertical_sides[depth] = vertical;
            self.submenus[depth].set_outer_position(position);
            parent_position = position;
            parent_size = child_size;
            should_close |= child_output.viewport.should_close;
            if selected_item.is_none() {
                selected_item = child_output.selected_item.clone();
            }
            parent_output = child_output;
            if parent_output.hovered_item.is_some() {
                self.focus_lost_at = None;
            }
            depth += 1;
        }

        self.close_submenus_from(self.submenu_path.len());
        should_close |= selected_item.is_some();
        should_close |= self
            .focus_lost_at
            .is_some_and(|lost_at| lost_at.elapsed() >= Duration::from_millis(120));
        if should_close {
            self.close();
        }
        (selected_item, should_close)
    }

    fn ensure_submenu(&self, depth: usize) {
        assert!(
            depth < self.submenus.len(),
            "submenu depth exceeds prewarmed windows"
        );
    }

    fn parent_window(&self, depth: usize) -> &Window {
        if depth == 0 {
            self.window.window()
        } else {
            self.submenus[depth - 1].window()
        }
    }

    /// 销毁菜单树的全部原生窗口。
    pub(crate) fn destroy(&mut self) {
        self.window.destroy();
        for submenu in &mut self.submenus {
            submenu.destroy();
        }
    }
}

fn definition_at<'a>(root: &'a MenuDefinition, path: &[usize]) -> Option<&'a MenuDefinition> {
    let mut definition = root;
    for &index in path {
        definition = definition.items.get(index)?.submenu.as_deref()?;
    }
    Some(definition)
}
