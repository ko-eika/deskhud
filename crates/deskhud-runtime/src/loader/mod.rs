//! 本地包加载器（骨架）。

use std::fs;
use std::path::{Path, PathBuf};

use deskhud_package::{PackCatalog, PackManifest, PackageError};
use tracing::warn;

use crate::{RuntimeError, default_package_dirs};

/// 已发现但尚未实例化的包。
#[derive(Debug, Clone)]
pub struct DiscoveredPack {
    /// 包根目录（解压后的 `.deskhud` 目录）。
    pub root: PathBuf,
    /// 清单。
    pub manifest: PackManifest,
}

/// 扫描并解析清单；WASM 实例化见 [`crate::wasm`]。
#[derive(Debug, Default)]
pub struct PackageLoader {
    roots: Vec<PathBuf>,
}

impl PackageLoader {
    /// 使用默认扫描路径。
    pub fn new() -> Self {
        Self {
            roots: default_package_dirs(),
        }
    }

    /// 自定义扫描根。
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self { roots }
    }

    /// 发现所有含 `manifest.toml` 的包目录。
    pub fn discover(&self) -> Result<Vec<DiscoveredPack>, RuntimeError> {
        let mut out = Vec::new();
        for root in &self.roots {
            if !root.exists() {
                continue;
            }
            for entry in fs::read_dir(root)? {
                let entry = entry?;
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                match load_manifest_dir(&path) {
                    Ok(manifest) => out.push(DiscoveredPack {
                        root: path,
                        manifest,
                    }),
                    Err(err) => warn!(?path, %err, "skip pack"),
                }
            }
        }
        Ok(out)
    }

    /// 读取包内某一 locale 的目录（文件可不存在）。
    pub fn read_catalog(
        pack_root: &Path,
        locale: &str,
    ) -> Result<Option<PackCatalog>, RuntimeError> {
        let path = pack_root.join("i18n").join(format!("{locale}.toml"));
        if !path.exists() {
            return Ok(None);
        }
        let text = fs::read_to_string(path)?;
        let catalog = PackCatalog::parse_toml(&text).map_err(PackageError::from)?;
        Ok(Some(catalog))
    }
}

fn load_manifest_dir(dir: &Path) -> Result<PackManifest, RuntimeError> {
    let text = fs::read_to_string(dir.join("manifest.toml"))?;
    Ok(PackManifest::parse_toml(&text)?)
}
