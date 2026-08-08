//! # deskhud-package
//!
//! `.deskhud` 包格式：清单、目录/zip 约定、包内 i18n 文件形状。
//! 不负责 WASM 实例化或宿主注册（见 `deskhud-runtime`）。

#![deny(missing_docs)]

pub mod error;
pub mod i18n;
pub mod io;
pub mod manifest;

pub use error::PackageError;
pub use i18n::PackCatalog;
pub use io::{
    open_pack, pack_directory, read_catalog_dir, read_manifest_dir, unpack_archive,
    write_manifest_dir, PackRoot,
};
pub use manifest::{PackHudEntry, PackKind, PackManifest};
