//! 中性按键（宠窗获焦转发 / 全局子集采样）。

/// 宠物可感知的按键；未列出的键不转发。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PetKey {
    /// Escape。
    Escape,
    /// Tab。
    Tab,
    /// Enter / Return。
    Enter,
    /// Space。
    Space,
    /// Backspace。
    Backspace,
    /// Delete。
    Delete,
    /// Insert（含 NumLock 关闭时的小键盘 0）。
    Insert,
    /// Clear（常见于 NumLock 关闭时的小键盘 5）。
    Clear,
    /// 方向上。
    ArrowUp,
    /// 方向下。
    ArrowDown,
    /// 方向左。
    ArrowLeft,
    /// 方向右。
    ArrowRight,
    /// Home。
    Home,
    /// End。
    End,
    /// PageUp。
    PageUp,
    /// PageDown。
    PageDown,
    /// PrintScreen。
    PrintScreen,
    /// ScrollLock。
    ScrollLock,
    /// Pause/Break。
    Pause,
    /// Context menu key。
    ContextMenu,
    /// 左/右 Shift（合并为一种）。
    Shift,
    /// 左/右 Ctrl。
    Ctrl,
    /// 左/右 Alt。
    Alt,
    /// Win / Meta。
    Super,
    /// CapsLock。
    CapsLock,
    /// NumLock。
    NumLock,
    /// 小键盘 Enter。
    NumpadEnter,
    /// 小键盘数字 0..=9。
    NumpadDigit(u8),
    /// 小键盘加号。
    NumpadAdd,
    /// 小键盘减号。
    NumpadSubtract,
    /// 小键盘乘号。
    NumpadMultiply,
    /// 小键盘除号。
    NumpadDivide,
    /// 小键盘小数点。
    NumpadDecimal,
    /// 小键盘分隔符（取决于键盘布局）。
    NumpadSeparator,
    /// F1..=F12。
    Function(u8),
    /// 字母 A..=Z（统一大写）。
    Letter(char),
    /// 数字 '0'..='9'。
    Digit(char),
    /// 标点（按键主铭文，US 布局未按下 Shift 时的字符）。
    Punct(char),
}

impl PetKey {
    /// Returns the stable PO key used by the host when displaying this key.
    pub fn i18n_key(self) -> String {
        match self {
            Self::Escape => "InputKey.Escape".into(),
            Self::Tab => "InputKey.Tab".into(),
            Self::Enter => "InputKey.Enter".into(),
            Self::Space => "InputKey.Space".into(),
            Self::Backspace => "InputKey.Backspace".into(),
            Self::Delete => "InputKey.Delete".into(),
            Self::Insert => "InputKey.Insert".into(),
            Self::Clear => "InputKey.Clear".into(),
            Self::ArrowUp => "InputKey.ArrowUp".into(),
            Self::ArrowDown => "InputKey.ArrowDown".into(),
            Self::ArrowLeft => "InputKey.ArrowLeft".into(),
            Self::ArrowRight => "InputKey.ArrowRight".into(),
            Self::Home => "InputKey.Home".into(),
            Self::End => "InputKey.End".into(),
            Self::PageUp => "InputKey.PageUp".into(),
            Self::PageDown => "InputKey.PageDown".into(),
            Self::Shift => "InputKey.Shift".into(),
            Self::Ctrl => "InputKey.Ctrl".into(),
            Self::Alt => if cfg!(target_os = "macos") {
                "InputKey.Option"
            } else {
                "InputKey.Alt"
            }
            .into(),
            Self::Super => if cfg!(target_os = "macos") {
                "InputKey.Command"
            } else {
                "InputKey.Super"
            }
            .into(),
            Self::CapsLock => "InputKey.CapsLock".into(),
            Self::NumLock => "InputKey.NumLock".into(),
            Self::NumpadEnter => "InputKey.NumpadEnter".into(),
            Self::NumpadDigit(n) => format!("InputKey.NumpadDigit.{n}"),
            Self::NumpadAdd => "InputKey.NumpadAdd".into(),
            Self::NumpadSubtract => "InputKey.NumpadSubtract".into(),
            Self::NumpadMultiply => "InputKey.NumpadMultiply".into(),
            Self::NumpadDivide => "InputKey.NumpadDivide".into(),
            Self::NumpadDecimal => "InputKey.NumpadDecimal".into(),
            Self::NumpadSeparator => "InputKey.NumpadSeparator".into(),
            Self::PrintScreen => "InputKey.PrintScreen".into(),
            Self::ScrollLock => "InputKey.ScrollLock".into(),
            Self::Pause => "InputKey.Pause".into(),
            Self::ContextMenu => "InputKey.ContextMenu".into(),
            Self::Function(n) => format!("InputKey.F{n}"),
            Self::Letter(c) | Self::Digit(c) | Self::Punct(c) => c.to_string(),
        }
    }
}
