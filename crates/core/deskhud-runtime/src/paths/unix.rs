use std::path::PathBuf;

pub(crate) fn user_data_packages() -> Option<PathBuf> {
    let home = std::env::var_os("HOME")?;
    Some(
        PathBuf::from(home)
            .join(".local")
            .join("share")
            .join("DeskHud")
            .join("packages"),
    )
}

pub(crate) fn default_pack_cache_dir() -> Option<PathBuf> {
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
