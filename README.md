<h1 align="center" style="margin: 30px 0 30px; font-weight: bold;">DeskHud</h1>
<h4 align="center">一个可扩展的桌宠引擎</h4>
<p align="center">
	<img src="https://img.shields.io/badge/license-Apache%202.0-blue.svg" alt="license">
    <img src="https://img.shields.io/badge/version-0.9.2-green.svg" alt="version">
    <img src="https://img.shields.io/badge/rustc-1.95+-green.svg" alt="rustc">
    <img src="https://img.shields.io/badge/egui-0.36-green.svg" alt="egui">
</p>
<p align="center">
	<img src="https://img.shields.io/badge/author-KO.EIKA-blue.svg" alt="author">
    <img src="https://img.shields.io/badge/copyright-%40KOEIKA-blue.svg" alt="copyright">
</p>

简体中文 | [English](./README_EN.md)

DeskHud 是可扩展的 **桌宠引擎**：用户可以切换 **宠物包**（外观 + 行为），并按需开关 **HUD 插件** 及其贡献条目。界面基于 **egui + winit / egui_glow**，支持多语言与本地社区包加载（商店能力后置）。

当前版本：`0.9.2`。增强 HUD 布局编辑，支持独立宽高、网格吸附、比例锁定和视觉参数调整。

## 功能概览

### 桌宠窗口

- 透明宠窗，可拖动；松手后靠近屏幕边缘时吸附
- 贴边 / 拖拽状态会通知当前宠物包（便于做姿势、反馈）
- 右键菜单：设置、置顶、插件开关、插件层级、插件布局与退出
- 可选宠物置顶；窗口尺寸跟随当前宠物包

### 设置（侧栏）

| 分区 | 内容 |
|------|------|
| **常规** | 主题（浅色 / 深色 / 跟随系统）、语言、界面字体（系列 / 样式 / 大小，含可搜索下拉） |
| **宠物** | 网格或列表挑选宠物；宠物置顶；当前宠物的行为开关 |
| **插件** | 按插件折叠；总开关 + 单条 HUD 开关 |
| **关于** | 版本号、作者、许可证、主页与技术栈信息 |

偏好会持久化（语言、主题、字体、活动宠物、HUD 开关、窗口几何、宠行为配置等）。

### 宠物包

- 一套包 = **皮肤资源 + 行为逻辑**；切换包即切换外观与行为
- 全 ID 约定：`pet.<组织>.<标识>`
- 内置宠物：`pet.deskhud.mochi`（糯米团）、`pet.deskhud.sesame`（芝麻豆）
- 包可声明 `PetConfigOption`（布尔行为项），在设置「当前宠物行为」中配置
- 社区包目标形态：`.deskhud`（目录或 zip）+ WASM（路线图 Phase 3）

### HUD 插件

- 插件可贡献 0..N 条 HUD；prefs 支持插件总开关与条目开关
- 全 ID：`hud.<组织>.<标识>`
- 运行态按“总开关 ∧ 插件开关 ∧ 条目开关”扫描贡献并绘制插件 `HudFrame`
- 内置 `hud.deskhud.demo` 提供时钟和提示两条演示 HUD

### 国际化

- 外壳固定文案 + 宠 / 插件包文案合并进 `CatalogStore`
- 键命名空间：`shell.*` / `pet.<id>.*` / `hud.<id>.*`（包内相对键加载时加前缀）
- 回退：当前语言 → `en` → 键名本身
- 设置中切换语言后，壳与已加载包文案一并生效

## 技术栈

| 领域 | 选择 |
|------|------|
| UI | 原生平台窗口 + 设置页 egui；透明合成由平台覆盖层负责；无系统托盘 |
| 内置扩展 | 原生 Rust `PetKind` / `Plugin` |
| 社区扩展 | WASM（wasmtime）+ `deskhud-sdk`（规划中） |
| 包格式 | `.deskhud` + `manifest.toml` |
| 配置 | `serde` + TOML prefs / manifest；包内 i18n 使用 gettext PO/MO |

**本阶段不做**：插件商店、社区原生 dll、插件直接使用 egui、UI 依赖 `git2`。

## 架构

```
deskhud-egui        可执行 UI 壳（平台窗口编排 / 设置）
       │
       ▼
deskhud-runtime     本地发现包 → 加载（原生内置 / WASM）→ 注册
       │
       ├── deskhud-engine      契约 + 内置宠 / 演示插件
       ├── deskhud-package   manifest、包 IO、包内 i18n
       └── deskhud-ui        Locale、prefs、目录合并（零 egui）

deskhud-sdk         社区 Guest SDK（编译为 wasm32）
```

仓库主要目录：

```
crates/core/     通用业务 crate（engine、runtime、package、ui、sdk）
crates/apps/     应用入口（deskhud-egui、deskhud-native）
crates/platform/ 平台抽象与 Windows/macOS/Linux 实现
crates/packs/    内置宠物包与 HUD 包
crates/tools/    构建与打包工具
docs/            架构、扩展指南、路线图
```

## 环境要求

- Rust **1.95+**（`Cargo.toml` 中 `rust-version`）
- Windows 使用原生 GPU 覆盖层；macOS/Linux 原生窗口后端按 [`docs/agent/roadmap/README.md`](./docs/agent/roadmap/README.md) 推进，fallback 仅表示明确降级

## 构建与运行

```bash
# 运行桌宠
cargo run -p deskhud-egui

# 检查 / 测试
cargo check --workspace
cargo test -p deskhud-package -p deskhud-ui -p deskhud-engine -p deskhud-runtime

# 导出 crates/packs/ → target/packages/*.deskhud（manifest + assets + i18n）
cargo pack-builtins
cargo pack-builtin pet-deskhud-mochi

# 格式与静态检查
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```

首次运行后，偏好与数据目录由引擎写入用户配置区（Windows 上通常在 `%APPDATA%/DeskHud` 一带）。构建产物位于 `target/<profile>/packages/`；长期安装的本地包应放在用户 packages 目录。

## 发布

完整清单见 [`docs/release.md`](./docs/release.md)。摘要：

```bash
# 1. 修改根 Cargo.toml 的 workspace.package.version，并同步 README 徽章
# 2. 检查
cargo clippy --workspace --all-targets -- -D warnings
cargo test -p deskhud-package -p deskhud-ui -p deskhud-engine -p deskhud-runtime --all-targets

# 3. 发行构建（在目标平台上）
cargo build -p deskhud-egui --release
# Windows: target/release/deskhud-egui.exe
# macOS / Linux: target/release/deskhud-egui

# 4. 打 tag 并推送后，在 GitHub Release 上传二进制
git tag -a v0.9.2 -m "DeskHud 0.9.2"
git push origin v0.9.2
```

当前 CI（[`.github/workflows/ci.yml`](./.github/workflows/ci.yml)）只做三端 `check` / 测试，**不会**自动发布安装包。

## 使用提示

1. 启动后拖动桌宠；右键打开设置或退出。
2. **设置 → 宠物**：选择内置宠，按需打开「键鼠短提示」等行为项。
3. **设置 → 插件**：开关演示 HUD 插件及条目。
4. **设置 → 常规**：调整主题、语言与字体（默认 Inter / Regular / 13）。
5. **设置 → 关于**：查看当前应用版本（编译时自 `CARGO_PKG_VERSION` 注入，对应 workspace `version`）。

## 扩展开发

社区作者请阅读：

- [`docs/extension-guide.md`](./docs/extension-guide.md) — 宠物包 / HUD 插件契约、事件、**导出 `.deskhud`**
- [`docs/architecture.md`](./docs/architecture.md) — crate 边界与依赖方向
- [`docs/release.md`](./docs/release.md) — 发版与 `cargo pack-builtins` 说明

包内典型结构：

```text
my-cool-pet.deskhud/
  manifest.toml
  guest.wasm          # 社区包（规划）
  assets/
  i18n/
    zh-CN/
      info.mo
      config.mo
    en-US/
      info.mo
      config.mo
```

## 文档索引

| 文档 | 说明 |
|------|------|
| [`README_EN.md`](./README_EN.md) | English README |
| [`AGENTS.md`](./AGENTS.md) | 协作者 / Agent 工作手册（唯一入口） |
| [`docs/agent/`](./docs/agent/README.md) | Agent 索引；含 CONSTRAINTS / MEMORY |
| [`docs/architecture.md`](./docs/architecture.md) | 架构 |
| [`docs/extension-guide.md`](./docs/extension-guide.md) | 扩展指南 |
| [`docs/roadmap.md`](./docs/roadmap.md) | 路线图 |
| [`docs/release.md`](./docs/release.md) | 发版与构建产物 |

## 许可证

本项目采用 [Apache License 2.0](./LICENSE)。

开发构建只嵌入一个字体文件：[`assets/fonts/Inter.ttc`](./assets/fonts/Inter.ttc)。

Copyright © KO.EIKA / @KOEIKA
