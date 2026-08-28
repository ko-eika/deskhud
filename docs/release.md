# 发布指南

DeskHud 当前 **没有** 自动打安装包的 GitHub Release workflow；发布以本地 `cargo build --release` 产物为主，再用 git tag / GitHub Release 归档。

## 版本号

版本以根目录 [`Cargo.toml`](../Cargo.toml) 的 `workspace.package.version` 为准（设置「关于」页通过 `CARGO_PKG_VERSION` 注入）。

发版前请同步：

1. `Cargo.toml` → `[workspace.package] version`
2. [`README.md`](../README.md) / [`README_EN.md`](../README_EN.md) 中的 version 徽章
3. （可选）`CHANGELOG` / Release 说明正文

### 0.7.0 变更摘要

- 完成阶段 F 社区 WASM Guest：外部宠物包通过 Wasmtime Component Model 加载并纳入运行时注册表。
- `cargo pack-external` 自动编译 WASM、生成 Component 并打包到 `target/<profile>/packages/`，源码目录不再保存 `guest.wasm`。
- 外部宠物包支持通过 Guest 元数据提供设置预览图与布尔配置项；蓝点改为外部 WASM 包。
- WASM Guest ABI 增加配置项元数据，`api_version` 升至 2，兼容族升至 `0.7`。

### 0.6.26 变更摘要

- 修复 Linux、macOS 的 workspace 跨平台编译：Windows 原生依赖仅在 Windows target 引入。
- 统一 Source Han Sans 的字体家族 ID，修复 Windows 字体扫描测试失败。
- 将项目最低 Rust 版本与当前依赖同步为 1.95，并同步 CI 工具链。

## 发布前检查

```bash
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p deskhud-package -p deskhud-ui -p deskhud-engine -p deskhud-runtime --all-targets
cargo check --workspace --all-targets
```

推送到 `main` / `master` 后，[`.github/workflows/ci.yml`](../.github/workflows/ci.yml) 会在 Windows / Ubuntu / macOS 上跑 `check` 与部分测试。

## 构建发行二进制

在目标平台上执行：

```bash
cargo build -p deskhud-egui --release
```

产物路径：

| 平台 | 路径 |
|------|------|
| Windows | `target/release/deskhud.exe` |
| macOS / Linux | `target/release/deskhud` |

说明：

- 应用图标按平台处理：Windows 构建脚本通过 `winresource` 将 `assets/icon.ico` 嵌入 exe，资源管理器和任务栏使用 exe 图标；Windows、Linux 窗口使用编译时嵌入的 `assets/icon.png`，因此窗口和任务栏不依赖发布目录中的图标文件；macOS 在主线程将 `assets/icon.icns` 设置到 `NSApplication`，保证未打成 `.app` 的 cargo 二进制和已打包应用的 Dock 图标一致。
- `cargo build` 产出的是 `target/release/deskhud-egui`（Windows 为 `.exe`）；它不会自动生成 macOS `.app` 或 Linux `.desktop` 安装包。制作这类原生安装包时，应将 `assets/icon.icns` 与 macOS 应用包的原生元数据一起安装。运行中的窗口图标仍由程序自身提供。
- macOS 本地打包可执行 `bash scripts/package-macos.sh`，生成 `target/release/DeskHud.app` 和 `target/release/DeskHud-macos.dmg`；使用 `--skip-build` 可复用已经生成的 release 二进制。该脚本同时供后续 GitHub Actions 发布 workflow 调用，不依赖第三方打包工具。
- 字体不嵌入可执行文件；Cargo 构建会将 `assets/fonts/` 递归复制到 `target/<profile>/fonts/`。macOS 打包脚本会将其放入 `.app/Contents/Resources/fonts/`，裸二进制仍从可执行文件旁的 `fonts/` 目录读取。应用支持按字簇、语言和样式自由分层，缺失外置字体时回退到系统字体。
- 设置窗口暂使用 **Glow（OpenGL）**；Windows 宠物、菜单、气泡和 HUD 原生合成使用 D3D11 + Direct2D + DirectComposition。设置窗不承担透明覆盖层职责。
- 当前体验最完整的目标平台是 **Windows**；macOS/Linux 的原生窗口后端按迁移里程碑推进，fallback 仅作为能力不足时的明确降级。

### 可选：体积与符号

根 `Cargo.toml` 已配置 release：`lto = "thin"`、`codegen-units = 1`、`strip = "symbols"`。一般无需额外 strip。

## 打标签与 GitHub Release（建议流程）

假设版本为 `0.4.1`：

```bash
# 1. 提交版本 bump 与说明文档更新
git add -A
git commit -m "chore: release 0.4.1"

# 2. 打 annotated tag 并推送
git tag -a v0.4.1 -m "DeskHud 0.4.1"
git push origin HEAD
git push origin v0.4.1

# 3. 在 GitHub 创建 Release，上传对应平台的 deskhud 二进制
#    （可附简短变更说明与校验和）
```

上传前可为产物生成校验：

```bash
# Windows (PowerShell)
Get-FileHash target\release\deskhud.exe -Algorithm SHA256

# macOS / Linux
shasum -a 256 target/release/deskhud
```

## 尚未自动化（后续可做）

- 按 tag 触发多平台 `cargo build --release` 并上传 Artifact / Release
- 安装器（如 MSI / NSIS）与代码签名
- 自动更新通道

有需要时可在 `.github/workflows/` 增加 `release.yml`；在此之前请按上文手动构建与归档。

## 导出内置参考包（`.deskhud`）

从 [`crates/packs/`](../crates/packs/) 导出对照用包（仅 `manifest.toml` + `assets/` + `i18n/`；原生实现仍 compile-in）：

```bash
# 全部 → target/packages/*.deskhud
cargo pack-builtins

# 单个（参数为 crates/packs/ 下目录名）
cargo pack-builtin pet-deskhud-specs
cargo pack-builtin pet-deskhud-blob
cargo pack-builtin hud-deskhud-demo

# 指定输出目录
cargo pack-builtins --out dist/my-packs
```

可将产物拷到 [`packages/`](../packages/) 做扫描侧验证。更多上下文见 [`docs/extension-guide.md`](./extension-guide.md) §1.6。
