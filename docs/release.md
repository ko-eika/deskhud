# 发布指南

DeskHud 当前 **没有** 自动打安装包的 GitHub Release workflow；发布以本地 `cargo build --release` 产物为主，再用 git tag / GitHub Release 归档。

## 版本号

版本以根目录 [`Cargo.toml`](../Cargo.toml) 的 `workspace.package.version` 为准（设置「关于」页通过 `CARGO_PKG_VERSION` 注入）。

发版前请同步：

1. `Cargo.toml` → `[workspace.package] version`
2. [`README.md`](../README.md) / [`README_EN.md`](../README_EN.md) 中的 version 徽章
3. （可选）`CHANGELOG` / Release 说明正文

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

- Windows 会通过 `winresource` 嵌入 `assets/icon.ico`；release 构建无控制台窗口。
- 内置 JetBrains Mono + Noto Sans SC 全字重会使体积明显变大，属预期。
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

从仓库根 [`packs/`](../packs/) 导出对照用包（仅 `manifest.toml` + `assets/` + `i18n/`；原生实现仍 compile-in）：

```bash
# 全部 → target/packages/*.deskhud
cargo pack-builtins

# 单个（参数为 packs/ 下目录名）
cargo pack-builtin pet-deskhud-specs
cargo pack-builtin pet-deskhud-blob
cargo pack-builtin hud-deskhud-demo

# 指定输出目录
cargo pack-builtins --out dist/my-packs
```

可将产物拷到 [`packages/`](../packages/) 做扫描侧验证。更多上下文见 [`docs/extension-guide.md`](./extension-guide.md) §1.6。
