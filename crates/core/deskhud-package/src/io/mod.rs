//! `.deskhud` 目录 / zip 包 IO。

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{PackCatalog, PackManifest, PackResourceKind, PackageError, validate_relative_path};

/// 打开的包根：目录，或已解压到缓存的 zip。
#[derive(Debug, Clone)]
pub struct PackRoot {
    /// 可读的包根目录（含 `manifest.toml`）。
    pub path: PathBuf,
    /// 若来自 zip/`.deskhud` 文件，记录原路径。
    pub archive: Option<PathBuf>,
}

/// 包资源的可读索引。构造时会校验所有清单声明，后续读取不会越过包根。
#[derive(Debug, Clone)]
pub struct PackResourceIndex {
    root: PathBuf,
    manifest: PackManifest,
}

impl PackResourceIndex {
    /// 校验清单引用的入口、预览图、图标和所有资源，并建立索引。
    pub fn load(root: &Path, manifest: &PackManifest) -> Result<Self, PackageError> {
        manifest.validate()?;
        let index = Self {
            root: root.to_path_buf(),
            manifest: manifest.clone(),
        };
        for path in [&manifest.entry, &manifest.preview, &manifest.icon]
            .into_iter()
            .flatten()
        {
            index.validate_file(path)?;
        }
        for resource in &manifest.resources {
            index.validate_file(&resource.path)?;
            let bytes = index.read_path(&resource.path)?;
            if matches!(
                resource.kind,
                PackResourceKind::Image | PackResourceKind::Atlas | PackResourceKind::Sequence
            ) {
                validate_image(
                    &resource.path,
                    &bytes,
                    resource.width,
                    resource.height,
                    &resource.frames,
                )?;
            }
        }
        Ok(index)
    }

    /// 读取清单中声明的资源；未声明或路径不安全时失败。
    pub fn read(&self, id: &str) -> Result<Vec<u8>, PackageError> {
        let resource = self
            .manifest
            .resources
            .iter()
            .find(|r| r.id == id)
            .ok_or_else(|| PackageError::MissingEntry(format!("undeclared resource `{id}`")))?;
        self.read_path(&resource.path)
    }

    /// 返回资源声明，供渲染器解释 atlas 帧。
    pub fn resource(&self, id: &str) -> Option<&crate::PackResource> {
        self.manifest.resources.iter().find(|r| r.id == id)
    }

    fn validate_file(&self, path: &str) -> Result<(), PackageError> {
        validate_relative_path(path).map_err(|e| PackageError::InvalidResource(e.to_string()))?;
        let full = self.root.join(path);
        if !full.is_file() {
            return Err(PackageError::MissingEntry(format!(
                "resource `{path}` not found"
            )));
        }
        Ok(())
    }

    fn read_path(&self, path: &str) -> Result<Vec<u8>, PackageError> {
        self.validate_file(path)?;
        let bytes = fs::read(self.root.join(path))?;
        if bytes.is_empty() {
            return Err(PackageError::DamagedResource {
                path: path.into(),
                reason: "file is empty".into(),
            });
        }
        Ok(bytes)
    }
}

fn validate_image(
    path: &str,
    bytes: &[u8],
    width: u32,
    height: u32,
    frames: &[crate::PackFrame],
) -> Result<(), PackageError> {
    let svg = std::str::from_utf8(bytes).is_ok_and(|s| {
        let s = s.trim_start().to_ascii_lowercase();
        s.starts_with("<svg") && s.contains("</svg")
    });
    if svg {
        return Ok(());
    }
    let image = image::load_from_memory(bytes).map_err(|e| PackageError::DamagedResource {
        path: path.into(),
        reason: e.to_string(),
    })?;
    if (width != 0 && image.width() != width) || (height != 0 && image.height() != height) {
        return Err(PackageError::DamagedResource {
            path: path.into(),
            reason: format!(
                "dimensions {}x{} do not match declared {}x{}",
                image.width(),
                image.height(),
                width,
                height
            ),
        });
    }
    for frame in frames {
        let in_bounds = frame.x <= image.width()
            && frame.y <= image.height()
            && frame.width <= image.width() - frame.x
            && frame.height <= image.height() - frame.y;
        if !in_bounds {
            return Err(PackageError::DamagedResource {
                path: path.into(),
                reason: format!(
                    "frame ({},{},{},{}) exceeds image {}x{}",
                    frame.x,
                    frame.y,
                    frame.width,
                    frame.height,
                    image.width(),
                    image.height()
                ),
            });
        }
    }
    Ok(())
}

/// 从目录读取并校验清单。
pub fn read_manifest_dir(dir: &Path) -> Result<PackManifest, PackageError> {
    let text = fs::read_to_string(dir.join("manifest.toml"))?;
    PackManifest::parse_toml(&text)
}

/// 校验并索引一个已打开的目录包。
pub fn index_pack_dir(dir: &Path) -> Result<PackResourceIndex, PackageError> {
    let manifest = read_manifest_dir(dir)?;
    PackResourceIndex::load(dir, &manifest)
}

/// 将清单写入目录（创建父目录）。
pub fn write_manifest_dir(dir: &Path, manifest: &PackManifest) -> Result<(), PackageError> {
    fs::create_dir_all(dir)?;
    let text = toml::to_string_pretty(manifest)
        .map_err(|e| PackageError::InvalidManifest(format!("serialize manifest: {e}")))?;
    fs::write(dir.join("manifest.toml"), text)?;
    Ok(())
}

/// 读取包内 locale 目录；文件不存在则 `Ok(None)`。
pub fn read_catalog_dir(dir: &Path, locale: &str) -> Result<Option<PackCatalog>, PackageError> {
    let locale_dir = dir.join("i18n").join(locale);
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
    let mut messages = std::collections::BTreeMap::new();
    for path in files {
        let catalog =
            PackCatalog::parse_gettext(&fs::read(path)?).map_err(PackageError::InvalidResource)?;
        messages.extend(catalog.messages);
    }
    Ok((!messages.is_empty()).then_some(PackCatalog { messages }))
}

/// 把目录打成 `.deskhud`（zip）。
pub fn pack_directory(src_dir: &Path, dest_zip: &Path) -> Result<(), PackageError> {
    if !src_dir.is_dir() {
        return Err(PackageError::MissingEntry(format!(
            "not a directory: {}",
            src_dir.display()
        )));
    }
    // 打包前执行与加载相同的资源门闸，避免生成宿主必然拒绝的坏包。
    let _ = index_pack_dir(src_dir)?;
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(dest_zip)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, src_dir, src_dir, opts)?;
    zip.finish().map_err(|e| PackageError::Zip(e.to_string()))?;
    Ok(())
}

fn add_dir_to_zip(
    zip: &mut ZipWriter<File>,
    base: &Path,
    dir: &Path,
    opts: SimpleFileOptions,
) -> Result<(), PackageError> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let rel = path
            .strip_prefix(base)
            .map_err(|e| PackageError::Zip(e.to_string()))?;
        let name = rel.to_string_lossy().replace('\\', "/");
        if path.is_dir() {
            if !name.is_empty() {
                zip.add_directory(format!("{name}/"), opts)
                    .map_err(|e| PackageError::Zip(e.to_string()))?;
            }
            add_dir_to_zip(zip, base, &path, opts)?;
        } else {
            zip.start_file(&name, opts)
                .map_err(|e| PackageError::Zip(e.to_string()))?;
            let mut f = File::open(&path)?;
            let mut buf = Vec::new();
            f.read_to_end(&mut buf)?;
            zip.write_all(&buf)?;
        }
    }
    Ok(())
}

/// 解压 `.deskhud` / zip 到目标目录（先清空再写入）。
pub fn unpack_archive(archive: &Path, dest_dir: &Path) -> Result<PackManifest, PackageError> {
    let file = File::open(archive)?;
    let mut zip = ZipArchive::new(file).map_err(|e| PackageError::Zip(e.to_string()))?;
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    fs::create_dir_all(dest_dir)?;
    for i in 0..zip.len() {
        let mut entry = zip
            .by_index(i)
            .map_err(|e| PackageError::Zip(e.to_string()))?;
        let name = entry
            .enclosed_name()
            .ok_or_else(|| PackageError::Zip("unsafe zip path".into()))?
            .to_owned();
        let out = dest_dir.join(&name);
        if entry.is_dir() {
            fs::create_dir_all(&out)?;
        } else {
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut outfile = File::create(&out)?;
            std::io::copy(&mut entry, &mut outfile)?;
        }
    }
    read_manifest_dir(dest_dir)
}

/// 打开包：目录直接用；`.deskhud`/`.zip` 解压到 `cache_dir/<id>/`。
pub fn open_pack(path: &Path, cache_dir: &Path) -> Result<PackRoot, PackageError> {
    if path.is_dir() {
        let manifest = read_manifest_dir(path)?;
        let _ = PackResourceIndex::load(path, &manifest)?;
        return Ok(PackRoot {
            path: path.to_path_buf(),
            archive: None,
        });
    }
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "deskhud" && ext != "zip" {
        return Err(PackageError::MissingEntry(format!(
            "unsupported pack path: {}",
            path.display()
        )));
    }
    // 先解到临时名，读 id 后再落到 cache/<id>
    let tmp = cache_dir.join("_tmp_unpack");
    let manifest = unpack_archive(path, &tmp)?;
    let _ = PackResourceIndex::load(&tmp, &manifest)?;
    let dest = cache_dir.join(&manifest.id);
    if dest.exists() {
        fs::remove_dir_all(&dest)?;
    }
    fs::rename(&tmp, &dest)?;
    Ok(PackRoot {
        path: dest,
        archive: Some(path.to_path_buf()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::PackKind;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_dir(name: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("deskhud-pack-{name}-{nanos}"))
    }

    #[test]
    fn roundtrip_dir_and_zip() {
        let dir = temp_dir("dir");
        let manifest = PackManifest {
            id: "pet.example.roundtrip".into(),
            kind: PackKind::Pet,
            load: crate::PackLoadMode::Builtin,
            version: "0.3.0".into(),
            engine: "0.3".into(),
            api_version: PackManifest::SUPPORTED_API_VERSION,
            display_name: "Roundtrip".into(),
            description: "test".into(),
            author: "t".into(),
            homepage: None,
            entry: None,
            window_width: 140,
            window_height: 140,
            preview: None,
            icon: None,
            hud: vec![],
            resources: vec![],
        };
        write_manifest_dir(&dir, &manifest).unwrap();
        fs::create_dir_all(dir.join("i18n")).unwrap();
        let catalog = PackCatalog {
            messages: [("display_name".to_owned(), "往返".to_owned())]
                .into_iter()
                .collect(),
        };
        fs::create_dir_all(dir.join("i18n/zh-CN")).unwrap();
        fs::write(dir.join("i18n/zh-CN/info.mo"), catalog.to_mo()).unwrap();

        let zip_path = temp_dir("out").with_extension("deskhud");
        pack_directory(&dir, &zip_path).unwrap();

        let cache = temp_dir("cache");
        let root = open_pack(&zip_path, &cache).unwrap();
        let m2 = read_manifest_dir(&root.path).unwrap();
        assert_eq!(m2.id, manifest.id);
        let cat = read_catalog_dir(&root.path, "zh-CN").unwrap().unwrap();
        assert_eq!(
            cat.messages.get("display_name").map(String::as_str),
            Some("往返")
        );

        let _ = fs::remove_dir_all(&dir);
        let _ = fs::remove_file(&zip_path);
        let _ = fs::remove_dir_all(&cache);
    }

    #[test]
    fn rejects_missing_and_damaged_resources() {
        let dir = temp_dir("bad-resource");
        let manifest = PackManifest {
            id: "pet.example.bad_resource".into(),
            kind: PackKind::Pet,
            load: crate::PackLoadMode::Builtin,
            version: "0.6.25".into(),
            engine: "0.6".into(),
            api_version: PackManifest::SUPPORTED_API_VERSION,
            display_name: "Bad".into(),
            description: String::new(),
            author: String::new(),
            homepage: None,
            entry: None,
            window_width: 140,
            window_height: 140,
            preview: None,
            icon: None,
            hud: vec![],
            resources: vec![crate::PackResource {
                id: "pet/body".into(),
                path: "assets/body.png".into(),
                kind: crate::PackResourceKind::Image,
                width: 1,
                height: 1,
                frames: vec![],
            }],
        };
        write_manifest_dir(&dir, &manifest).unwrap();
        assert!(matches!(
            index_pack_dir(&dir),
            Err(PackageError::MissingEntry(_))
        ));
        fs::create_dir_all(dir.join("assets")).unwrap();
        fs::write(dir.join("assets/body.png"), b"not-an-image").unwrap();
        assert!(matches!(
            index_pack_dir(&dir),
            Err(PackageError::DamagedResource { .. })
        ));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn rejects_unsafe_manifest_paths() {
        let err = PackManifest::parse_toml(
            r#"id="pet.example.path" kind="pet" version="0.9" engine="0.9" api_version=4 display_name="x" preview="../x""#,
        )
        .unwrap_err();
        assert!(format!("{err}").contains(".."));
    }
}
