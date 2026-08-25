# 原生 UI 架构迁移计划

## 目标

DeskHud 从当前的 `deskhud-egui` 设置页逐步迁移到平台原生 UI：

- Windows：WinUI
- macOS：AppKit
- Linux：GTK

迁移期间保留 egui 作为 legacy 参考实现；迁移完成后移除 egui 及其相关依赖。

## 计划状态

阶段 2 / 目标 3 已完成并通过最终人工验收；后续进入阶段 2 / 目标 4 的平台抽象工作。

当前进度（2026-08-15）：阶段 0、阶段 1 已完成验收；阶段 1 额外完成根目录 `assets/` 与 `locales/` 资源迁移，字体统一使用 `assets/fonts/Inter.ttc`，下一步进入阶段 2。

状态含义：

- `未开始`：尚未开始执行。
- `进行中`：正在实施，可能已有部分提交。
- `已完成`：工作内容、验收和验证命令均已完成。
- `阻塞`：存在无法由当前阶段继续解决的问题，需要外部决策或环境变化。

| 阶段 | 内容 | 状态 | 开始日期 | 完成日期 | 当前提交 | 验收状态 |
|---|---|---|---|---|---|---|
| 0 | 冻结现状与更新约束 | 已完成 | 2026-08-14 | 2026-08-14 | 文档变更（工作区待提交） | 已验收 |
| 1 | 调整 workspace 和目录结构 | 已完成 | 2026-08-14 | 2026-08-15 | 工作区当前变更（待提交） | 已验收 |
| 2 | 抽离平台无关 UI 模型 | 已完成 | 2026-08-15 | 2026-08-15 | 工作区当前变更（待提交） | 已验收 |
| 3 | 建立平台抽象和平台 crate | 进行中 | 2026-08-16 | — | 工作区当前变更（待提交） | Windows 目标4已实现，待阶段验收 |
| 4 | Windows 原生设置页 | 未开始 | — | — | — | 未执行 |
| 5 | macOS AppKit 设置页 | 未开始 | — | — | — | 未执行 |
| 6 | Linux GTK 设置页 | 未开始 | — | — | — | 未执行 |
| 7 | 迁移剩余 UI 能力 | 未开始 | — | — | — | 未执行 |
| 8 | 移除 egui 与最终验收 | 未开始 | — | — | — | 未执行 |

状态更新规则：

1. 开始修改某阶段的代码或文档时，将该阶段标记为 `进行中`。
2. 只有完成该阶段全部工作内容并通过验收命令后，才能标记为 `已完成`。
3. 阶段中出现未解决问题时，在“问题记录”中登记，并将阶段标记为 `阻塞` 或继续保持 `进行中`。
4. 每次阶段状态变化都要同步填写日期、当前提交和验收状态。
5. 阶段完成后补充实际改动、验证结果和遗留问题，不只保留计划文字。

## 问题记录

| 编号 | 日期 | 阶段 | 问题 | 影响 | 当前处理 | 状态 |
|---|---|---|---|---|---|---|
| — | — | — | 暂无记录 | — | — | — |

问题状态使用：`待处理`、`调查中`、`已解决`、`不采用`。

新增问题时记录：

```text
编号：P-001
日期：YYYY-MM-DD
阶段：阶段编号
问题：具体描述
影响：受影响的 crate、平台、功能或验收项
处理：调查结果、候选方案和最终决定
状态：待处理 / 调查中 / 已解决 / 不采用
```

## 决策记录

| 编号 | 日期 | 决策 | 原因 | 影响范围 |
|---|---|---|---|---|
| D-001 | 2026-08-14 | Linux 选择 GTK | 先固定单一 Linux toolkit，降低迁移复杂度 | Linux 平台后端 |
| D-002 | 2026-08-14 | 平台 crate 放在根目录 `platform/` | 与通用业务 crate、开发工具和应用入口分组隔离 | workspace 目录 |
| D-003 | 2026-08-14 | `deskhud-xtask` 放到 `tools/` | 它是开发工具，不属于业务或平台运行时 | 构建与打包流程 |
| D-004 | 2026-08-14 | egui 作为迁移期 legacy 保留 | 便于逐页对照和回退，最终移除 | 设置页迁移 |

新增架构决策时，应同时更新本表和 `docs/agent/MEMORY.md`。

## 阶段完成记录模板

每个阶段完成后，在对应章节末尾追加以下内容：

```markdown
### 执行记录

- 状态：已完成 / 进行中 / 阻塞
- 开始日期：YYYY-MM-DD
- 完成日期：YYYY-MM-DD
- 实际提交：`<commit>`
- 实际改动：
  - …
- 验证命令：
  - `cargo check --workspace --all-targets`
- 验证结果：通过 / 失败（说明原因）
- 遗留问题：无 / 见 P-xxx
```

最终目录方向：

```text
deskhud/
├── crates/
│   ├── core/                          # 通用业务 crate
│   ├── apps/                          # 应用入口（含 deskhud-egui）
│   ├── platform/                      # 平台抽象与原生实现
│   ├── packs/                         # 内置宠物包与 HUD 包
│   └── tools/                         # 构建与打包工具
└── locales/
```

## 不变原则

资源目录补充：根目录 `assets/` 保存全局图标、清单和唯一字体 `fonts/Inter.ttc`，根目录 `locales/` 保存外壳 TOML 翻译源；扩展包翻译仍保留在各包的 `i18n/` 目录。`.po/.mo` 支持后续另行增加。

1. `deskhud-engine` 只放领域契约，不依赖任何 UI toolkit 或平台 API。
2. `deskhud-runtime` 负责包发现、加载、注册和运行协调。
3. `deskhud-ui` 只负责平台无关的 UI 状态、设置模型、prefs 和国际化。
4. `deskhud-platform` 只定义平台抽象和中性窗口/覆盖层契约。
5. 平台实现独立为 crate，平台类型不得进入 engine、runtime、package 或 SDK。
6. 宠物包和 HUD 插件只消费 `PetEvent`、`PetFrame`、`HudFrame` 等中性契约。
7. 运行态 HUD 继续遵守每屏一个合成窗、同层绘制的约束。
8. 所有用户可见文案必须经过国际化目录，不在平台页面中硬编码。
9. 每个阶段都必须保持可编译、可测试、可回退。
10. 每个阶段的提交说明和阶段小结统一使用中文。

## 阶段 0：冻结现状与更新约束

### 工作内容

- 更新 `docs/agent/CONSTRAINTS.md`，说明 egui 进入迁移期 legacy 状态。
- 更新 `docs/architecture.md`，记录原生 UI、平台 crate 和中性契约边界。
- 在 `docs/agent/MEMORY.md` 追加本次架构决策。
- 检查近期相关提交、当前 workspace 和平台代码历史。
- 建立迁移前基线。

### 验收

```bash
cargo check --workspace --all-targets
cargo test --workspace
cargo fmt --check
```

### 中文提交小结

```text
docs: 确定原生 UI 迁移方向与分层边界
```

## 阶段 1：调整 workspace 和目录结构

### 工作内容

- 在 `crates/` 下建立 `platform/`、`apps/` 和 `tools/` 分类目录。
- 将 `deskhud-xtask` 移到 `crates/tools/deskhud-xtask/`。
- 更新根目录 workspace members。
- 更新 `.cargo/config.toml` 中的 xtask 别名。
- 完成根目录 `assets/` 与 `locales/` 资源迁移，字体统一为 `assets/fonts/Inter.ttc` 单文件源。
- 保留现有 `deskhud-egui`，不改变功能行为，作为 legacy 参考入口，补充旧字体 prefs ID 到 Inter 的兼容映射。
- 预留`.po/.mo` 支持后续再实现说明。
- 修正文档、脚本和 CI 中的路径引用。

### 验收

```bash
cargo metadata --format-version 1 --no-deps
cargo check --workspace --all-targets
cargo test --workspace
cargo pack-builtins

# 运行测试，保证egui功能不受影响
cargo run -p deskhud-egui
cargo run -p deskhud-native
```

### 中文提交小结

```text
build: 整理 workspace 目录并迁移开发工具
```

## 阶段 2：抽离平台无关 UI 模型

### 工作内容

- 让 `deskhud-ui` 脱离 egui 依赖。
- 抽离设置页状态、页面导航、表单模型和操作命令。
- 抽离设置导航顺序与草稿重置等不依赖绘制的交互业务规则。
- 将 egui 页面限制为模型绘制和事件转换。
- 将设置业务逻辑放入可测试的纯 Rust 模块。
- 将字体家族/样式分类、系统字体扫描与合并视为可复用的 UI 数据逻辑；字体加载到具体 toolkit 仍由平台后端负责。
- 支持 TTC/字体集合枚举内部 face，读取真实 family、style 与 face index；不得仅按文件名 stem 将整个字体集合识别为单一 Regular。
- 将主题偏好解析（浅色 / 深色 / 跟随系统）放入通用模型；egui 视觉样式、透明背景和字号应用保留在 egui 后端。

### 目标边界

```text
deskhud-ui
    ↑
deskhud-ui-egui-legacy
```

### 验收

- `deskhud-ui` 无 egui 依赖。
- 设置模型和命令可以独立单元测试。
- 语言、prefs 和包信息逻辑不再散落在 egui 页面中。
- 字体扫描/分类/家族合并与主题偏好解析可在无 egui 环境下复用和测试。
- TTC 内部 face 元数据可被正确枚举，字体集合中的多个字重/样式不会被压缩成单一字体项。
- 设置页重复打开不再重复扫描字体目录，Inter 样式切换、应用/取消/重置和关于页信息通过人工验收。
- 设置会话结果已由通用 `SettingsEffect` 描述，并完成应用、取消、重置和页面切换验证。
- 字体数据加载、字体图集注册和具体视觉样式不进入通用 crate。

### 执行记录

- 状态：已完成
- 开始日期：2026-08-14
- 完成日期：2026-08-15
- 实际改动：设置模型、字体目录模型、TTC face 元数据、语言覆盖检测、主题解析、通用设置写入和文案覆盖测试。
- 验证命令：`cargo fmt --all`、`cargo check --workspace --all-targets`、`cargo test --workspace`。
- 验证结果：通过；`deskhud-ui` 28 项测试通过，最终人工验收通过。
- 遗留问题：HUD 布局编辑器当前由 `#[cfg(any())]` 禁用，不属于阶段 2 / 目标 3 运行路径。

### 中文提交小结

```text
refactor: 抽离平台无关的设置模型与 UI 状态
```

## 阶段 3：建立平台抽象和平台 crate

### 工作内容

新增：

```text
platform/deskhud-platform/
platform/deskhud-platform-windows/
platform/deskhud-platform-macos/
platform/deskhud-platform-linux-gtk/
```

- 定义 `SettingsHost`、`OverlayHost`、`MenuHost` 等能力边界。
- 定义 `WindowHost`、`DisplayHost`、`InputHost` 等必要抽象。
- 平台后端只消费中性场景、窗口和事件契约。
- 不设计一个包含全部职责的巨大 `Platform` trait。

### 执行记录（Windows 目标4）

- 状态：进行中（Windows 完成，Linux/macOS 暂不纳入本任务）。
- 开始日期：2026-08-16
- 实际改动：Windows 窗口回退到系统 Acrylic 材料；移除独立最小 DirectComposition host；增加宿主管理的简易右键菜单。
- 验证命令：`cargo check -p deskhud-platform-windows --target x86_64-pc-windows-msvc`
- 验证结果：通过编译检查；实机视觉与交互验收待执行。
- 遗留问题：完整菜单命令、主题、多显示器/DPI 验收及 HUD 合成后续处理。

### 验收

- 通用 crate 不依赖平台 API。
- 三个平台 crate 可分别检查。
- GTK 是 Linux 唯一 UI toolkit 后端。

### 中文提交小结

```text
architecture: 建立跨平台窗口与原生 UI 契约
```

## 阶段 4：实现 Windows 原生设置页

### 工作内容

按以下顺序迁移：

1. 常规
2. 宠物
3. 插件
4. 关于

- 使用 `SettingsModel` 和 `SettingsCommand`。
- 验证 WinUI 与 Rust 的窗口生命周期、DPI、字体和多语言支持。
- 保持设置窗非置顶。
- 保留 egui 页面用于行为和视觉参考。

### 验收

- Windows 不再使用 egui 设置页。
- prefs、语言、宠物、插件和关于页面功能完整。
- 运行态宠物与 HUD 行为不回退到 egui。

### 中文提交小结

```text
feat: 接入 Windows 原生设置页面
```

## 阶段 5：实现 macOS AppKit 设置页

### 工作内容

- 使用 AppKit 实现设置窗口。
- 处理 NSWindow 生命周期、主线程、菜单、DPI 和多显示器。
- 复用通用 `SettingsModel`，不复制业务逻辑。

### 验收

- macOS 不依赖 egui 设置页。
- AppKit 类型不进入通用 crate。
- 设置窗、菜单和宠物窗层级符合现行约束。

### 中文提交小结

```text
feat: 接入 macOS 原生设置页面
```

## 阶段 6：实现 Linux GTK 设置页

### 工作内容

- 使用 GTK 实现设置窗口。
- 先完成 X11 基础支持。
- 明确 Wayland 的透明、命中和窗口能力边界。
- 不引入 Qt。

### 验收

- Linux GTK 设置页功能完整。
- GTK 依赖只存在于 Linux 平台 crate。
- 文档明确 X11 和 Wayland 的支持范围。

### 中文提交小结

```text
feat: 接入 Linux GTK 原生设置页面
```

## 阶段 7：迁移剩余 UI 能力

### 工作内容

- 迁移 HUD 布局编辑器。
- 迁移原生菜单和对话气泡。
- 迁移宠物配置控件、插件条目开关和关于页完整信息。
- 保持宿主负责窗口几何、生命周期、命中和屏幕避让。

### 验收

- 所有正式用户可见 UI 都由平台原生后端提供。
- egui 不再承担正式功能。
- 运行态覆盖层继续遵守每屏一个合成窗。

### 中文提交小结

```text
refactor: 迁移布局编辑器与剩余原生窗口能力
```

## 阶段 8：移除 egui 与最终验收

### 工作内容

- 删除 `deskhud-egui` 或归档 legacy crate。
- 删除 egui、egui_glow 和相关 winit-egui 依赖。
- 清理旧设置页、feature、脚本和文档。
- 更新 CI、发布文档、架构文档和 README。
- 检查包、SDK 和引擎不包含 UI toolkit 依赖。

### 最终验收

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo pack-builtins
```

并完成：

- Windows：WinUI + 原生覆盖层
- macOS：AppKit + 原生覆盖层
- Linux：GTK + 已声明能力范围的 X11/Wayland

### 中文提交小结

```text
chore: 移除 egui 依赖并完成原生 UI 迁移
```

## 提交约定

提交标题统一使用中文，格式建议为：

```text
docs: 说明本次架构决策
build: 调整 workspace 或构建流程
architecture: 新增或调整跨平台契约
refactor: 抽离通用逻辑或迁移模块
feat: 新增平台实现或页面能力
fix: 修复迁移过程中的问题
chore: 删除旧实现或无用依赖
```

每个阶段完成后，在本文件对应阶段补充实际完成情况、验证命令和遗留问题。
