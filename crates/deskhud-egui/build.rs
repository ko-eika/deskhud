//! 嵌入 Windows 程序图标（`assets/icon.ico` → exe 资源）。

fn main() {
    println!("cargo:rerun-if-changed=assets/icon.ico");
    println!("cargo:rerun-if-changed=assets/icon.png");

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }

    let mut res = winresource::WindowsResource::new();
    res.set_icon("assets/icon.ico");
    if let Err(e) = res.compile() {
        // 缺 rc.exe 时仍可编过；仅资源管理器图标会回退
        println!("cargo:warning=embed icon.ico failed: {e}");
    }
}
