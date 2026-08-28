//! # deskhud-package
//!
//! `.deskhud` 包格式：清单、目录/zip 约定、包内 i18n 文件形状。
//! 不负责 WASM 实例化或宿主注册（见 `deskhud-runtime`）。

#![deny(missing_docs)]

pub mod compat;
pub mod error;
pub mod i18n;
pub mod io;
pub mod manifest;

pub use compat::{ENGINE_PRODUCT_VERSION, engine_family_of_product, pack_engine_matches};
pub use error::PackageError;
pub use i18n::PackCatalog;
pub use io::{
    PackResourceIndex, PackRoot, index_pack_dir, open_pack, pack_directory, read_catalog_dir,
    read_manifest_dir, unpack_archive, write_manifest_dir,
};
pub use manifest::{
    PackFrame, PackHudEntry, PackKind, PackManifest, PackResource, PackResourceKind,
    validate_relative_path,
};
