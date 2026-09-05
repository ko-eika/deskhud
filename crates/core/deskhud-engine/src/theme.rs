//! Renderer-neutral semantic colors shared by host-owned surfaces.

use crate::OverlayColor;

/// Semantic colors resolved by the host for the active application theme.
///
/// Packages and renderers consume roles instead of depending on egui, a
/// platform theme API, or hard-coded light/dark color values. The host may
/// derive these values from any UI toolkit and can extend this palette without
/// changing package contracts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ThemePalette {
    /// Primary accent used for selection and active controls.
    pub accent: OverlayColor,
    /// Accent color for hovered controls.
    pub accent_hover: OverlayColor,
    /// Accent color for pressed controls.
    pub accent_active: OverlayColor,
    /// Base application background.
    pub background: OverlayColor,
    /// Raised surface/card background.
    pub surface: OverlayColor,
    /// Secondary surface used for inputs and inactive controls.
    pub surface_alt: OverlayColor,
    /// Recessed background used by text fields, selectors and switches.
    pub control: OverlayColor,
    /// Surface used while hovering a control.
    pub surface_hover: OverlayColor,
    /// Surface used while pressing or activating a control.
    pub surface_active: OverlayColor,
    /// Default border and divider color.
    pub border: OverlayColor,
    /// Low-emphasis divider color.
    pub divider: OverlayColor,
    /// Focus-ring color.
    pub focus: OverlayColor,
    /// Primary readable text color.
    pub text: OverlayColor,
    /// Secondary text and metadata color.
    pub muted_text: OverlayColor,
    /// Disabled text/control color.
    pub disabled_text: OverlayColor,
    /// Selection highlight color.
    pub selection: OverlayColor,
    /// Text drawn on an accent-colored surface.
    pub text_on_accent: OverlayColor,
    /// Text drawn on a selection surface.
    pub selection_text: OverlayColor,
    /// Informational status color.
    pub info: OverlayColor,
    /// Successful status color.
    pub success: OverlayColor,
    /// Warning status color.
    pub warning: OverlayColor,
    /// Error status color.
    pub danger: OverlayColor,
    /// Default shadow color.
    pub shadow: OverlayColor,
}

impl Default for ThemePalette {
    fn default() -> Self {
        Self::dark()
    }
}

impl ThemePalette {
    /// Returns the bundled dark application palette.
    pub const fn dark() -> Self {
        Self {
            accent: rgba(42, 119, 224, 255),
            accent_hover: rgba(60, 138, 235, 255),
            accent_active: rgba(35, 98, 190, 255),
            background: rgba(26, 28, 33, 255),
            surface: rgba(30, 33, 38, 255),
            surface_alt: rgba(49, 53, 61, 255),
            control: rgba(38, 41, 48, 255),
            surface_hover: rgba(60, 68, 82, 255),
            surface_active: rgba(41, 45, 52, 255),
            border: rgba(67, 72, 82, 255),
            divider: rgba(67, 72, 82, 180),
            focus: rgba(92, 160, 255, 255),
            text: rgba(232, 235, 241, 255),
            muted_text: rgba(164, 171, 182, 255),
            disabled_text: rgba(112, 118, 130, 255),
            selection: rgba(42, 119, 224, 255),
            text_on_accent: rgba(255, 255, 255, 255),
            selection_text: rgba(255, 255, 255, 255),
            info: rgba(92, 160, 255, 255),
            success: rgba(78, 190, 125, 255),
            warning: rgba(235, 178, 75, 255),
            danger: rgba(225, 92, 92, 255),
            shadow: rgba(0, 0, 0, 180),
        }
    }

    /// Returns the bundled light application palette.
    pub const fn light() -> Self {
        Self {
            accent: rgba(126, 194, 244, 255),
            accent_hover: rgba(145, 205, 248, 255),
            accent_active: rgba(105, 174, 228, 255),
            // Keep the light theme's depth direction consistent with the
            // dark theme: cards are raised from the page, while controls are
            // recessed inside those cards.
            background: rgba(244, 246, 249, 255),
            surface: rgba(251, 252, 254, 255),
            surface_alt: rgba(248, 250, 252, 255),
            control: rgba(239, 242, 246, 255),
            surface_hover: rgba(240, 245, 251, 255),
            surface_active: rgba(235, 239, 245, 255),
            border: rgba(218, 223, 231, 255),
            divider: rgba(218, 223, 231, 150),
            focus: rgba(25, 109, 180, 255),
            text: rgba(42, 46, 54, 255),
            muted_text: rgba(102, 109, 121, 255),
            disabled_text: rgba(145, 151, 161, 255),
            selection: rgba(126, 194, 244, 255),
            text_on_accent: rgba(25, 77, 119, 255),
            selection_text: rgba(25, 77, 119, 255),
            info: rgba(25, 109, 180, 255),
            success: rgba(37, 145, 87, 255),
            warning: rgba(174, 112, 20, 255),
            danger: rgba(190, 55, 55, 255),
            // Light-theme overlays use a soft light tint rather than the
            // dark shadow used by the dark palette.
            shadow: rgba(238, 242, 247, 100),
        }
    }
}

const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> OverlayColor {
    OverlayColor {
        red,
        green,
        blue,
        alpha,
    }
}
