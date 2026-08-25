# 本地包目录

宿主通过 [`deskhud-runtime`](../crates/core/deskhud-runtime) 扫描本目录（以及 `%APPDATA%/DeskHud/packages`）。

每个子目录或 `.deskhud` 归档为一个包根，至少包含：

```text
my-pack/                 # 或 my-pack.deskhud（zip）
  manifest.toml
  guest.wasm             # 社区 WASM 包（Phase 3）
  i18n/                  # 可选
    zh-CN.toml
    en.toml
  assets/                # 可选皮肤 / 图标
```

## 用出厂包做扫描验证

仓库 [`packs/`](../crates/packs/) 是 compile-in 源；可导出对照用 `.deskhud` 再放到本目录：

```bash
# 在仓库根
cargo pack-builtins
# 产物：target/packages/*.deskhud

# 拷到本目录后重启 / 再跑宿主即可被扫描
# 例（PowerShell）：
Copy-Item target/packages/*.deskhud packages/
```

单个包：

```bash
cargo pack-builtin pet-deskhud-specs
cargo pack-builtin hud-deskhud-demo
```

说明见 [`docs/extension-guide.md`](../docs/extension-guide.md) §1.6、[`docs/release.md`](../docs/release.md)。
