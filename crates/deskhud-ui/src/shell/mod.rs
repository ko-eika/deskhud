//! 桌宠外壳偏好。

use serde::{Deserialize, Serialize};

/// 设置页宠物选择视图。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PetPickerMode {
    /// 网格卡片。
    #[default]
    Grid,
    /// 列表行。
    List,
}

/// 外壳窗口偏好。
///
/// 注意：`pet_width` / `pet_height` 是当前激活宠的尺寸缓存，
/// 切换宠物时应从宠物元数据同步，不要写死。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ShellPrefs {
    /// 当前宠物类型 ID。
    pub active_pet_kind_id: String,
    /// 当前宠窗宽（来自宠物包）。
    pub pet_width: f32,
    /// 当前宠窗高。
    pub pet_height: f32,
    /// 宠窗左上角屏幕 X（egui 逻辑像素）；缺省由系统摆放。
    #[serde(default)]
    pub pet_pos_x: Option<f32>,
    /// 宠窗左上角屏幕 Y。
    #[serde(default)]
    pub pet_pos_y: Option<f32>,
    /// 置顶。
    pub pet_topmost: bool,
    /// 设置页宠物选择：网格 / 列表。
    #[serde(default)]
    pub pet_picker_mode: PetPickerMode,
    /// 设置窗宽（逻辑像素）。
    #[serde(default)]
    pub settings_width: Option<f32>,
    /// 设置窗高。
    #[serde(default)]
    pub settings_height: Option<f32>,
    /// 设置窗左上角 X。
    #[serde(default)]
    pub settings_pos_x: Option<f32>,
    /// 设置窗左上角 Y。
    #[serde(default)]
    pub settings_pos_y: Option<f32>,
}

impl Default for ShellPrefs {
    fn default() -> Self {
        Self {
            active_pet_kind_id: "pet.deskhud.specs".into(),
            // 占位；启动后由宿主按 PetKindInfo 覆盖
            pet_width: 140.0,
            pet_height: 140.0,
            pet_pos_x: None,
            pet_pos_y: None,
            pet_topmost: true,
            pet_picker_mode: PetPickerMode::Grid,
            settings_width: None,
            settings_height: None,
            settings_pos_x: None,
            settings_pos_y: None,
        }
    }
}

impl ShellPrefs {
    /// 设置窗最小内尺寸（16:9）。
    pub const SETTINGS_MIN_W: f32 = 800.0;
    /// 设置窗最小内高度（16:9）。
    pub const SETTINGS_MIN_H: f32 = 450.0;
    /// 设置窗默认宽（16:9）。
    pub const SETTINGS_DEFAULT_W: f32 = 960.0;
    /// 设置窗默认高（16:9）。
    pub const SETTINGS_DEFAULT_H: f32 = 540.0;

    /// 用宠物元数据覆盖窗尺寸。
    pub fn apply_pet_window_size(&mut self, width: f32, height: f32) {
        self.pet_width = width.max(48.0);
        self.pet_height = height.max(48.0);
    }

    /// 记录宠窗屏幕位置（逻辑像素）。
    pub fn set_pet_pos(&mut self, x: f32, y: f32) {
        self.pet_pos_x = Some(x);
        self.pet_pos_y = Some(y);
    }

    /// 若有已存位置则返回。
    pub fn pet_pos(&self) -> Option<[f32; 2]> {
        Some([self.pet_pos_x?, self.pet_pos_y?])
    }

    /// 设置窗默认 / 已存尺寸。
    pub fn settings_size(&self) -> [f32; 2] {
        [
            self.settings_width
                .unwrap_or(Self::SETTINGS_DEFAULT_W)
                .clamp(Self::SETTINGS_MIN_W, 1600.0),
            self.settings_height
                .unwrap_or(Self::SETTINGS_DEFAULT_H)
                .clamp(Self::SETTINGS_MIN_H, 1200.0),
        ]
    }

    /// 设置窗已存位置。
    pub fn settings_pos(&self) -> Option<[f32; 2]> {
        Some([self.settings_pos_x?, self.settings_pos_y?])
    }

    /// 写入设置窗几何（尺寸 + 外框位置）。
    pub fn set_settings_geometry(&mut self, width: f32, height: f32, pos_x: f32, pos_y: f32) {
        self.settings_width = Some(width.clamp(Self::SETTINGS_MIN_W, 1600.0));
        self.settings_height = Some(height.clamp(Self::SETTINGS_MIN_H, 1200.0));
        self.settings_pos_x = Some(pos_x);
        self.settings_pos_y = Some(pos_y);
    }
}
