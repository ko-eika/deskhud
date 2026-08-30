//! Pure window docking geometry for four edges and four corners.

use deskhud_engine::DockState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rect {
    pub(super) left: i32,
    pub(super) top: i32,
    pub(super) right: i32,
    pub(super) bottom: i32,
}

impl Rect {
    pub(super) fn new(left: i32, top: i32, width: u32, height: u32) -> Self {
        Self {
            left,
            top,
            right: left.saturating_add(width.min(i32::MAX as u32) as i32),
            bottom: top.saturating_add(height.min(i32::MAX as u32) as i32),
        }
    }
}

pub(super) fn dock_state(window: Rect, area: Rect, tolerance: i32) -> DockState {
    let tolerance = tolerance.max(0);
    DockState {
        left: window.left <= area.left.saturating_add(tolerance),
        right: window.right >= area.right.saturating_sub(tolerance),
        top: window.top <= area.top.saturating_add(tolerance),
        bottom: window.bottom >= area.bottom.saturating_sub(tolerance),
    }
}

pub(super) fn snap_position(
    position: [i32; 2],
    size: [u32; 2],
    area: Rect,
    tolerance: i32,
) -> [i32; 2] {
    let tolerance = tolerance.max(0);
    let width = size[0].min(i32::MAX as u32) as i32;
    let height = size[1].min(i32::MAX as u32) as i32;
    let right = position[0].saturating_add(width);
    let bottom = position[1].saturating_add(height);
    [
        if position[0] <= area.left.saturating_add(tolerance) {
            area.left
        } else if right >= area.right.saturating_sub(tolerance) {
            area.right.saturating_sub(width)
        } else {
            position[0]
        },
        if position[1] <= area.top.saturating_add(tolerance) {
            area.top
        } else if bottom >= area.bottom.saturating_sub(tolerance) {
            area.bottom.saturating_sub(height)
        } else {
            position[1]
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    const AREA: Rect = Rect {
        left: -100,
        top: 20,
        right: 900,
        bottom: 820,
    };
    const SIZE: [u32; 2] = [100, 80];

    #[test]
    fn detects_each_edge_and_all_four_corners() {
        let cases = [
            (
                [-100, 200],
                DockState {
                    left: true,
                    ..DockState::FREE
                },
            ),
            (
                [800, 200],
                DockState {
                    right: true,
                    ..DockState::FREE
                },
            ),
            (
                [200, 20],
                DockState {
                    top: true,
                    ..DockState::FREE
                },
            ),
            (
                [200, 740],
                DockState {
                    bottom: true,
                    ..DockState::FREE
                },
            ),
            (
                [-100, 20],
                DockState {
                    left: true,
                    top: true,
                    ..DockState::FREE
                },
            ),
            (
                [800, 20],
                DockState {
                    right: true,
                    top: true,
                    ..DockState::FREE
                },
            ),
            (
                [-100, 740],
                DockState {
                    left: true,
                    bottom: true,
                    ..DockState::FREE
                },
            ),
            (
                [800, 740],
                DockState {
                    right: true,
                    bottom: true,
                    ..DockState::FREE
                },
            ),
        ];
        for (position, expected) in cases {
            assert_eq!(
                dock_state(
                    Rect::new(position[0], position[1], SIZE[0], SIZE[1]),
                    AREA,
                    16
                ),
                expected
            );
        }
    }

    #[test]
    fn snaps_negative_coordinate_edges_without_touching_free_axis() {
        assert_eq!(snap_position([-115, 333], SIZE, AREA, 16), [-100, 333]);
        assert_eq!(snap_position([333, 4], SIZE, AREA, 16), [333, 20]);
        assert_eq!(snap_position([-115, 4], SIZE, AREA, 16), [-100, 20]);
        assert_eq!(snap_position([333, 333], SIZE, AREA, 16), [333, 333]);
    }
}
