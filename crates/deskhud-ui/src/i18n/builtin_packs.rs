//! 内置宠 / 演示插件的多语言文案入口（已迁至 `packs/*/i18n`）。
//!
//! 运行时由 [`deskhud_runtime::build_catalog_store`] 经 `PackCatalog` + 前缀合并；
//! 本函数保留为空操作，避免破坏旧调用点。

use super::CatalogStore;

/// 不再硬编码文案；请依赖 runtime 的内置包 i18n 合并。
pub fn seed_builtin_packs(_store: &mut CatalogStore) {}
