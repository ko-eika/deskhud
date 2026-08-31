//! 将原始按键事件提升为组合键事件。

use super::{PetEvent, PetKey, PetModifiers};

/// 引擎统一维护的键盘组合状态。
///
/// 宿主只负责把平台按键归一化为 [`PetKey`]；组合键的判定和事件形态
/// 由引擎完成，宠物包无需分别实现全局监听和窗口内监听两套逻辑。
#[derive(Debug, Clone, Copy, Default)]
pub struct PetKeyTracker {
    modifiers: PetModifiers,
}

impl PetKeyTracker {
    /// 记录一次按键按下，并在当前按键形成组合时生成第二条引擎事件。
    pub fn press(&mut self, key: PetKey, modifiers: PetModifiers) -> Option<PetEvent> {
        self.modifiers = modifiers;
        let modifier_count = [
            modifiers.ctrl,
            modifiers.shift,
            modifiers.alt,
            modifiers.meta,
        ]
        .into_iter()
        .filter(|active| *active)
        .count();
        if modifiers.any() && (!is_modifier(key) || modifier_count > 1) {
            Some(PetEvent::KeyCombinationPressed { key, modifiers })
        } else {
            None
        }
    }

    /// 记录按键释放，避免下一次输入沿用旧的组合状态。
    pub fn release(&mut self, modifiers: PetModifiers) {
        self.modifiers = modifiers;
    }

    /// 当前由宿主报告的修饰键快照。
    pub fn modifiers(&self) -> PetModifiers {
        self.modifiers
    }
}

fn is_modifier(key: PetKey) -> bool {
    matches!(
        key,
        PetKey::Ctrl | PetKey::Shift | PetKey::Alt | PetKey::Super
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn emits_incremental_combinations_after_raw_key_events() {
        let mut tracker = PetKeyTracker::default();
        let ctrl = PetModifiers {
            ctrl: true,
            ..PetModifiers::NONE
        };
        let ctrl_shift = PetModifiers {
            ctrl: true,
            shift: true,
            ..PetModifiers::NONE
        };
        assert_eq!(tracker.press(PetKey::Ctrl, ctrl), None);
        assert_eq!(
            tracker.press(PetKey::Shift, ctrl_shift),
            Some(PetEvent::KeyCombinationPressed {
                key: PetKey::Shift,
                modifiers: ctrl_shift
            })
        );
        assert_eq!(
            tracker.press(PetKey::Letter('A'), ctrl_shift),
            Some(PetEvent::KeyCombinationPressed {
                key: PetKey::Letter('A'),
                modifiers: ctrl_shift
            })
        );
    }
}
