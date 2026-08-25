//! 宠物可读取的、已解析的界面明暗方案。

/// 宿主当前为宠物内容解析出的明暗方案。
///
/// 这是语义信息而非平台主题对象：宠物包可据此选择自己的气泡配色，且不需要依赖
/// egui、Windows 或其它窗口系统。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum PetTheme {
    /// 浅色方案。
    Light,
    /// 深色方案。
    #[default]
    Dark,
}
