//! 包扫描路径约定。

use std::path::PathBuf;

/// 默认扫描根：开发树 `./packages`，以及用户数据目录下的 `packages`（若可解析）。
pub fn default_package_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(PathBuf::from("packages"));
    if let Some(data) = user_data_packages() {
        dirs.push(data);
    }
    dirs
}

fn user_data_packages() -> Option<PathBuf> {
    // 轻量约定，避免本阶段引入 directories crate；Windows 用 APPDATA。
    #[cfg(windows)]
    {
        let appdata = std::env::var_os("APPDATA")?;
        Some(PathBuf::from(appdata).join("DeskHud").join("packages"))
    }
    #[cfg(not(windows))]
    {
        let home = std::env::var_os("HOME")?;
        Some(
            PathBuf::from(home)
                .join(".local")
                .join("share")
                .join("DeskHud")
                .join("packages"),
        )
    }
}
