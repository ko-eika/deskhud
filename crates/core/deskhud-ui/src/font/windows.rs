use std::path::PathBuf;

pub(crate) fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Ok(windir) = std::env::var("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    } else {
        dirs.push(PathBuf::from(r"C:\Windows\Fonts"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join(r"Microsoft\Windows\Fonts"));
    }
    dirs
}

pub(crate) fn priority_system_cjk() -> Vec<PathBuf> {
    [
        "msyh.ttc",
        "msyh.ttf",
        "msyhbd.ttc",
        "simhei.ttf",
        "simsun.ttc",
        "msyhl.ttc",
    ]
    .into_iter()
    .filter_map(|name| {
        font_dirs()
            .into_iter()
            .map(|dir| dir.join(name))
            .find(|path| path.is_file())
    })
    .collect()
}
