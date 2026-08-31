//! DeskHud xtask：内置包导出等开发命令。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use deskhud_package::{PackCatalog, pack_directory, read_manifest_dir};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../../../.."))
}

fn packs_dir(root: &Path) -> PathBuf {
    root.join("crates/packs")
}

fn default_out(root: &Path, args: &[String]) -> PathBuf {
    if args.iter().any(|arg| arg == "--release") {
        return root.join("target/release/packages");
    }
    env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf))
        .map(|profile_dir| profile_dir.join("packages"))
        .unwrap_or_else(|| root.join("target/packages"))
}

fn usage() {
    eprintln!(
        "usage:
  cargo pack-builtins [--out DIR]
  cargo pack-builtin <crate-dir-name> [--out DIR]
  cargo pack-external [<DIR>] [--release] [--out DIR]

Builds external WASM Components, then packs manifest.toml + entry + assets/ + i18n/ into .deskhud.
Without DIR, pack-external packs every external package under packs/.
It requires manifest.toml load = \"external\" (or a legacy entry) and is
intended for community WASM packages."
    );
}

fn copy_pack_tree(src: &Path, dest: &Path, generated_entry: Option<&Path>) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let manifest = src.join("manifest.toml");
    if !manifest.is_file() {
        return Err(format!("missing manifest.toml in {}", src.display()));
    }
    fs::write(
        dest.join("manifest.toml"),
        fs::read(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let manifest_data = read_manifest_dir(src).map_err(|e| e.to_string())?;
    if let Some(entry) = manifest_data.entry {
        let source_entry = src.join(&entry);
        let from = generated_entry.unwrap_or(&source_entry);
        if !from.is_file() {
            return Err(format!("missing manifest entry {}", source_entry.display()));
        }
        let to = dest.join(&entry);
        if let Some(parent) = to.parent() {
            fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        fs::write(to, fs::read(from).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    }
    for name in ["assets", "i18n"] {
        let from = src.join(name);
        if !from.is_dir() {
            continue;
        }
        copy_dir_recursive(&from, &dest.join(name))?;
    }
    Ok(())
}

fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    for entry in fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        let name = entry.file_name();
        let to = dest.join(&name);
        if path.is_dir() {
            copy_dir_recursive(&path, &to)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("po")
            && src.components().any(|c| c.as_os_str() == "i18n")
        {
            let catalog = PackCatalog::parse_gettext(&fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| format!("parse {}: {e}", path.display()))?;
            let mo = to.with_extension("mo");
            fs::write(mo, catalog.to_mo()).map_err(|e| e.to_string())?;
        } else {
            fs::write(&to, fs::read(&path).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn package_name(src_crate: &Path) -> Result<String, String> {
    let cargo_toml = fs::read_to_string(src_crate.join("Cargo.toml")).map_err(|e| e.to_string())?;
    let in_package = cargo_toml
        .lines()
        .scan(false, |seen, line| {
            if line.trim() == "[package]" {
                *seen = true;
            } else if line.starts_with('[') {
                *seen = false;
            }
            Some((*seen, line))
        })
        .find_map(|(seen, line)| {
            if !seen || !line.trim_start().starts_with("name") {
                return None;
            }
            line.split_once('=')
                .map(|(_, value)| value.trim().trim_matches('"').to_owned())
        });
    in_package.ok_or_else(|| format!("missing [package].name in {}", src_crate.display()))
}

fn build_external_component(
    root: &Path,
    src_crate: &Path,
    out_file: &Path,
    release: bool,
) -> Result<(), String> {
    let name = package_name(src_crate)?;
    let mut build = Command::new("cargo");
    // `pack-external` is itself launched by Cargo. Use a separate target
    // directory so the nested guest build does not wait on Cargo's outer
    // artifact lock forever.
    let guest_target = root.join("target/deskhud-guest");
    build
        .current_dir(root)
        .env_remove("CARGO_MAKEFLAGS")
        .args(["build", "-p"])
        .arg(&name)
        .args(["--target", "wasm32-unknown-unknown", "--target-dir"])
        .arg(&guest_target);
    if release {
        build.arg("--release");
    }
    let output = build
        .output()
        .map_err(|e| format!("failed to run cargo build: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "cargo build for {name} failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    let profile = if release { "release" } else { "debug" };
    let wasm = root
        .join("target/deskhud-guest/wasm32-unknown-unknown")
        .join(profile)
        .join(format!("{}.wasm", name.replace('-', "_")));
    if !wasm.is_file() {
        return Err(format!("cargo build did not produce {}", wasm.display()));
    }
    if let Some(parent) = out_file.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let output = Command::new("wasm-tools")
        .current_dir(root)
        .args(["component", "new"])
        .arg(&wasm)
        .args(["-o"])
        .arg(out_file)
        .output()
        .map_err(|e| format!("failed to run wasm-tools: {e}"))?;
    if !output.status.success() {
        return Err(format!(
            "wasm-tools component new failed:\n{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    Ok(())
}

fn pack_one(
    root: &Path,
    src_crate: &Path,
    out_dir: &Path,
    release: bool,
) -> Result<PathBuf, String> {
    let manifest_data = read_manifest_dir(src_crate)
        .map_err(|e| format!("read manifest in {}: {e}", src_crate.display()))?;
    let staging = out_dir.join(".staging").join(
        src_crate
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pack"),
    );
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }
    let generated_entry = if manifest_data.is_external() {
        manifest_data
            .entry
            .as_deref()
            .map(|entry| {
                let generated = staging.join(entry);
                build_external_component(root, src_crate, &generated, release).map(|_| generated)
            })
            .transpose()?
    } else {
        None
    };
    copy_pack_tree(src_crate, &staging, generated_entry.as_deref())
        .map_err(|e| format!("stage {}: {e}", src_crate.display()))?;
    let manifest = read_manifest_dir(&staging).map_err(|e| format!("read staged manifest: {e}"))?;
    let dest = out_dir.join(format!("{}.deskhud", manifest.id));
    pack_directory(&staging, &dest).map_err(|e| e.to_string())?;
    let _ = fs::remove_dir_all(out_dir.join(".staging"));
    Ok(dest)
}

fn list_pack_crates(packs: &Path) -> Result<Vec<PathBuf>, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(packs).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_dir() && path.join("manifest.toml").is_file() {
            out.push(path);
        }
    }
    out.sort();
    Ok(out)
}

fn list_external_pack_crates(packs: &Path) -> Result<Vec<PathBuf>, String> {
    list_pack_crates(packs)?
        .into_iter()
        .try_fold(Vec::new(), |mut external, path| {
            let manifest = read_manifest_dir(&path).map_err(|e| e.to_string())?;
            if manifest.is_external() {
                external.push(path);
            }
            Ok(external)
        })
}

fn list_builtin_pack_crates(packs: &Path) -> Result<Vec<PathBuf>, String> {
    list_pack_crates(packs)?
        .into_iter()
        .try_fold(Vec::new(), |mut builtins, path| {
            let manifest = read_manifest_dir(&path).map_err(|e| e.to_string())?;
            if !manifest.is_external() {
                builtins.push(path);
            }
            Ok(builtins)
        })
}

fn external_source(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.exists() {
        path
    } else {
        root.join("crates/packs").join(value)
    }
}

fn requested_external_source(root: &Path, args: &[String]) -> Option<PathBuf> {
    let mut skip_next = false;
    for value in args.iter().skip(1) {
        if skip_next {
            skip_next = false;
            continue;
        }
        if value == "--out" {
            skip_next = true;
            continue;
        }
        if !value.starts_with('-') {
            return Some(external_source(root, value));
        }
    }
    None
}

fn parse_out(args: &[String]) -> PathBuf {
    let root = workspace_root();
    args.windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| default_out(&root, args))
}

fn is_release(args: &[String]) -> bool {
    args.iter().any(|arg| arg == "--release")
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.is_empty() || args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
        return ExitCode::FAILURE;
    }
    let root = workspace_root();
    let packs = packs_dir(&root);
    let cmd = args[0].as_str();
    let result = match cmd {
        "pack-builtins" => {
            let out = parse_out(&args);
            let release = is_release(&args);
            fs::create_dir_all(&out).expect("create out");
            let crates = list_builtin_pack_crates(&packs).expect("list packs");
            if crates.is_empty() {
                eprintln!("no builtin packs under {}", packs.display());
                return ExitCode::FAILURE;
            }
            for c in crates {
                match pack_one(&root, &c, &out, release) {
                    Ok(dest) => println!("packed {}", dest.display()),
                    Err(e) => {
                        eprintln!("failed {}: {e}", c.display());
                        return ExitCode::FAILURE;
                    }
                }
            }
            Ok(())
        }
        "pack-external" => {
            let out = parse_out(&args);
            let release = is_release(&args);
            fs::create_dir_all(&out).expect("create out");
            let sources = if let Some(src) = requested_external_source(&root, &args) {
                vec![src]
            } else {
                match list_external_pack_crates(&packs) {
                    Ok(sources) => sources,
                    Err(error) => {
                        eprintln!("failed to scan external packs: {error}");
                        return ExitCode::FAILURE;
                    }
                }
            };
            if sources.is_empty() {
                eprintln!(
                    "no external packs with load = `external` under {}",
                    packs.display()
                );
                return ExitCode::FAILURE;
            }
            for src in sources {
                match read_manifest_dir(&src) {
                    Ok(manifest) if manifest.is_external() => {
                        match pack_one(&root, &src, &out, release) {
                            Ok(dest) => println!("packed external {}", dest.display()),
                            Err(e) => {
                                eprintln!("failed {}: {e}", src.display());
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    Ok(_) => {
                        eprintln!(
                            "{} is not an external package: manifest.toml must declare load = \"external\"",
                            src.display()
                        );
                        return ExitCode::FAILURE;
                    }
                    Err(e) => {
                        eprintln!("failed {}: {e}", src.display());
                        return ExitCode::FAILURE;
                    }
                }
            }
            Ok(())
        }
        "pack-builtin" => {
            let name = args.get(1).cloned().unwrap_or_default();
            if name.is_empty() || name.starts_with('-') {
                usage();
                return ExitCode::FAILURE;
            }
            let out = parse_out(&args);
            let release = is_release(&args);
            fs::create_dir_all(&out).expect("create out");
            let src = packs.join(&name);
            match pack_one(&root, &src, &out, release) {
                Ok(dest) => {
                    println!("packed {}", dest.display());
                    Ok(())
                }
                Err(e) => {
                    eprintln!("{e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        _ => {
            usage();
            return ExitCode::FAILURE;
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(()) => ExitCode::FAILURE,
    }
}
