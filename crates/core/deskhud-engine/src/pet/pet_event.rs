//! 宿主派发给宠物包的事件。

use super::{DockState, DragState, PetKey, PetModifiers, PetMouseButton};

/// 宠物可响应的输入 / 环境事件。
///
/// 几何与 OS 细节只在 UI 壳计算；此处只传中性状态，供内置宠与日后 WASM Guest 共用。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PetEvent {
    /// 开始拖动宠窗。
    DragStarted,
    /// 结束拖动（吸附与 [`Self::DockChanged`] 可能紧随其后）。
    DragEnded {
        /// 结束瞬间的拖拽状态（一般为 idle）。
        drag: DragState,
    },
    /// 贴边状态变化（含松手吸附后）。
    DockChanged {
        /// 变化前。
        from: DockState,
        /// 变化后。
        to: DockState,
    },
    /// 指针进入 / 离开宠可点区域（局部）。
    MouseHover {
        /// 当前是否在区域内。
        inside: bool,
    },
    /// 鼠标键按下（指针在宠区域内，局部）。
    MousePressed {
        /// 按键。
        button: PetMouseButton,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 鼠标键抬起（局部跟踪中）。
    MouseReleased {
        /// 按键。
        button: PetMouseButton,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 单击（局部；右键仍会再开系统菜单）。
    MouseClicked {
        /// 按键。
        button: PetMouseButton,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 双击（局部）。
    MouseDoubleClicked {
        /// 按键。
        button: PetMouseButton,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 鼠标滚轮（宠物窗口局部输入；delta 正=向上，负=向下）。
    MouseWheel {
        /// 归一化滚轮刻度。
        delta: i8,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 全局鼠标键按下（不要求指针在宠上；与 `pointer_dir` 同属桌面采样）。
    GlobalMousePressed {
        /// 按键。
        button: PetMouseButton,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 全局鼠标键抬起。
    GlobalMouseReleased {
        /// 按键。
        button: PetMouseButton,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 全局鼠标滚轮（桌面采样；`delta` 为刻度，正=向上，负=向下）。
    GlobalMouseWheel {
        /// 滚轮刻度（通常 ±1 起）。
        delta: i8,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 全局键盘按下（桌面采样子集，不要求宠窗焦点；非完整钩子）。
    GlobalKeyPressed {
        /// 按键。
        key: PetKey,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 全局键盘抬起。
    GlobalKeyReleased {
        /// 按键。
        key: PetKey,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 键盘按下（需宠窗拥有键盘焦点；非全局热键）。
    KeyPressed {
        /// 按键。
        key: PetKey,
        /// 修饰键。
        modifiers: PetModifiers,
    },
    /// 键盘抬起。
    KeyReleased {
        /// 按键。
        key: PetKey,
        /// 修饰键。
        modifiers: PetModifiers,
    },
}
