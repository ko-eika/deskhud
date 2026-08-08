//! 鼠标即时状态（中性）：区分「宠上局部」与「桌面全局」。

/// 悬停 / 按键快照，供 [`crate::PetPaintCtx`] 每帧读取。
///
/// - **全局**：光标与按键在整桌面采样（与眼睛跟鼠标同一路思路）。
/// - **局部**：仅指针在宠可点区域内时的交互。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct MouseState {
    /// 指针是否在宠可点区域内（局部）。
    pub hovering: bool,
    /// 宠上主按钮按下（局部）。
    pub primary_down: bool,
    /// 宠上次按钮按下（局部）。
    pub secondary_down: bool,
    /// 宠上中键按下（局部）。
    pub middle_down: bool,
    /// 桌面任意处主按钮按下（全局，`GetAsyncKeyState` / 等价）。
    pub global_primary_down: bool,
    /// 桌面任意处次按钮按下（全局）。
    pub global_secondary_down: bool,
    /// 桌面任意处中键按下（全局）。
    pub global_middle_down: bool,
}

impl MouseState {
    /// 空闲默认。
    pub const IDLE: Self = Self {
        hovering: false,
        primary_down: false,
        secondary_down: false,
        middle_down: false,
        global_primary_down: false,
        global_secondary_down: false,
        global_middle_down: false,
    };
}
