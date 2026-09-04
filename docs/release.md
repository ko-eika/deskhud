# 发布指南

DeskHud 当前 **没有** 自动打安装包的 GitHub Release workflow；发布以本地 `cargo build --release` 产物为主，再用 git tag / GitHub Release 归档。

## 版本号

版本以根目录 [`Cargo.toml`](../Cargo.toml) 的 `workspace.package.version` 为准（设置「关于」页通过 `CARGO_PKG_VERSION` 注入）。

发版前请同步：

1. `Cargo.toml` → `[workspace.package] version`
2. [`README.md`](../README.md) / [`README_EN.md`](../README_EN.md) 中的 version 徽章
3. （可选）`CHANGELOG` / Release 说明正文

### 0.9.6 变更摘要

- 将 HUD 绘制与布局编辑拆分为按画布、帧绘制、调整面板、效果和覆盖层组织的模块。
- 在 HUD 布局模式中增加虚拟组的右键创建、调整与删除，并支持 HUD 拖入、移出及自动布局成员排序。
- 增加自由组内布局和成员级几何持久化；组内 HUD 可独立选择、缩放并保存自己的视觉样式。
- 继续保持每屏一个 HUD 合成窗；内置包同步升至 `0.9.6`，`engine = "0.9"` 与 `api_version = 4` 保持不变。

### 0.9.5 变更摘要

- 将 HUD 运行态收集链切换为稳定实例解析，并按总开关、插件、实例及所属组四层状态决定显示。
- 支持跨插件 HUD 实例按横向、纵向和网格排列，应用成员顺序、间距、内边距及起始、居中、末端对齐。
- 将 HUD 组作为统一虚拟槽参与显示器选择、移动和缩放，在同一合成窗内对子帧执行中性坐标变换与裁剪。
- 保留旧 contribution 开关对确定性默认实例的控制；内置包同步升至 `0.9.5`，`engine = "0.9"` 与 `api_version = 4` 保持不变。

### 0.9.4 变更摘要

- 增加平台无关的 HUD 来源、稳定实例身份、实例帧上下文及横向、纵向、网格组内布局契约。
- 增加 HUD 实例与分组偏好模型，持久化实例配置、成员顺序、组布局、间距、内边距和对齐方式。
- 将旧 HUD 条目确定性迁移为默认实例，保留暂时缺失的插件配置，并逐项恢复损坏或重复的实例与成员关系。
- 将 HUD 实例和组格式化为可读的 TOML 数组表；旧内联格式在启动后一次性重写。内置包同步升至 `0.9.4`，`engine = "0.9"` 与 `api_version = 4` 保持不变。

### 0.9.3 变更摘要

- 将 HUD 布局尺寸配置统一为 `.size = [width, height]`，移除旧 `scale` 配置读取兼容，并统一数值持久化格式。
- 完善 HUD 布局编辑器，支持四边缩放、选中态覆盖层、滚动调整面板、颜色输入及全局/窗口/内容阴影编辑。
- 增加 HUD 背景、边框、圆角、内容颜色与阴影效果控制；内置宠物默认启用自定义气泡，所有新增文案同步中英文 PO。
- 内置包版本同步升至 `0.9.3`；`engine = "0.9"` 与 `api_version = 4` 保持不变。

### 0.9.2 变更摘要

- HUD 布局编辑支持独立调整宽度与高度，并提供百分比/像素单位、比例锁定和网格吸附。
- 增加 HUD 背景透明度、背景模糊和内容透明度调整，布局修改会即时持久化并同步运行态窗口可见性。
- 保留旧 `scale` 布局配置的读取迁移；内置包版本同步升至 `0.9.2`，`engine = "0.9"` 与 `api_version = 4` 保持不变。

### 0.9.1 变更摘要

- 将设置页绘制拆分为按页面职责组织的模块，统一复用配置卡片、配置行和图标组件，降低单文件维护成本。
- 补齐常规、关于、宠物和 HUD 设置所需的中英文 PO 文案，新增配置区块标题、空状态和元信息标签。
- 内置包版本同步升至 `0.9.1`；`engine = "0.9"` 与 `api_version = 4` 保持不变。

### 0.9.0 变更摘要

- 将 WASM Guest ABI 升至 `api_version = 4`：补充 PrintScreen、ScrollLock、Pause、ContextMenu 按键及组合键按下事件。
- 完成 PO/MO 国际化资源迁移：源码使用 PO，发布目录使用 MO；支持语言族回退和运行时扫描，不再读取旧 TOML 国际化包。
- 增加统一的键盘组合状态跟踪、跨平台输入映射和本地化输入提示，并同步内置包与引擎兼容族至 `0.9`。

### 0.8.2 变更摘要

- 接通社区宠物包的图片、图集和序列资源，从包索引加载并在 egui 场景渲染器中绘制。
- 加强 WASM Guest 宠物元数据、配置项数量/文本/重复键和窗口尺寸校验，并将 SDK 的 `API_VERSION` 与当前 `api_version = 3` 契约对齐。
- 按拖拽窗口中心所在显示器进行贴边判定和吸附，避免跨显示器拖拽时沿用旧屏幕区域。

### 0.8.1 变更摘要

- 改进糯米团与芝麻豆在四边贴靠时的姿态、翻转、倾斜、阴影和眼神方向，并保留拖拽中的自由姿态。
- 修复设置窗口打开等渲染压力场景下气泡位置命令滞后导致的回弹与拖影。
- 为 `PetScene` 增加非有限变换与节点数量上限校验测试；内置包版本同步升至 `0.8.1`，`engine = "0.8"` 与 `api_version = 3` 保持不变。

### 0.8.0 变更摘要

- 完成糯米团与芝麻豆内置宠物的 `PetScene` 矢量绘制、动画行为、气泡样式和中英文包文案。
- 宿主渲染器支持凹多边形填充、渐变路径、按场景坐标缩放描边和自适应椭圆细分，并遵守宠物阴影开关。
- 修复跨透明边界拖拽时的状态保持与瞬时全局指针采样失败；运行时默认扫描 profile / 用户数据包目录。
- 内置包清单与 Guest ABI 同步升至 `engine = "0.8"`、`api_version = 3`；旧 `pet.deskhud.specs` 偏好迁移至 `pet.deskhud.dumpling`。

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
cargo pack-builtin pet-deskhud-mochi
cargo pack-builtin pet-deskhud-sesame
cargo pack-builtin hud-deskhud-demo

# 指定输出目录
cargo pack-builtins --out dist/my-packs
```

桌面程序构建会将 `i18n/<locale>/*.po` 编译为
`target/<profile>/i18n/<locale>/*.mo`。优先调用系统 `msgfmt`；开发环境未安装
gettext 时使用内置等价编译器。发布目录只需携带生成的 `i18n/`，不需要携带 PO
源文件。

Debug / release 运行时会直接扫描对应的 `target/<profile>/packages/`。更多上下文见 [`docs/extension-guide.md`](./extension-guide.md) §1.6。
