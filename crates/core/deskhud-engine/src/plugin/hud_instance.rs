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

impl Default for HudGroupLayout {
    fn default() -> Self {
        Self {
            arrangement: HudGroupArrangement::Horizontal,
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
}
