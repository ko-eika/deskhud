fn main() {
    #[cfg(windows)]
    if let Err(error) = deskhud_platform_windows::run_blank_window("DeskHud") {
        eprintln!("DeskHud failed to start native shell: {error}");
        std::process::exit(1);
    }

    #[cfg(target_os = "macos")]
    if let Err(error) = deskhud_platform_macos::run_blank_window("DeskHud") {
        eprintln!("DeskHud failed to start native shell: {error}");
        std::process::exit(1);
    }

    #[cfg(target_os = "linux")]
    if let Err(error) = deskhud_platform_linux_gtk::run_blank_window("DeskHud") {
        eprintln!("DeskHud failed to start native shell: {error}");
        std::process::exit(1);
    }
}
