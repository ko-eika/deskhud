//! 宠物偏好（落盘 `[pet]`：窗体字段 + `pet.global.*` + 包选项扁平键）。

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use crate::shell::{LayerPreference, PetPickerMode};

/// 当前宠物窗体与包级选项。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PetPrefs {
    /// 当前宠物类型 ID。
    #[serde(default = "default_kind")]
    pub kind: String,
    /// 宠窗宽（来自宠物包元数据缓存）。
    #[serde(default = "default_size")]
    pub width: f32,
    /// 宠窗高。
    #[serde(default = "default_size")]
    pub height: f32,
    /// 宠窗左上角 X（逻辑像素）。
    #[serde(default)]
    pub pos_x: Option<f32>,
    /// 宠窗左上角 Y。
    #[serde(default)]
    pub pos_y: Option<f32>,
    /// 设置页宠物选择：网格 / 列表。
    #[serde(default)]
    pub picker_mode: PetPickerMode,
    /// 宠物桌面覆盖层级。
    #[serde(default)]
    pub layer: LayerPreference,
    /// 是否显示聊天气泡。
    #[serde(default = "default_true")]
    pub bubbles: bool,
    /// 包自定义布尔选项（扁平键，如 `pet.deskhud.specs.follow_eyes`）。
    #[serde(default, flatten)]
    pub options: HashMap<String, bool>,
}

/// 二维宠物窗口尺寸。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PetSize {
    /// 窗口宽度。
    pub width: f32,
    /// 窗口高度。
    pub height: f32,
}

/// 二维宠物屏幕位置。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct PetPosition {
    /// 屏幕 X 坐标。
    pub x: f32,
    /// 屏幕 Y 坐标。
    pub y: f32,
}

fn default_kind() -> String {
    "pet.deskhud.specs".into()
}

fn default_size() -> f32 {
    140.0
}

fn default_true() -> bool {
    true
}

impl Default for PetPrefs {
    fn default() -> Self {
        Self {
            kind: default_kind(),
            width: default_size(),
            height: default_size(),
            pos_x: None,
            pos_y: None,
            picker_mode: PetPickerMode::Grid,
            layer: LayerPreference::default(),
            bubbles: true,
            options: HashMap::new(),
        }
    }
}

impl PetPrefs {
    /// 返回组合尺寸。
    pub fn size(&self) -> PetSize {
        PetSize {
            width: self.width,
            height: self.height,
        }
    }

    /// 返回组合位置（仅当 X/Y 均存在时）。
    pub fn position(&self) -> Option<PetPosition> {
        Some(PetPosition {
            x: self.pos_x?,
            y: self.pos_y?,
        })
    }
    /// 当前宠物类型：`pet.global.kind`。
    pub const GLOBAL_KIND_KEY: &'static str = "pet.global.kind";
    /// 宠窗宽：`pet.global.width`。
    pub const GLOBAL_WIDTH_KEY: &'static str = "pet.global.width";
    /// 宠窗高：`pet.global.height`。
    pub const GLOBAL_HEIGHT_KEY: &'static str = "pet.global.height";
    /// 宠窗 X：`pet.global.pos_x`。
    pub const GLOBAL_POS_X_KEY: &'static str = "pet.global.pos_x";
    /// 宠窗 Y：`pet.global.pos_y`。
    pub const GLOBAL_POS_Y_KEY: &'static str = "pet.global.pos_y";
    /// 宠选择视图键：`pet.global.picker_mode`（落盘字符串，内存仍用 `picker_mode`）。
    pub const GLOBAL_PICKER_MODE_KEY: &'static str = "pet.global.picker_mode";
    /// 宠物层级键：`pet.global.layer`。
    pub const GLOBAL_LAYER_KEY: &'static str = "pet.global.layer";
    /// 聊天气泡开关键：`pet.global.bubbles`。
    pub const GLOBAL_BUBBLES_KEY: &'static str = "pet.global.bubbles";

    /// 规范键：`{pet_id}.{option_key}`。
    pub fn option_key(pet_id: &str, option_key: &str) -> String {
        format!("{pet_id}.{option_key}")
    }

    /// 用宠物元数据覆盖窗尺寸。
    pub fn apply_window_size(&mut self, width: f32, height: f32) {
        self.width = width.max(48.0);
        self.height = height.max(48.0);
    }

    /// 记录宠窗屏幕位置（逻辑像素）。
    pub fn set_pos(&mut self, x: f32, y: f32) {
        self.pos_x = Some(x);
        self.pos_y = Some(y);
    }

    /// 若有已存位置则返回。
    pub fn pos(&self) -> Option<[f32; 2]> {
        Some([self.pos_x?, self.pos_y?])
    }

    /// 读取布尔配置；缺省 `default`。
    pub fn get_bool(&self, key: &str, default: bool) -> bool {
        self.options.get(key).copied().unwrap_or(default)
    }

    /// 读取宠选项。
    pub fn get_option(&self, pet_id: &str, option_key: &str, default: bool) -> bool {
        self.get_bool(&Self::option_key(pet_id, option_key), default)
    }

    /// 写入布尔配置。
    pub fn set_bool(&mut self, key: impl Into<String>, value: bool) {
        self.options.insert(key.into(), value);
    }

    /// 写入宠选项。
    pub fn set_option(&mut self, pet_id: &str, option_key: &str, value: bool) {
        self.set_bool(Self::option_key(pet_id, option_key), value);
    }

    /// 解析当前宠的短键表（供 `PetConfigBag`）。
    pub fn short_map_for(&self, pet_id: &str, options: &[(&str, bool)]) -> HashMap<String, bool> {
        let mut map = HashMap::new();
        for &(key, default) in options {
            map.insert(key.to_string(), self.get_option(pet_id, key, default));
        }
        map
    }
}
