//! `.deskhud` 目录 / zip 包 IO。

use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use crate::{PackCatalog, PackManifest, PackageError};

/// 打开的包根：目录，或已解压到缓存的 zip。
#[derive(Debug, Clone)]
pub struct PackRoot {
    /// 可读的包根目录（含 `manifest.toml`）。
    pub path: PathBuf,
    /// 若来自 zip/`.deskhud` 文件，记录原路径。
    pub archive: Option<PathBuf>,
}

/// 从目录读取并校验清单。
pub fn read_manifest_dir(dir: &Path) -> Result<PackManifest, PackageError> {
    let text = fs::read_to_string(dir.join("manifest.toml"))?;
    PackManifest::parse_toml(&text)
}

/// 将清单写入目录（创建父目录）。
pub fn write_manifest_dir(dir: &Path, manifest: &PackManifest) -> Result<(), PackageError> {
    fs::create_dir_all(dir)?;
    let text = toml::to_string_pretty(manifest).map_err(|e| {
        PackageError::InvalidManifest(format!("serialize manifest: {e}"))
    })?;
    fs::write(dir.join("manifest.toml"), text)?;
    Ok(())
}

/// 读取包内 locale 目录；文件不存在则 `Ok(None)`。
pub fn read_catalog_dir(dir: &Path, locale: &str) -> Result<Option<PackCatalog>, PackageError> {
    let path = dir.join("i18n").join(format!("{locale}.toml"));
    if !path.exists() {
        return Ok(None);
    }
    let text = fs::read_to_string(path)?;
    Ok(Some(PackCatalog::parse_toml(&text)?))
}

/// 把目录打成 `.deskhud`（zip）。
pub fn pack_directory(src_dir: &Path, dest_zip: &Path) -> Result<(), PackageError> {
    if !src_dir.is_dir() {
        return Err(PackageError::MissingEntry(format!(
            "not a directory: {}",
            src_dir.display()
        )));
    }
    // 确保有清单
    let _ = read_manifest_dir(src_dir)?;
    if let Some(parent) = dest_zip.parent() {
        fs::create_dir_all(parent)?;
    }
    let file = File::create(dest_zip)?;
    let mut zip = ZipWriter::new(file);
    let opts = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
    add_dir_to_zip(&mut zip, src_dir, src_dir, opts)?;
    zip.finish()
        .map_err(|e| PackageError::Zip(e.to_string()))?;
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
    let mut zip =
        ZipArchive::new(file).map_err(|e| PackageError::Zip(e.to_string()))?;
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
        let _ = manifest;
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
        };
        write_manifest_dir(&dir, &manifest).unwrap();
        fs::create_dir_all(dir.join("i18n")).unwrap();
        fs::write(
            dir.join("i18n/zh-CN.toml"),
            "display_name = \"往返\"\n",
        )
        .unwrap();

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
}
