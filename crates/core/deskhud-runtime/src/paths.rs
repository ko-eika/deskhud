//! 包扫描路径约定。

use std::path::PathBuf;

#[cfg(windows)]
#[path = "paths/windows.rs"]
mod platform;
#[cfg(unix)]
#[path = "paths/unix.rs"]
mod platform;
#[cfg(not(any(windows, unix)))]
#[path = "paths/fallback.rs"]
mod platform;

/// 默认扫描根：可执行文件旁的 profile 包目录，以及用户数据目录下的 `packages`。
pub fn default_package_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    // Release and debug binaries load sibling packages, which makes
    // `target/<profile>/packages` self-contained and relocatable.
    if let Ok(exe) = std::env::current_exe()
        && let Some(parent) = exe.parent()
    {
        dirs.push(parent.join("packages"));
    }
    if let Some(data) = platform::user_data_packages() {
        dirs.push(data);
    }
    dirs
}

/// Returns the default cache directory for extracted package archives.
pub(crate) fn default_pack_cache_dir() -> Option<PathBuf> {
    platform::default_pack_cache_dir()
}
