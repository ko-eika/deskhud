//! 从已发现包装配 [`CatalogStore`]。

use deskhud_package::PackKind;
use deskhud_ui::{
    locale_file_candidates, locale_tag, seed_builtin_packs, CatalogStore, Locale,
};

use crate::{DiscoveredPack, PackageLoader};

/// 为当前 UI locale 合并内置包 + 已发现包文案（并尽量补 `en` 回退层）。
pub fn build_catalog_store(discovered: &[DiscoveredPack], locale: Locale) -> CatalogStore {
    let mut store = CatalogStore::new();
    seed_builtin_packs(&mut store);
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
