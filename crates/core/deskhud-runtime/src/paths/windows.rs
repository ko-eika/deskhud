use std::path::PathBuf;

pub(crate) fn user_data_packages() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("DeskHud").join("packages"))
}

pub(crate) fn default_pack_cache_dir() -> Option<PathBuf> {
    let appdata = std::env::var_os("APPDATA")?;
    Some(PathBuf::from(appdata).join("DeskHud").join("cache"))
}
