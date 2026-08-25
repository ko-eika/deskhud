//! Pet 菜单模块。
//!
//! Pet 菜单相关代码统一放在本目录：菜单定义负责描述业务项，窗口适配负责
//! 管理通用菜单控制器。后续新增 Pet 菜单功能时，也应优先在本目录内扩展。

mod definition;
mod window;

pub(crate) use definition::{Action as PetMenuAction, definition, parse_action};
pub(crate) use window::PetMenu;
