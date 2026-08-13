<h1 align="center" style="margin: 30px 0 30px; font-weight: bold;">DeskHud</h1>
<h4 align="center">An extensible desktop pet engine</h4>
<p align="center">
	<img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="license">
    <img src="https://img.shields.io/badge/version-0.6.4-green.svg" alt="version">
    <img src="https://img.shields.io/badge/rustc-1.85+-green.svg" alt="rustc">
    <img src="https://img.shields.io/badge/egui-0.36-green.svg" alt="egui">
</p>
<p align="center">
	<img src="https://img.shields.io/badge/author-KO.EIKA-blue.svg" alt="author">
    <img src="https://img.shields.io/badge/copyright-%40KOEIKA-blue.svg" alt="copyright">
</p>

[简体中文](./README.md) | English

DeskHud is an extensible **desktop pet engine**: switch **pet packs** (look + behavior), and toggle **HUD plugins** and their contributions. The UI is built with **egui + winit / egui_glow**, with multi-language support and local community pack loading (a store comes later). Current release line: `0.6.4`.

## Features

### Pet window

- Transparent, draggable pet window; snaps when released near screen edges
- Dock / drag state is forwarded to the active pet pack (poses, feedback, etc.)
- Context menu: Settings, Quit
- Optional pet always-on-top; window size follows the active pet pack

### Settings (sidebar)

| Section | Contents |
|---------|----------|
| **General** | Theme (light / dark / system), language, UI font (family / style / size, searchable combos) |
| **Pet** | Grid or list picker; pet on top; behavior toggles for the active pet |
| **Plugins** | Grouped by plugin; master switch + per-HUD contribution switches |
| **About** | Version, author, license, homepage, and stack info |

Preferences are persisted (locale, theme, font, active pet, HUD switches, window geometry, pet behavior config, and more).

### Pet packs

- One pack = **skin assets + behavior**; switching packs switches both
- Full ID: `pet.<org>.<name>`
- Built-ins: `pet.deskhud.specs` (Big Eyes), `pet.deskhud.blob` (Blue Dot)
- Packs may declare `PetConfigOption` (boolean options) under Settings → active pet behavior
- Community target: `.deskhud` (directory or zip) + WASM (roadmap Phase 3)

### HUD plugins

- A plugin may contribute 0..N HUD items; prefs support plugin-level and item-level switches
- Full ID: `hud.<org>.<name>`
- Demo plugin is wired; real HUD frame data is still evolving

### Internationalization

- Shell copy + pet / plugin pack catalogs merge into `CatalogStore`
- Key namespaces: `shell.*` / `pet.<id>.*` / `hud.<id>.*` (pack-relative keys get a prefix on load)
- Fallback: requested locale → `en` → key name
- Changing language in Settings updates shell and loaded pack strings together

## Tech stack

| Area | Choice |
|------|--------|
| UI | egui + winit / egui_glow, sole UI toolkit; platform overlays handle transparent composition; no system tray |
| Built-in extensions | Native Rust `PetKind` / `Plugin` |
| Community extensions | WASM (wasmtime) + `deskhud-sdk` (planned) |
| Pack format | `.deskhud` + `manifest.toml` |
| Config | `serde` + TOML prefs / manifest / pack i18n |

**Out of scope for now**: plugin store, community native DLLs, plugins using egui directly, UI depending on `git2`.

## Architecture

```
deskhud-egui        Executable UI (pet window / menu / settings)
       │
       ▼
deskhud-runtime     Discover packs → load (native built-in / WASM) → register
       │
       ├── deskhud-engine      Contracts + built-in pets / demo plugins
       ├── deskhud-package   Manifest, pack I/O, pack i18n
       └── deskhud-ui        Locale, prefs, catalog merge (no egui)

deskhud-sdk         Community guest SDK (wasm32)
```

Repository layout:

```
crates/          Crate sources
packages/        Local installed / dev pack scan root
examples/        Community authoring examples
docs/            Architecture, extension guide, roadmap
```

## Requirements

- Rust **1.85+** (`rust-version` in `Cargo.toml`)
- Windows uses the native GPU overlay; macOS uses native platform windows and screen safe-area APIs; Linux remains a platform fallback (see CI and the `platform` module)

## Build & run

```bash
# Run the pet shell
cargo run -p deskhud-egui

# Check / test
cargo check --workspace
cargo test -p deskhud-package -p deskhud-ui -p deskhud-engine -p deskhud-runtime

# Export packs/ → target/packages/*.deskhud (manifest + assets + i18n)
cargo pack-builtins
cargo pack-builtin pet-deskhud-specs

# Format & lint
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```

After the first run, preferences live under the user config area (on Windows, typically under `%APPDATA%/DeskHud`). Local packs can go in [`packages/`](./packages/) or the user packages directory; see that folder’s README.

## Release

Full checklist: [`docs/release.md`](./docs/release.md). Summary:

```bash
# 1. Bump workspace.package.version in the root Cargo.toml; sync README badges
# 2. Verify
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p deskhud-package -p deskhud-ui -p deskhud-engine -p deskhud-runtime --all-targets

# 3. Release build (on the target OS)
cargo build -p deskhud-egui --release
# Windows: target/release/deskhud.exe
# macOS / Linux: target/release/deskhud

# 4. Tag, push, and attach binaries on a GitHub Release
git tag -a v0.6.4 -m "DeskHud 0.6.4"
git push origin v0.6.4
```

CI ([`.github/workflows/ci.yml`](./.github/workflows/ci.yml)) runs cross-platform `check` / tests only; it does **not** publish installers yet.

## Quick start

1. Launch, drag the pet; right-click for Settings or Quit.
2. **Settings → Pet**: pick a built-in pet; enable behavior options such as key/mouse tips as needed.
3. **Settings → Plugins**: toggle the demo HUD plugin and its items.
4. **Settings → General**: theme, language, and font (default JetBrains Mono / Regular / 13).
5. **Settings → About**: see the app version (same as the workspace `version` in `Cargo.toml`, via `CARGO_PKG_VERSION` at build time).

## Extending DeskHud

For community authors:

- [`docs/extension-guide.md`](./docs/extension-guide.md) — pet / HUD contracts, events, pack layout
- [`docs/architecture.md`](./docs/architecture.md) — crate boundaries and dependency direction
- [`packages/README.md`](./packages/README.md) — where to put local packs

Typical pack layout:

```text
my-cool-pet.deskhud/
  manifest.toml
  guest.wasm          # community pack (planned)
  assets/
  i18n/
    zh-CN.toml
    en.toml
```

## Docs

| Doc | Description |
|-----|-------------|
| [`README.md`](./README.md) | Chinese README |
| [`AGENTS.md`](./AGENTS.md) | Collaborator / agent handbook (single entry) |
| [`docs/agent/`](./docs/agent/README.md) | Agent index; CONSTRAINTS + MEMORY |
| [`docs/architecture.md`](./docs/architecture.md) | Architecture |
| [`docs/extension-guide.md`](./docs/extension-guide.md) | Extension guide |
| [`docs/roadmap.md`](./docs/roadmap.md) | Roadmap |
| [`docs/release.md`](./docs/release.md) | Release & build artifacts |

## License

Licensed under the [Apache License 2.0](./LICENSE).

Bundled fonts (Noto Sans SC, JetBrains Mono) are SIL OFL 1.1; see [`NOTICE`](./NOTICE) and `crates/deskhud-egui/assets/fonts/`.

Copyright © KO.EIKA / @KOEIKA
