//! 应用内各个窗口的 egui 视图。
//!
//! 每个视图只负责构建自己的 UI，并通过 [`ViewOutput`] 向运行时返回操作请求。

pub(crate) mod bubble;
pub(crate) mod hud;
pub(crate) mod pet;
pub(crate) mod setting;
pub(crate) mod theme;

use deskhud_ui::UiPreferences;
use egui::FullOutput;

/// egui 视图完成一帧绘制后返回给应用层的结果。
#[derive(Default)]
pub(crate) struct ViewOutput {
    /// egui 生成的平台输出和绘制数据。
    pub full_output: FullOutput,
    /// 请求关闭当前窗口或整个应用。
    pub should_close: bool,
    /// 设置页点击“应用”后提交的偏好。
    pub applied_preferences: Option<UiPreferences>,
    /// 请求调整当前窗口的内容尺寸，单位为逻辑像素。
    pub resize_to: Option<[f32; 2]>,
    /// 请求移动当前原生窗口，单位为逻辑像素增量。
    pub move_by: Option<[f32; 2]>,
    /// 被菜单点击的菜单项标识，由菜单窗口返回给窗口管理器。
    pub selected_menu_item: Option<String>,
    /// 当前悬浮的子菜单索引。
    pub open_submenu: Option<usize>,
    /// 当前悬浮的菜单项索引。
    pub hovered_item: Option<usize>,
    /// 子菜单触发项相对于父菜单客户区的逻辑矩形 `[x, y, height]`。
    pub submenu_anchor: Option<[f32; 3]>,
}
