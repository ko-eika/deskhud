//! Platform-neutral settings state and commands.

use crate::UiPreferences;

/// Settings pages exposed by the shell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SettingsTab {
    /// General shell preferences.
    #[default]
    General,
    /// Performance preferences.
    Performance,
    /// Pet selection and configuration.
    Pet,
    /// HUD and plugin preferences.
    Hud,
    /// Product information.
    About,
}

impl SettingsTab {
    /// Stable navigation order shared by native and legacy views.
    pub const ALL: [Self; 5] = [
        Self::General,
        Self::Performance,
        Self::Pet,
        Self::Hud,
        Self::About,
    ];

    /// Stable i18n key for the navigation label.
    pub const fn nav_message(self) -> crate::MessageKey {
        match self {
            Self::General => crate::MessageKey::SettingsNavGeneral,
            Self::Performance => crate::MessageKey::SettingsNavPerformance,
            Self::Pet => crate::MessageKey::SettingsNavPet,
            Self::Hud => crate::MessageKey::SettingsNavHud,
            Self::About => crate::MessageKey::SettingsNavAbout,
        }
    }
}

/// User actions emitted by a settings view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsCommand {
    /// Apply the current draft and close the view.
    Apply,
    /// Close and restore the last applied draft.
    Cancel,
    /// Apply without closing.
    ApplyKeepOpen,
    /// Reset the draft to the last applied values.
    Reset,
    /// Navigate to a page.
    Navigate(SettingsTab),
}

/// Result of processing a settings command.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsEffect {
    /// Updated preferences to publish, if any.
    pub preferences: Option<UiPreferences>,
    /// Whether the settings view should close.
    pub close: bool,
    /// Whether an unapplied draft should be discarded.
    pub discard: bool,
}

/// Toolkit-independent pet card grid geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PetCardLayout {
    /// Number of columns.
    pub columns: usize,
    /// Card width.
    pub card_width: f32,
    /// Card height.
    pub card_height: f32,
    /// Preview square side length.
    pub preview_side: f32,
}

/// Product information displayed by an about page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AboutInfo {
    /// Product version.
    pub version: String,
    /// Product authors.
    pub authors: String,
    /// License identifier.
    pub license: String,
    /// Technology stack summary.
    pub stack: String,
    /// Project homepage.
    pub homepage: String,
}

/// Calculates pet picker card geometry without UI toolkit types.
pub fn pet_card_layout(available_width: f32) -> PetCardLayout {
    let gap = 12.0;
    let min = 156.0;
    let max = 280.0;
    let pad = 12.0;
    let text = 78.0;
    let avail = available_width.max(min);
    let mut columns = 1;
    for count in 2..=5 {
        if (avail - gap * (count - 1) as f32) / (count as f32) < min {
            break;
        }
        columns = count;
    }
    let raw = (avail - gap * (columns - 1) as f32) / columns as f32;
    let card_width = if columns == 1 {
        raw.clamp(min, max)
    } else {
        raw.max(min)
    };
    let preview_side = (card_width - 2.0 - pad * 2.0).max(96.0);
    let card_height = 2.0 + pad + preview_side + pad + text + pad;
    PetCardLayout {
        columns,
        card_width,
        card_height,
        preview_side,
    }
}

/// Platform-neutral settings draft and navigation state.
#[derive(Debug, Clone, PartialEq)]
pub struct SettingsModel {
    /// Whether the settings view is open.
    pub open: bool,
    /// Current draft values.
    pub draft: UiPreferences,
    /// Values restored by reset/cancel.
    pub baseline: UiPreferences,
    /// Current page.
    pub tab: SettingsTab,
}

impl SettingsModel {
    /// Creates a closed model using the supplied preferences.
    pub fn new(prefs: UiPreferences) -> Self {
        Self {
            open: false,
            draft: prefs.clone(),
            baseline: prefs,
            tab: SettingsTab::default(),
        }
    }

    /// Opens the model with a fresh draft.
    pub fn open(&mut self, prefs: &UiPreferences) {
        self.draft = prefs.clone();
        self.baseline = prefs.clone();
        self.tab = SettingsTab::default();
        self.open = true;
    }

    /// Whether the draft contains an applicable preference change.
    pub fn is_dirty(&self) -> bool {
        draft_is_dirty(&self.draft, &self.baseline)
    }

    /// Resets editable settings while preserving view-only preferences and window geometry.
    pub fn reset_draft(&mut self) {
        let picker_mode = self.draft.pet.picker_mode;
        let geometry = (
            self.draft.shell.settings_width,
            self.draft.shell.settings_height,
            self.draft.shell.settings_pos_x,
            self.draft.shell.settings_pos_y,
        );
        self.draft = self.baseline.clone();
        self.draft.pet.picker_mode = picker_mode;
        self.draft.shell.settings_width = geometry.0;
        self.draft.shell.settings_height = geometry.1;
        self.draft.shell.settings_pos_x = geometry.2;
        self.draft.shell.settings_pos_y = geometry.3;
    }

    /// Applies a command and returns new preferences when the caller must commit them.
    pub fn command(&mut self, command: SettingsCommand) -> Option<UiPreferences> {
        self.command_effect(command).preferences
    }

    /// Processes a command and reports lifecycle effects for the host.
    pub fn command_effect(&mut self, command: SettingsCommand) -> SettingsEffect {
        match command {
            SettingsCommand::Apply => {
                self.baseline = self.draft.clone();
                self.open = false;
                SettingsEffect {
                    preferences: Some(self.draft.clone()),
                    close: true,
                    discard: false,
                }
            }
            SettingsCommand::ApplyKeepOpen => {
                self.baseline = self.draft.clone();
                SettingsEffect {
                    preferences: Some(self.draft.clone()),
                    close: false,
                    discard: false,
                }
            }
            SettingsCommand::Cancel => {
                self.draft = self.baseline.clone();
                self.open = false;
                SettingsEffect {
                    preferences: None,
                    close: true,
                    discard: true,
                }
            }
            SettingsCommand::Reset => {
                self.reset_draft();
                SettingsEffect {
                    preferences: None,
                    close: false,
                    discard: false,
                }
            }
            SettingsCommand::Navigate(tab) => {
                self.tab = tab;
                SettingsEffect {
                    preferences: None,
                    close: false,
                    discard: false,
                }
            }
        }
    }
}

/// Compares preference values while ignoring settings-window geometry.
pub fn draft_is_dirty(draft: &UiPreferences, baseline: &UiPreferences) -> bool {
    let mut draft = draft.clone();
    let mut baseline = baseline.clone();
    for prefs in [&mut draft, &mut baseline] {
        prefs.shell.settings_width = None;
        prefs.shell.settings_height = None;
        prefs.shell.settings_pos_x = None;
        prefs.shell.settings_pos_y = None;
    }
    draft != baseline
}

/// Applies the platform-neutral general settings form to a preference draft.
pub fn apply_general_preferences(
    prefs: &mut UiPreferences,
    locale: crate::Locale,
    theme: crate::UiTheme,
    font_id: String,
    font_family: String,
    font_style: String,
    font_size: f32,
) {
    prefs.locale = locale;
    prefs.shell.ui_theme = theme;
    prefs.shell.ui_font_id = font_id;
    prefs.shell.ui_font_family = font_family;
    prefs.shell.ui_font_style = font_style;
    prefs.shell.ui_font_size = font_size;
}

/// Applies the platform-neutral performance form to a preference draft.
pub fn apply_graphics_preferences(
    prefs: &mut UiPreferences,
    fps_limit: crate::FpsLimit,
    animation_quality: crate::AnimationQuality,
    effects: bool,
    power_mode: crate::PowerMode,
) {
    prefs.graphics.fps_limit = fps_limit;
    prefs.graphics.animation_quality = animation_quality;
    prefs.graphics.effects = effects;
    prefs.graphics.power_mode = power_mode;
}

/// Applies the platform-neutral pet picker selection to a preference draft.
pub fn apply_pet_selection(prefs: &mut UiPreferences, pet_id: String, mode: crate::PetPickerMode) {
    prefs.pet.kind = pet_id;
    prefs.pet.picker_mode = mode;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn general_preferences_are_applied_by_the_common_model() {
        let mut prefs = UiPreferences::default();
        apply_general_preferences(
            &mut prefs,
            crate::Locale::En,
            crate::UiTheme::Light,
            "Inter#face=1".into(),
            "inter".into(),
            "Bold".into(),
            16.0,
        );
        assert_eq!(prefs.locale, crate::Locale::En);
        assert_eq!(prefs.shell.ui_theme, crate::UiTheme::Light);
        assert_eq!(prefs.shell.ui_font_id, "Inter#face=1");
        assert_eq!(prefs.shell.ui_font_style, "Bold");
        assert_eq!(prefs.shell.ui_font_size, 16.0);
    }

    #[test]
    fn graphics_preferences_are_applied_by_the_common_model() {
        let mut prefs = UiPreferences::default();
        apply_graphics_preferences(
            &mut prefs,
            crate::FpsLimit::Fps60,
            crate::AnimationQuality::High,
            false,
            crate::PowerMode::Smooth,
        );
        assert_eq!(prefs.graphics.fps_limit, crate::FpsLimit::Fps60);
        assert_eq!(
            prefs.graphics.animation_quality,
            crate::AnimationQuality::High
        );
        assert!(!prefs.graphics.effects);
        assert_eq!(prefs.graphics.power_mode, crate::PowerMode::Smooth);
    }

    #[test]
    fn geometry_does_not_make_draft_dirty() {
        let prefs = UiPreferences::default();
        let mut model = SettingsModel::new(prefs.clone());
        model
            .draft
            .shell
            .set_settings_geometry(900.0, 600.0, 10.0, 20.0);
        assert!(!model.is_dirty());
    }

    #[test]
    fn apply_and_cancel_have_distinct_commit_semantics() {
        let mut model = SettingsModel::new(UiPreferences::default());
        model.open = true;
        model.draft.shell.topmost = false;
        assert!(model.command(SettingsCommand::Cancel).is_none());
        assert!(model.draft.shell.topmost);
        model.draft.shell.topmost = false;
        let committed = model
            .command(SettingsCommand::Apply)
            .expect("apply commits");
        assert!(!committed.shell.topmost);
        assert!(!model.open);
    }

    #[test]
    fn reset_preserves_picker_mode_and_geometry() {
        let mut model = SettingsModel::new(UiPreferences::default());
        model.draft.pet.picker_mode = crate::PetPickerMode::List;
        model
            .draft
            .shell
            .set_settings_geometry(900.0, 600.0, 10.0, 20.0);
        model.draft.shell.topmost = false;
        model.reset_draft();
        assert_eq!(model.draft.pet.picker_mode, crate::PetPickerMode::List);
        assert_eq!(model.draft.shell.settings_pos(), Some([10.0, 20.0]));
        assert!(model.draft.shell.topmost);
    }

    #[test]
    fn pet_card_layout_has_minimums() {
        let layout = pet_card_layout(900.0);
        assert!(layout.columns >= 1);
        assert!(layout.card_width >= 156.0);
        assert!(layout.preview_side >= 96.0);
    }

    #[test]
    fn command_effect_describes_apply_and_cancel_lifecycle() {
        let mut model = SettingsModel::new(UiPreferences::default());
        model.open = true;
        let apply = model.command_effect(SettingsCommand::Apply);
        assert!(apply.close && !apply.discard && apply.preferences.is_some());
        model.open = true;
        let cancel = model.command_effect(SettingsCommand::Cancel);
        assert!(cancel.close && cancel.discard && cancel.preferences.is_none());
    }
}
