//! 本地包加载器：扫描目录与 `.deskhud` zip，解压到缓存后解析清单。

use std::fs;
use std::path::{Path, PathBuf};

use deskhud_package::{
    PackCatalog, PackManifest, engine_family_of_product, open_pack, pack_engine_matches,
    read_catalog_dir, read_manifest_dir,
};
use tracing::{info, warn};

use crate::{RuntimeError, default_package_dirs};

/// 本 crate / workspace 的引擎产品 SemVer（发现时用于 `engine` 门闸）。
const RUNTIME_PRODUCT_VERSION: &str = env!("CARGO_PKG_VERSION");

/// 已发现但尚未实例化的包。
#[derive(Debug, Clone)]
pub struct DiscoveredPack {
    /// 包根目录（目录包或解压后的缓存）。
    pub root: PathBuf,
    /// 若来自归档，原 `.deskhud` / zip 路径。
    pub archive: Option<PathBuf>,
    /// 清单。
    pub manifest: PackManifest,
    /// 不适配原因（如 `engine` 族不匹配）；有值则仍列入发现列表，但勿注册 WASM。
    pub incompatible_reason: Option<String>,
}

impl DiscoveredPack {
    /// 是否可通过后续注册（`engine` 族与产品匹配）。
    pub fn is_compatible(&self) -> bool {
        self.incompatible_reason.is_none()
    }
}

/// 扫描并解析清单；WASM 实例化见 [`crate::wasm`]。
#[derive(Debug, Default)]
pub struct PackageLoader {
    roots: Vec<PathBuf>,
    cache_dir: PathBuf,
}

impl PackageLoader {
    /// 使用默认扫描路径与缓存目录。
    pub fn new() -> Self {
        Self {
            roots: default_package_dirs(),
            cache_dir: default_pack_cache_dir().unwrap_or_else(|| PathBuf::from("packages/.cache")),
        }
    }

    /// 自定义扫描根与缓存。
    pub fn with_roots(roots: Vec<PathBuf>, cache_dir: PathBuf) -> Self {
        Self { roots, cache_dir }
    }

    /// 发现目录包与 `.deskhud`/`.zip` 归档。
    pub fn discover(&self) -> Result<Vec<DiscoveredPack>, RuntimeError> {
        let mut out = Vec::new();
        if let Err(error) = fs::create_dir_all(&self.cache_dir)
            && error.kind() != std::io::ErrorKind::AlreadyExists
        {
            return Err(error.into());
        }
        for root in &self.roots {
            if !root.exists() {
                continue;
            }
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let path = entry.path();
                // 跳过缓存目录
                if path.file_name().and_then(|n| n.to_str()) == Some(".cache") {
                    continue;
                }
                match self.load_entry(&path) {
                    Ok(Some(pack)) => {
                        if let Some(reason) = &pack.incompatible_reason {
                            warn!(
                                id = %pack.manifest.id,
                                root = %pack.root.display(),
                                %reason,
                                "discovered incompatible pack"
                            );
                        } else {
                            info!(
                                id = %pack.manifest.id,
                                root = %pack.root.display(),
                                "discovered pack"
                            );
                        }
                        out.push(pack);
                    }
                    Ok(None) => {}
                    Err(err) => warn!(?path, %err, "skip pack"),
                }
            }
        }
        Ok(out)
    }

    fn load_entry(&self, path: &Path) -> Result<Option<DiscoveredPack>, RuntimeError> {
        if path.is_dir() {
            if !path.join("manifest.toml").exists() {
                return Ok(None);
            }
            let manifest = read_manifest_dir(path)?;
            return Ok(Some(make_discovered(path.to_path_buf(), None, manifest)));
        }
        let ext = path
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if ext != "deskhud" && ext != "zip" {
            return Ok(None);
        }
        let opened = open_pack(path, &self.cache_dir)?;
        let manifest = read_manifest_dir(&opened.path)?;
        Ok(Some(make_discovered(opened.path, opened.archive, manifest)))
    }

    /// 读取包内某一 locale 的目录（文件可不存在）。
    pub fn read_catalog(
        pack_root: &Path,
        locale: &str,
    ) -> Result<Option<PackCatalog>, RuntimeError> {
        Ok(read_catalog_dir(pack_root, locale)?)
    }
}

fn make_discovered(
    root: PathBuf,
    archive: Option<PathBuf>,
    manifest: PackManifest,
) -> DiscoveredPack {
    let incompatible_reason = engine_incompatible_reason(&manifest);
    DiscoveredPack {
        root,
        archive,
        manifest,
        incompatible_reason,
    }
}

fn engine_incompatible_reason(manifest: &PackManifest) -> Option<String> {
    if pack_engine_matches(&manifest.engine, RUNTIME_PRODUCT_VERSION) {
        return None;
    }
    let need = engine_family_of_product(RUNTIME_PRODUCT_VERSION);
    Some(format!(
        "engine `{}` incompatible (need `{}` for product {})",
        manifest.engine, need, RUNTIME_PRODUCT_VERSION
    ))
}

fn default_pack_cache_dir() -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let appdata = std::env::var_os("APPDATA")?;
        Some(
            PathBuf::from(appdata)
                .join("DeskHud")
                .join("cache")
                .join("packs"),
        )
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("DeskHud")
                .join("cache")
                .join("packs"),
        )
    }
}
