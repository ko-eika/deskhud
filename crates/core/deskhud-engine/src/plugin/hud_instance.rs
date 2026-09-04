//! Platform-independent HUD instance and group layout contracts.

use serde::{Deserialize, Serialize};

/// Stable identity of the plugin contribution used by a HUD instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct HudSourceId {
    /// Full plugin package ID, for example `hud.deskhud.demo`.
    pub plugin_id: String,
    /// Contribution-local ID, for example `clock`.
    pub contribution_id: String,
}

impl HudSourceId {
    /// Creates a source identity without coupling it to a UI or platform type.
    pub fn new(plugin_id: impl Into<String>, contribution_id: impl Into<String>) -> Self {
        Self {
            plugin_id: plugin_id.into(),
            contribution_id: contribution_id.into(),
        }
    }

    /// Returns whether both identity components can safely be persisted.
    pub fn is_valid(&self) -> bool {
        valid_id_part(&self.plugin_id) && valid_id_part(&self.contribution_id)
    }
}

/// Stable host-owned identity of one HUD instance.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct HudInstanceId(pub String);

impl HudInstanceId {
    /// Creates an instance identity. Hosts should use their documented allocator.
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the persisted string form.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Returns whether this identity can safely be persisted and referenced.
    pub fn is_valid(&self) -> bool {
        valid_id_part(&self.0)
    }
}

/// Context supplied when asking a contribution to render one particular instance.
///
/// The context deliberately carries identities and time only. Instance configuration
/// and theme data can be added when the corresponding neutral value contract is ready.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudFrameCtx<'a> {
    /// Stable identity of the instance being rendered.
    pub instance_id: &'a HudInstanceId,
    /// Definition that owns the rendering capability.
    pub source: &'a HudSourceId,
    /// Monotonic elapsed time supplied by the host.
    pub elapsed_secs: f32,
}

/// Direction used to arrange members inside a HUD group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudGroupArrangement {
    /// Place members at host-provided logical rectangles.
    Free,
    /// Place members from left to right.
    #[default]
    Horizontal,
    /// Place members from top to bottom.
    Vertical,
    /// Place members in rows with a host-selected column count.
    Grid,
}

/// Alignment of members on the cross axis of a HUD group.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HudGroupAlignment {
    /// Align members to the leading edge.
    #[default]
    Start,
    /// Center members on the cross axis.
    Center,
    /// Align members to the trailing edge.
    End,
}

/// Platform-independent layout settings applied inside a HUD group.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HudGroupLayout {
    /// Flow direction.
    #[serde(default)]
    pub arrangement: HudGroupArrangement,
    /// Number of columns used by grid flow. Values below one normalize to one.
    #[serde(default = "default_grid_columns")]
    pub grid_columns: u16,
    /// Logical pixels between adjacent members.
    #[serde(default)]
    pub spacing: f32,
    /// Logical padding `[top, right, bottom, left]` around the group content.
    #[serde(default)]
    pub padding: [f32; 4],
    /// Cross-axis member alignment.
    #[serde(default)]
    pub alignment: HudGroupAlignment,
}

/// Finite logical size supplied by a host after measuring a HUD frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudLogicalSize {
    /// Horizontal extent in logical pixels.
    pub width: f32,
    /// Vertical extent in logical pixels.
    pub height: f32,
}

impl HudLogicalSize {
    /// Creates a bounded size suitable for neutral group composition.
    pub fn new(width: f32, height: f32) -> Self {
        Self {
            width: finite_clamp(width, 1.0, 16_384.0),
            height: finite_clamp(height, 1.0, 16_384.0),
        }
    }
}

/// Logical rectangle used to transform and clip one child frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudLogicalRect {
    /// Left edge relative to the group origin.
    pub x: f32,
    /// Top edge relative to the group origin.
    pub y: f32,
    /// Rectangle width.
    pub width: f32,
    /// Rectangle height.
    pub height: f32,
}

/// Placement produced for one child, in the same order as the measured inputs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HudGroupMemberPlacement {
    /// Child content rectangle relative to the group origin.
    pub frame: HudLogicalRect,
    /// Clip rectangle relative to the group origin.
    pub clip: HudLogicalRect,
}

/// Platform-independent result of arranging a group of measured HUD frames.
#[derive(Debug, Clone, PartialEq)]
pub struct HudGroupComposition {
    /// Natural logical size of the virtual group slot, including padding.
    pub size: HudLogicalSize,
    /// Child transforms and clips in input order.
    pub members: Vec<HudGroupMemberPlacement>,
}

impl Default for HudGroupLayout {
    fn default() -> Self {
        Self {
            arrangement: HudGroupArrangement::Free,
            grid_columns: default_grid_columns(),
            spacing: 8.0,
            padding: [8.0; 4],
            alignment: HudGroupAlignment::Start,
        }
    }
}

impl HudGroupLayout {
    /// Normalizes untrusted persisted values to finite, bounded layout inputs.
    pub fn normalized(mut self) -> Self {
        const MAX_LOGICAL_GAP: f32 = 256.0;
        self.grid_columns = self.grid_columns.clamp(1, 64);
        self.spacing = finite_clamp(self.spacing, 0.0, MAX_LOGICAL_GAP);
        self.padding = self
            .padding
            .map(|value| finite_clamp(value, 0.0, MAX_LOGICAL_GAP));
        self
    }

    /// Arranges host-measured child frames without depending on a renderer or OS type.
    pub fn compose(&self, measured: &[HudLogicalSize]) -> HudGroupComposition {
        let layout = self.clone().normalized();
        let measured: Vec<_> = measured
            .iter()
            .map(|size| HudLogicalSize::new(size.width, size.height))
            .collect();
        let [top, right, bottom, left] = layout.padding;
        if measured.is_empty() {
            return HudGroupComposition {
                size: HudLogicalSize::new(left + right, top + bottom),
                members: Vec::new(),
            };
        }

        let (content_width, content_height, origins) = match layout.arrangement {
            HudGroupArrangement::Free => {
                let origins = measured
                    .iter()
                    .enumerate()
                    .map(|(index, _)| {
                        let offset = layout.spacing * index as f32;
                        (offset, offset)
                    })
                    .collect::<Vec<_>>();
                let width = measured
                    .iter()
                    .zip(&origins)
                    .map(|(size, (x, _))| x + size.width)
                    .fold(0.0, f32::max);
                let height = measured
                    .iter()
                    .zip(&origins)
                    .map(|(size, (_, y))| y + size.height)
                    .fold(0.0, f32::max);
                (width, height, origins)
            }
            HudGroupArrangement::Horizontal => compose_horizontal(&layout, &measured),
            HudGroupArrangement::Vertical => compose_vertical(&layout, &measured),
            HudGroupArrangement::Grid => compose_grid(&layout, &measured),
        };
        let content_clip = HudLogicalRect {
            x: left,
            y: top,
            width: content_width,
            height: content_height,
        };
        let members = measured
            .iter()
            .zip(origins)
            .map(|(size, (x, y))| {
                let frame = HudLogicalRect {
                    x: left + x,
                    y: top + y,
                    width: size.width,
                    height: size.height,
                };
                HudGroupMemberPlacement {
                    frame,
                    clip: intersect_rect(frame, content_clip),
                }
            })
            .collect();
        HudGroupComposition {
            size: HudLogicalSize::new(left + content_width + right, top + content_height + bottom),
            members,
        }
    }

    /// Composes freely positioned child rectangles supplied by the host.
    pub fn compose_free(&self, frames: &[HudLogicalRect]) -> HudGroupComposition {
        let layout = self.clone().normalized();
        let [top, right, bottom, left] = layout.padding;
        let frames = frames
            .iter()
            .map(|frame| HudLogicalRect {
                x: finite_clamp(frame.x, 0.0, 16_384.0),
                y: finite_clamp(frame.y, 0.0, 16_384.0),
                width: finite_clamp(frame.width, 1.0, 16_384.0),
                height: finite_clamp(frame.height, 1.0, 16_384.0),
            })
            .collect::<Vec<_>>();
        let content_width = frames
            .iter()
            .map(|frame| frame.x + frame.width)
            .fold(1.0, f32::max);
        let content_height = frames
            .iter()
            .map(|frame| frame.y + frame.height)
            .fold(1.0, f32::max);
        let content_clip = HudLogicalRect {
            x: left,
            y: top,
            width: content_width,
            height: content_height,
        };
        let members = frames
            .into_iter()
            .map(|frame| {
                let frame = HudLogicalRect {
                    x: left + frame.x,
                    y: top + frame.y,
                    ..frame
                };
                HudGroupMemberPlacement {
                    frame,
                    clip: intersect_rect(frame, content_clip),
                }
            })
            .collect();
        HudGroupComposition {
            size: HudLogicalSize::new(left + content_width + right, top + content_height + bottom),
            members,
        }
    }
}

fn intersect_rect(left: HudLogicalRect, right: HudLogicalRect) -> HudLogicalRect {
    let x = left.x.max(right.x);
    let y = left.y.max(right.y);
    let right_edge = (left.x + left.width).min(right.x + right.width);
    let bottom_edge = (left.y + left.height).min(right.y + right.height);
    HudLogicalRect {
        x,
        y,
        width: (right_edge - x).max(0.0),
        height: (bottom_edge - y).max(0.0),
    }
}

fn align_offset(available: f32, used: f32, alignment: HudGroupAlignment) -> f32 {
    let remaining = (available - used).max(0.0);
    match alignment {
        HudGroupAlignment::Start => 0.0,
        HudGroupAlignment::Center => remaining * 0.5,
        HudGroupAlignment::End => remaining,
    }
}

fn compose_horizontal(
    layout: &HudGroupLayout,
    measured: &[HudLogicalSize],
) -> (f32, f32, Vec<(f32, f32)>) {
    let height = measured.iter().map(|size| size.height).fold(0.0, f32::max);
    let width = measured.iter().map(|size| size.width).sum::<f32>()
        + layout.spacing * measured.len().saturating_sub(1) as f32;
    let mut x = 0.0;
    let origins = measured
        .iter()
        .map(|size| {
            let origin = (x, align_offset(height, size.height, layout.alignment));
            x += size.width + layout.spacing;
            origin
        })
        .collect();
    (width, height, origins)
}

fn compose_vertical(
    layout: &HudGroupLayout,
    measured: &[HudLogicalSize],
) -> (f32, f32, Vec<(f32, f32)>) {
    let width = measured.iter().map(|size| size.width).fold(0.0, f32::max);
    let height = measured.iter().map(|size| size.height).sum::<f32>()
        + layout.spacing * measured.len().saturating_sub(1) as f32;
    let mut y = 0.0;
    let origins = measured
        .iter()
        .map(|size| {
            let origin = (align_offset(width, size.width, layout.alignment), y);
            y += size.height + layout.spacing;
            origin
        })
        .collect();
    (width, height, origins)
}

fn compose_grid(
    layout: &HudGroupLayout,
    measured: &[HudLogicalSize],
) -> (f32, f32, Vec<(f32, f32)>) {
    let columns = usize::from(layout.grid_columns).min(measured.len()).max(1);
    let rows = measured.len().div_ceil(columns);
    let mut column_widths = vec![0.0_f32; columns];
    let mut row_heights = vec![0.0_f32; rows];
    for (index, size) in measured.iter().enumerate() {
        column_widths[index % columns] = column_widths[index % columns].max(size.width);
        row_heights[index / columns] = row_heights[index / columns].max(size.height);
    }
    let width =
        column_widths.iter().sum::<f32>() + layout.spacing * columns.saturating_sub(1) as f32;
    let height = row_heights.iter().sum::<f32>() + layout.spacing * rows.saturating_sub(1) as f32;
    let mut column_x = vec![0.0_f32; columns];
    let mut row_y = vec![0.0_f32; rows];
    for index in 1..columns {
        column_x[index] = column_x[index - 1] + column_widths[index - 1] + layout.spacing;
    }
    for index in 1..rows {
        row_y[index] = row_y[index - 1] + row_heights[index - 1] + layout.spacing;
    }
    let origins = measured
        .iter()
        .enumerate()
        .map(|(index, size)| {
            let column = index % columns;
            let row = index / columns;
            (
                column_x[column]
                    + align_offset(column_widths[column], size.width, layout.alignment),
                row_y[row] + align_offset(row_heights[row], size.height, layout.alignment),
            )
        })
        .collect();
    (width, height, origins)
}

fn default_grid_columns() -> u16 {
    2
}

fn finite_clamp(value: f32, min: f32, max: f32) -> f32 {
    if value.is_finite() {
        value.clamp(min, max)
    } else {
        min
    }
}

fn valid_id_part(value: &str) -> bool {
    !value.is_empty() && value.len() <= 512 && !value.chars().any(char::is_control)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn layout_normalization_bounds_untrusted_numbers() {
        let layout = HudGroupLayout {
            grid_columns: 0,
            spacing: f32::NAN,
            padding: [-1.0, 4.0, f32::INFINITY, 999.0],
            ..HudGroupLayout::default()
        }
        .normalized();
        assert_eq!(layout.grid_columns, 1);
        assert_eq!(layout.spacing, 0.0);
        assert_eq!(layout.padding, [0.0, 4.0, 0.0, 256.0]);
    }

    #[test]
    fn identities_reject_empty_and_control_text() {
        assert!(HudSourceId::new("hud.deskhud.demo", "clock").is_valid());
        assert!(!HudSourceId::new("", "clock").is_valid());
        assert!(!HudInstanceId::new("instance:\n1").is_valid());
    }

    #[test]
    fn horizontal_composition_applies_padding_spacing_and_alignment() {
        let layout = HudGroupLayout {
            arrangement: HudGroupArrangement::Horizontal,
            spacing: 5.0,
            padding: [1.0, 2.0, 3.0, 4.0],
            alignment: HudGroupAlignment::End,
            ..HudGroupLayout::default()
        };
        let result = layout.compose(&[
            HudLogicalSize::new(10.0, 10.0),
            HudLogicalSize::new(20.0, 20.0),
        ]);
        assert_eq!(result.size, HudLogicalSize::new(41.0, 24.0));
        assert_eq!(result.members[0].frame.x, 4.0);
        assert_eq!(result.members[0].frame.y, 11.0);
        assert_eq!(result.members[1].frame.x, 19.0);
        assert_eq!(result.members[0].clip, result.members[0].frame);
    }

    #[test]
    fn grid_composition_uses_stable_rows_and_columns() {
        let layout = HudGroupLayout {
            arrangement: HudGroupArrangement::Grid,
            grid_columns: 2,
            spacing: 2.0,
            padding: [0.0; 4],
            alignment: HudGroupAlignment::Center,
        };
        let result = layout.compose(&[
            HudLogicalSize::new(10.0, 10.0),
            HudLogicalSize::new(20.0, 20.0),
            HudLogicalSize::new(30.0, 5.0),
        ]);
        assert_eq!(result.size, HudLogicalSize::new(52.0, 27.0));
        assert_eq!(result.members[0].frame.x, 10.0);
        assert_eq!(result.members[2].frame.x, 0.0);
        assert_eq!(result.members[2].frame.y, 22.0);
    }

    #[test]
    fn free_composition_uses_host_rectangles_and_padding() {
        let layout = HudGroupLayout {
            arrangement: HudGroupArrangement::Free,
            padding: [2.0, 3.0, 4.0, 5.0],
            ..HudGroupLayout::default()
        };
        let result = layout.compose_free(&[
            HudLogicalRect {
                x: 10.0,
                y: 20.0,
                width: 30.0,
                height: 40.0,
            },
            HudLogicalRect {
                x: 50.0,
                y: 5.0,
                width: 20.0,
                height: 10.0,
            },
        ]);
        assert_eq!(result.size, HudLogicalSize::new(78.0, 66.0));
        assert_eq!(result.members[0].frame.x, 15.0);
        assert_eq!(result.members[0].frame.y, 22.0);
    }
}
