//! 宠物包可声明的配置项（设置页开关；prefs 键为 `{pet_id}.{key}`）。

/// 一条宠配置。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PetConfigOption {
    /// 短键（插件内 / 宠内唯一），如 `follow_eyes`。
    pub key: &'static str,
    /// 设置页标题。
    pub label: &'static str,
    /// 说明。
    pub description: &'static str,
    /// 未写入 prefs 时的默认值。
    pub default: bool,
}
