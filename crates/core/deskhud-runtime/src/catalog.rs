//! 从已发现包装配 [`CatalogStore`]。

use deskhud_package::{PackCatalog, PackKind};
use deskhud_ui::{CatalogStore, Locale, locale_file_candidates, locale_tag};
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};

use crate::{DiscoveredPack, PackageLoader};

/// 内置包 i18n：短键 + 包 ID 前缀，与社区 `PackCatalog` 合并规则一致。
fn merge_builtin_pack_catalogs(store: &mut CatalogStore) {
    const BUILTINS: &[(&str, &str, &str)] = &[
        (
            "pet.deskhud.mochi.",
            include_str!(concat!(env!("OUT_DIR"), "/builtin-mochi-zh-CN.po")),
            include_str!(concat!(env!("OUT_DIR"), "/builtin-mochi-en-US.po")),
        ),
        (
            "pet.deskhud.sesame.",
            include_str!(concat!(env!("OUT_DIR"), "/builtin-sesame-zh-CN.po")),
            include_str!(concat!(env!("OUT_DIR"), "/builtin-sesame-en-US.po")),
        ),
        (
            "hud.deskhud.demo.",
            include_str!(concat!(env!("OUT_DIR"), "/builtin-demo-zh-CN.po")),
            include_str!(concat!(env!("OUT_DIR"), "/builtin-demo-en-US.po")),
        ),
    ];
    for &(prefix, zh_file, en_file) in BUILTINS {
        if let Ok(cat) = PackCatalog::parse_gettext(zh_file.as_bytes()) {
            store.merge_prefixed("zh-CN", prefix, &cat.messages);
        }
        if let Ok(cat) = PackCatalog::parse_gettext(en_file.as_bytes()) {
            store.merge_prefixed("en-US", prefix, &cat.messages);
        }
    }
}

/// 输入提示是宿主运行时能力，不应因为发布目录中的 i18n 被删除而退化
/// 为中文硬编码。将键盘和鼠标 PO 作为最后一道内置资源；外部扫描到的
/// `.mo` 仍会在前面合并并覆盖这里的默认值。
fn merge_builtin_input_catalogs(store: &mut CatalogStore) {
    for (locale, source) in [
        (
            "zh-CN",
            include_str!(concat!(env!("OUT_DIR"), "/builtin-input-zh-CN.po")),
        ),
        (
            "en-US",
            include_str!(concat!(env!("OUT_DIR"), "/builtin-input-en-US.po")),
        ),
    ] {
        if let Ok(catalog) = PackCatalog::parse_gettext(source.as_bytes()) {
            store.merge_layer(locale, &catalog.messages);
        }
    }
}

/// 为当前 UI locale 合并内置包 + 已发现包文案（并尽量补 `en` 回退层）。
pub fn build_catalog_store(discovered: &[DiscoveredPack], _locale: Locale) -> CatalogStore {
    let mut store = CatalogStore::new();
    merge_builtin_input_catalogs(&mut store);
    scan_runtime_catalogs(&mut store);
    merge_builtin_pack_catalogs(&mut store);
    // Settings can switch language before Apply; keep every supported pack layer
    // available so the preview updates immediately.
    merge_discovered_into(&mut store, discovered, Locale::ZhCn);
    merge_discovered_into(&mut store, discovered, Locale::En);
    store
}

fn merge_discovered_into(store: &mut CatalogStore, discovered: &[DiscoveredPack], locale: Locale) {
    let tag = locale_tag(locale);
    for pack in discovered {
        let prefix = match pack.manifest.kind {
            PackKind::Pet | PackKind::Plugin => format!("{}.", pack.manifest.id),
        };
        for name in locale_file_candidates(locale) {
            match read_pack_catalog(&pack.root, &name) {
                Ok(Some(cat)) => {
                    store.merge_prefixed(&tag, &prefix, &cat.messages);
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

fn read_pack_catalog(
    root: &Path,
    locale: &str,
) -> Result<Option<PackCatalog>, crate::RuntimeError> {
    if let Some(cat) = PackageLoader::read_catalog(root, locale)? {
        return Ok(Some(cat));
    }
    let locale_dir = root.join("i18n").join(locale);
    let Ok(entries) = fs::read_dir(&locale_dir) else {
        return Ok(None);
    };
    let mut files = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.is_file()
                && path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    .is_some_and(|extension| extension.eq_ignore_ascii_case("mo"))
        })
        .collect::<Vec<_>>();
    files.sort();
    let mut merged = std::collections::BTreeMap::new();
    for path in files {
        match fs::read(&path) {
            Ok(bytes) => {
                let cat = PackCatalog::parse_gettext(&bytes).map_err(|e| {
                    crate::RuntimeError::Package(deskhud_package::PackageError::InvalidResource(e))
                })?;
                merged.extend(cat.messages);
            }
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => return Err(crate::RuntimeError::Package(err.into())),
        }
    }
    Ok((!merged.is_empty()).then_some(PackCatalog { messages: merged }))
}

/// Scans executable-adjacent `i18n/` and the working-directory `i18n/`.
/// Files are read on every bootstrap, so adding a catalog takes effect on restart
/// without recompiling; malformed files are isolated to that file.
pub fn scan_runtime_catalogs(store: &mut CatalogStore) {
    let mut roots = BTreeSet::<PathBuf>::new();
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        roots.insert(parent.join("i18n"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        roots.insert(cwd.join("i18n"));
    }
    for root in roots {
        scan_locale_root(store, &root);
    }
}

fn scan_locale_root(store: &mut CatalogStore, root: &Path) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(raw_tag) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let tag = deskhud_ui::normalize_locale_tag(raw_tag);
        if tag.is_empty() {
            continue;
        }
        let files: Vec<PathBuf> = fs::read_dir(&path)
            .into_iter()
            .flatten()
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("mo"))
            .collect();
        for file in files {
            match fs::read(&file)
                .map_err(|e| e.to_string())
                .and_then(|b| PackCatalog::parse_gettext(&b))
            {
                Ok(cat) => store.merge_layer(&tag, &cat.messages),
                Err(err) => tracing::warn!(path = ?file, %err, "ignore invalid gettext catalog"),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::build_catalog_store;
    use deskhud_ui::Locale;

    #[test]
    fn builtin_pet_catalogs_are_available_to_settings_in_chinese() {
        let store = build_catalog_store(&[], Locale::ZhCn);
        assert_eq!(
            store.get(Locale::ZhCn, "pet.deskhud.mochi.display_name"),
            Some("糯米团")
        );
        assert_eq!(
            store.get(Locale::ZhCn, "pet.deskhud.sesame.display_name"),
            Some("芝麻豆")
        );
        assert_eq!(
            store.get(Locale::En, "pet.deskhud.mochi.follow_eyes.label"),
            Some("Eye effect")
        );
        assert_eq!(store.get(Locale::En, "InputKey.Escape"), Some("Esc"));
        assert_eq!(
            store.get(Locale::En, "InputKeyPrimary"),
            Some("Left mouse button")
        );
        assert_eq!(store.get(Locale::ZhCn, "InputKey.Escape"), Some("Esc"));
    }
}
