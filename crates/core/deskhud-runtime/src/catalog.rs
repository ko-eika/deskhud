//! 从已发现包装配 [`CatalogStore`]。

use deskhud_package::{PackCatalog, PackKind};
use deskhud_ui::{CatalogStore, Locale, locale_file_candidates, locale_tag};

use crate::{DiscoveredPack, PackageLoader};

/// 内置包 i18n：短键 + 包 ID 前缀，与社区 `PackCatalog` 合并规则一致。
fn merge_builtin_pack_catalogs(store: &mut CatalogStore) {
    const BUILTINS: &[(&str, &str, &str)] = &[
        (
            "pet.deskhud.specs.",
            include_str!("../../../packs/pet-deskhud-specs/i18n/zh-CN.toml"),
            include_str!("../../../packs/pet-deskhud-specs/i18n/en.toml"),
        ),
        (
            "pet.deskhud.blob.",
            include_str!("../../../packs/pet-deskhud-blob/i18n/zh-CN.toml"),
            include_str!("../../../packs/pet-deskhud-blob/i18n/en.toml"),
        ),
        (
            "hud.deskhud.demo.",
            include_str!("../../../packs/hud-deskhud-demo/i18n/zh-CN.toml"),
            include_str!("../../../packs/hud-deskhud-demo/i18n/en.toml"),
        ),
    ];
    for &(prefix, zh, en) in BUILTINS {
        if let Ok(cat) = PackCatalog::parse_toml(zh) {
            store.merge_prefixed("zh-CN", prefix, &cat.messages);
        }
        if let Ok(cat) = PackCatalog::parse_toml(en) {
            store.merge_prefixed("en", prefix, &cat.messages);
        }
    }
}

/// 为当前 UI locale 合并内置包 + 已发现包文案（并尽量补 `en` 回退层）。
pub fn build_catalog_store(discovered: &[DiscoveredPack], locale: Locale) -> CatalogStore {
    let mut store = CatalogStore::new();
    merge_builtin_pack_catalogs(&mut store);
    merge_discovered_into(&mut store, discovered, locale);
    if locale != Locale::En {
        merge_discovered_into(&mut store, discovered, Locale::En);
    }
    store
}

fn merge_discovered_into(store: &mut CatalogStore, discovered: &[DiscoveredPack], locale: Locale) {
    let tag = locale_tag(locale);
    for pack in discovered {
        let prefix = match pack.manifest.kind {
            PackKind::Pet | PackKind::Plugin => format!("{}.", pack.manifest.id),
        };
        for name in locale_file_candidates(locale) {
            match PackageLoader::read_catalog(&pack.root, name) {
                Ok(Some(cat)) => {
                    store.merge_prefixed(tag, &prefix, &cat.messages);
                    break;
                }
                Ok(None) => {}
                Err(err) => {
                    tracing::warn!(
                        id = %pack.manifest.id,
                        locale = %name,
                        %err,
                        "read pack catalog failed"
                    );
                }
            }
        }
    }
}
