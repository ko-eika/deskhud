//! 中性鼠标按键（无平台虚拟码）。

/// 宠物可感知的鼠标键。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetMouseButton {
    /// 左键 / 主按钮。
    Primary,
    /// 右键 / 次按钮。
    Secondary,
    /// 中键。
    Middle,
}

impl PetMouseButton {
    /// Returns the stable PO key used by the host when displaying this button.
    pub fn i18n_key(self) -> &'static str {
        match self {
            Self::Primary => "InputKeyPrimary",
            Self::Secondary => "InputKeySecondary",
            Self::Middle => "InputKeyMiddle",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::PetMouseButton;

    #[test]
    fn exposes_stable_translation_keys() {
        assert_eq!(PetMouseButton::Primary.i18n_key(), "InputKeyPrimary");
        assert_eq!(PetMouseButton::Secondary.i18n_key(), "InputKeySecondary");
        assert_eq!(PetMouseButton::Middle.i18n_key(), "InputKeyMiddle");
    }
}
