# DeskHud 版本与适配政策

DeskHud 区分三类版本，避免混用：

当前发布线为 `0.9.5`：包兼容族为 `engine = "0.9"`，Guest / WIT 契约为
`api_version = 4`。`0.8.x` 包应继续由对应的 `0.8` 运行时加载，不应直接混用到
`0.9` 兼容族。

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
engine = "0.4"       # 适配族
api_version = 4      # Guest/契约 ABI
```

## `engine` 族匹配

- 引擎处于 **`0.x`**：包的 `engine` 必须等于当前产品的 `MAJOR.MINOR`（例：引擎 `0.4.1` 只接受 `engine = "0.4"`）
- 引擎 **`≥ 1.0.0`**：包的 `engine` 只需匹配 **MAJOR**（例：引擎 `1.4.2` 接受 `engine = "1"`）
- 另：`api_version` 必须属于引擎当前支持的 ABI 集合

不匹配时引擎**拒绝注册**该包；设置页应可见并标明不适配原因。包自己的 `version` **不参与**适配门闸。

## 内置包

内置宠物 / 插件与社区包使用同一套 manifest 字段；运行时以原生 crate **compile-in**，也可用 `cargo pack-builtins` / `cargo pack-builtin <dir>` 从 [`packs/`](../packs/) 导出 `.deskhud` 做规范校验（不嵌入可执行文件加载路径）。完整命令见 [`extension-guide.md`](./extension-guide.md) §1.6 与 [`release.md`](./release.md)。

## 版本提交描述

版本提交描述使用 Conventional Commits 风格，并统一使用中文正文，便于发布记录、变更追踪与复制使用：

```yaml
feat: DeskHud 0.x.y — 简短主题

- 使用动词开头，描述一个可验证的用户可见变更
- 每条只描述一个变更点，避免把多个模块揉成一句
- 优先说明运行时行为、配置迁移、兼容性、国际化和内置包版本等影响
- 不写实现过程、测试过程或无法从代码验证的宣传性描述
```

标题类型按变更性质选择：功能与体验改进使用 `feat:`，问题修复使用 `fix:`，纯版本号或发版元数据调整才使用 `release:`。包含多类变更时，优先使用主要用户价值对应的类型。标题必须包含目标版本，正文条目使用中文并保持简洁、平行、可核对。
