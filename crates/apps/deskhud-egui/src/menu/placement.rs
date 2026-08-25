//! 右键菜单的屏幕位置计算。

use winit::{
    dpi::{PhysicalPosition, PhysicalSize},
    window::Window,
};

use crate::area;

const SUBMENU_OVERLAP: i32 = 10;

/// 子菜单相对于父菜单的弹出方向。
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmenuSide {
    /// 子菜单显示在父菜单左侧。
    Left,
    /// 子菜单显示在父菜单右侧。
    Right,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum SubmenuVerticalSide {
    /// 子菜单向下展开。
    Down,
    /// 子菜单向上展开。
    Up,
}

/// 根据右击位置选择菜单的最佳左上角坐标。
pub(crate) fn choose_position(
    window: &Window,
    anchor: PhysicalPosition<i32>,
) -> PhysicalPosition<i32> {
    choose_position_for_size(window, anchor, window.outer_size())
}

/// 使用即将生效的尺寸计算菜单位置，避免窗口先按默认尺寸定位、调整尺寸后又越过屏幕边界。
pub(crate) fn choose_position_for_size(
    window: &Window,
    anchor: PhysicalPosition<i32>,
    size: PhysicalSize<u32>,
) -> PhysicalPosition<i32> {
    let Some(active_area) = area::get_at(window, anchor) else {
        return anchor;
    };
    let area_position = active_area.position;
    let area_size = active_area.size;
    let max_x = (area_position.x + area_size.width as i32 - size.width as i32).max(area_position.x);
    let max_y =
        (area_position.y + area_size.height as i32 - size.height as i32).max(area_position.y);
    let right = anchor.x;
    let left = anchor.x - size.width as i32;
    let x = if right + size.width as i32 <= area_position.x + area_size.width as i32 {
        right
    } else {
        left
    };
    let below = anchor.y;
    let above = anchor.y - size.height as i32;
    let y = if below + size.height as i32 <= area_position.y + area_size.height as i32 {
        below
    } else {
        above
    };
    PhysicalPosition::new(
        x.clamp(area_position.x, max_x),
        y.clamp(area_position.y, max_y),
    )
}

pub(crate) fn choose_submenu_position_for_parent(
    parent: &Window,
    parent_position: PhysicalPosition<i32>,
    parent_size: PhysicalSize<u32>,
    anchor: PhysicalPosition<i32>,
    trigger_height: i32,
    submenu_size: PhysicalSize<u32>,
    preferred_vertical: SubmenuVerticalSide,
) -> (PhysicalPosition<i32>, SubmenuSide, SubmenuVerticalSide) {
    let Some(active_area) = area::get_at(parent, anchor) else {
        return (anchor, SubmenuSide::Right, preferred_vertical);
    };
    let area_position = active_area.position;
    let area_size = active_area.size;
    let width = submenu_size.width as i32;
    let parent_width = parent_size.width as i32;
    let right = parent_position.x + parent_width - SUBMENU_OVERLAP;
    let left = parent_position.x - width + SUBMENU_OVERLAP;
    let right_fits = right + width <= area_position.x + area_size.width as i32;
    let left_fits = left >= area_position.x;
    let side = if right_fits || !left_fits {
        SubmenuSide::Right
    } else {
        SubmenuSide::Left
    };
    let (position, vertical) = position_on_side_for_parent(
        parent,
        parent_position,
        parent_size,
        anchor,
        trigger_height,
        submenu_size,
        side,
        preferred_vertical,
    );
    (position, side, vertical)
}

pub(crate) fn choose_submenu_position_on_side_for_parent_with_vertical(
    parent: &Window,
    parent_position: PhysicalPosition<i32>,
    parent_size: PhysicalSize<u32>,
    anchor: PhysicalPosition<i32>,
    trigger_height: i32,
    submenu_size: PhysicalSize<u32>,
    side: SubmenuSide,
    preferred_vertical: SubmenuVerticalSide,
) -> (PhysicalPosition<i32>, SubmenuVerticalSide) {
    position_on_side_for_parent(
        parent,
        parent_position,
        parent_size,
        anchor,
        trigger_height,
        submenu_size,
        side,
        preferred_vertical,
    )
}

fn position_on_side_for_parent(
    parent: &Window,
    parent_position: PhysicalPosition<i32>,
    parent_size: PhysicalSize<u32>,
    anchor: PhysicalPosition<i32>,
    trigger_height: i32,
    submenu_size: PhysicalSize<u32>,
    side: SubmenuSide,
    preferred_vertical: SubmenuVerticalSide,
) -> (PhysicalPosition<i32>, SubmenuVerticalSide) {
    let Some(active_area) = area::get_at(parent, anchor) else {
        return (anchor, preferred_vertical);
    };
    let area_position = active_area.position;
    let area_size = active_area.size;
    let width = submenu_size.width as i32;
    let height = submenu_size.height as i32;
    let parent_width = parent_size.width as i32;
    let horizontal = match side {
        SubmenuSide::Left => parent_position.x - width + SUBMENU_OVERLAP,
        SubmenuSide::Right => parent_position.x + parent_width - SUBMENU_OVERLAP,
    };
    let area_right = area_position.x + area_size.width as i32;
    let area_bottom = area_position.y + area_size.height as i32;
    let max_x = (area_right - width).max(area_position.x);
    let max_y = (area_bottom - height).max(area_position.y);
    let top = anchor.y;
    let bottom = anchor.y + trigger_height - height;
    let (y, vertical) = match preferred_vertical {
        SubmenuVerticalSide::Down if top + height <= area_bottom => {
            (top, SubmenuVerticalSide::Down)
        }
        SubmenuVerticalSide::Up if bottom >= area_position.y => (bottom, SubmenuVerticalSide::Up),
        SubmenuVerticalSide::Down if bottom >= area_position.y => (bottom, SubmenuVerticalSide::Up),
        SubmenuVerticalSide::Up if top + height <= area_bottom => (top, SubmenuVerticalSide::Down),
        _ => (max_y, preferred_vertical),
    };
    (
        PhysicalPosition::new(
            horizontal.clamp(area_position.x, max_x),
            y.clamp(area_position.y, max_y),
        ),
        vertical,
    )
}
