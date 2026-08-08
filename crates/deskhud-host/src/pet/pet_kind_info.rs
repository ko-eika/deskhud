//! 宠物类型元数据。

/// 注册表用的宠物描述。
#[derive(Debug, Clone, PartialEq)]
pub struct PetKindInfo {
    /// 稳定 ID，格式 `pet.<组织>.<标识>`，如 `pet.deskhud.specs`。
    pub id: &'static str,
    /// 显示名。
    pub display_name: &'static str,
    /// 简短说明。
    pub description: &'static str,
    /// 作者 / 来源（便于用户识别包来源）。
    pub author: &'static str,
    /// 主页或仓库 URL（可选）。
    pub homepage: Option<&'static str>,
    /// 主宠窗逻辑像素宽（由皮肤/包决定，宿主不写死）。
    pub window_width: f32,
    /// 主宠窗逻辑像素高。
    pub window_height: f32,
    /// 设置页静态预览图字节（可选；png/jpeg/gif/webp 等，内置宠用 `include_bytes!`）。
    pub preview_png: Option<&'static [u8]>,
}

impl PetKindInfo {
    /// 窗尺寸。
    pub fn window_size(&self) -> [f32; 2] {
        [self.window_width, self.window_height]
    }
}
