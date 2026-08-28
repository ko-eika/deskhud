use std::path::PathBuf;

pub(crate) fn font_dirs() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts",
        "/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
    ]
    .into_iter()
    .map(PathBuf::from)
    .collect()
}

pub(crate) fn priority_system_cjk() -> Vec<PathBuf> {
    [
        "/System/Library/Fonts/PingFang.ttc",
        "/System/Library/Fonts/STHeiti Light.ttc",
        "/System/Library/Fonts/Hiragino Sans GB.ttc",
    ]
    .into_iter()
    .map(PathBuf::from)
    .filter(|path| path.is_file())
    .collect()
}
