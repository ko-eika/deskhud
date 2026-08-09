# DeskHud 版本与适配政策

DeskHud 区分三类版本，避免混用：

| 名称 | 谁拥有 | 作用 |
|------|--------|------|
| **引擎产品版本** | DeskHud 自身（`Cargo.toml` / 关于页） | SemVer 发版号 |
| **包 `version`** | 每个宠物包 / HUD 插件 | 包自身 SemVer；仅展示与更新比较 |
| **包 `engine`** | 每个包声明 | 适配的引擎**兼容族**；加载门闸 |
| **包 `api_version`** | 每个包声明 | Guest / 契约 ABI 整数；帧与 trait 破坏时递增 |

## 引擎产品 SemVer

### `0.x.y`（基础期）

- **PATCH**（`0.x.n`）：修 bug、文案、不影响包加载与契约 → 包的 `engine` **无需**修改
- **MINOR**（`0.n.0`）：破坏或改变包必须感知的行为 / 契约 → 换兼容族；旧 `engine` 的包**无法**适配
- **`1.0.0`**：宣布稳定期开始（合适契机，非随意跳号）

### `1.x.y`（正式期）

- **PATCH**：修 bug，不改适配
- **MINOR**：只加法（新事件、可选帧字段）；`engine = "1"` 的旧包仍可运行
- **MAJOR**：破坏性变更；需新的 `engine` 族

## 包清单字段

```toml
id = "pet.community.cool_cat"
kind = "pet"
version = "1.0.3"    # 包自身
engine = "0.3"       # 适配族
api_version = 1      # Guest/契约 ABI
```

## `engine` 族匹配

- 引擎处于 **`0.x`**：包的 `engine` 必须等于当前产品的 `MAJOR.MINOR`（例：引擎 `0.3.5` 只接受 `engine = "0.3"`）
- 引擎 **`≥ 1.0.0`**：包的 `engine` 只需匹配 **MAJOR**（例：引擎 `1.4.2` 接受 `engine = "1"`）
- 另：`api_version` 必须属于引擎当前支持的 ABI 集合

不匹配时引擎**拒绝注册**该包；设置页应可见并标明不适配原因。包自己的 `version` **不参与**适配门闸。

## 内置包

内置宠物 / 插件与社区包使用同一套 manifest 字段；运行时以原生 crate **compile-in**，也可用 `cargo pack-builtins` / `cargo pack-builtin <dir>` 从 [`packs/`](../packs/) 导出 `.deskhud` 做规范校验（不嵌入可执行文件加载路径）。完整命令见 [`extension-guide.md`](./extension-guide.md) §1.6 与 [`release.md`](./release.md)。
