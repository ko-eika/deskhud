use winit::{dpi::PhysicalPosition, window::Window};

use super::ActivityArea;

pub(super) fn get(window: &Window) -> Option<ActivityArea> {
    super::fallback(window)
}

pub(super) fn get_at(window: &Window, _position: PhysicalPosition<i32>) -> Option<ActivityArea> {
    super::fallback(window)
}
