//! DeskHud xtask：内置包导出等开发命令。

use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use deskhud_package::{pack_directory, read_manifest_dir};

fn workspace_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn packs_dir(root: &Path) -> PathBuf {
    root.join("packs")
}

fn default_out(root: &Path) -> PathBuf {
    root.join("target/packages")
}

fn usage() {
    eprintln!(
        "usage:
  cargo pack-builtins [--out DIR]
  cargo pack-builtin <crate-dir-name> [--out DIR]

Packs only manifest.toml + assets/ + i18n/ into .deskhud (no src/Cargo.toml).
Sources live under packs/ (pet-* / hud-*)."
    );
}

fn copy_pack_tree(src: &Path, dest: &Path) -> Result<(), String> {
    fs::create_dir_all(dest).map_err(|e| e.to_string())?;
    let manifest = src.join("manifest.toml");
    if !manifest.is_file() {
        return Err(format!("missing manifest.toml in {}", src.display()));
    }
    fs::copy(&manifest, dest.join("manifest.toml")).map_err(|e| e.to_string())?;
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
        } else {
            fs::copy(&path, &to).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

fn pack_one(src_crate: &Path, out_dir: &Path) -> Result<PathBuf, String> {
    let _ = read_manifest_dir(src_crate).map_err(|e| e.to_string())?;
    let staging = out_dir.join(".staging").join(
        src_crate
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("pack"),
    );
    if staging.exists() {
        fs::remove_dir_all(&staging).map_err(|e| e.to_string())?;
    }
    copy_pack_tree(src_crate, &staging)?;
    let manifest = read_manifest_dir(&staging).map_err(|e| e.to_string())?;
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

fn parse_out(args: &[String]) -> PathBuf {
    let root = workspace_root();
    args.windows(2)
        .find(|w| w[0] == "--out")
        .map(|w| PathBuf::from(&w[1]))
        .unwrap_or_else(|| default_out(&root))
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
            fs::create_dir_all(&out).expect("create out");
            let crates = list_pack_crates(&packs).expect("list packs");
            if crates.is_empty() {
                eprintln!("no packs with manifest.toml under {}", packs.display());
                return ExitCode::FAILURE;
            }
            for c in crates {
                match pack_one(&c, &out) {
                    Ok(dest) => println!("packed {}", dest.display()),
                    Err(e) => {
                        eprintln!("failed {}: {e}", c.display());
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
            fs::create_dir_all(&out).expect("create out");
            let src = packs.join(&name);
            match pack_one(&src, &out) {
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
