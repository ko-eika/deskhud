# 原生 UI 架构迁移

当前推进顺序：目标 1、目标 2、目标 3 已完成；目标 4 正在按三平台原生实现基线收敛。

- 状态：目标 1、目标 2、目标 3 已完成，目标 4 进行中

## 总目标

DeskHud 从现有 `deskhud-egui` 设置页逐步迁移到平台原生 UI：Windows 使用 WinUI，macOS 使用 AppKit，Linux 使用 GTK。迁移期间保留 egui 作为 legacy 参考实现，最终移除 egui 及相关依赖。

平台实现基线（不可互换）：

- Windows：WinUI 3（Windows App SDK）
- macOS：AppKit
- Linux：GTK

`deskhud-egui` 不属于正式入口；`apps/deskhud-native` 根据目标平台选择对应平台 crate。临时 Win32 自绘窗口仅用于迁移验证，不能作为 Windows 最终实现。

本计划不改变现有运行态覆盖层的中性契约、每屏一个 HUD 合成窗、扩展包 WASM 安全边界和国际化要求。

## 目标清单

- 目标 1：冻结现状并更新迁移约束
- 目标 2：调整 workspace 与目录结构
- 目标 3：抽离平台无关 UI 模型
- 目标 4：建立平台抽象与平台 crate
- 目标 5：实现 Windows WinUI 设置页
- 目标 6：实现 macOS AppKit 设置页
- 目标 7：实现 Linux GTK 设置页
- 目标 8：迁移剩余 UI 能力并移除 egui

---

## 目标 1：冻结现状并更新迁移约束

### 目标效果

- 项目文档明确 egui 是迁移期 legacy，而不是最终 UI 架构。
- 架构、约束、决策时间线和迁移状态彼此一致。

### 实现范围

- 更新 `docs/agent/CONSTRAINTS.md`、`docs/architecture.md` 和 `docs/agent/MEMORY.md`。
- 读取近期提交和相关文件历史，建立迁移前基线。

### 验收标准

- [x] 当前约束明确原生 UI 迁移边界。
- [x] 记录 Linux 选择 GTK、平台 crate 目录和 xtask 目录决策。
- [x] `cargo check --workspace --all-targets` 通过。
- [x] `cargo test --workspace` 通过。
- [x] `cargo fmt --check` 通过。

### 验证状态

状态：已完成

- 验证平台：当前开发环境及 CI 三平台目标。
- 已验证：暂无。
- 未验证：迁移后的平台原生 UI。

### 已知问题

- 问题：当前现行约束仍将 egui 设置页描述为唯一 UI。
- 修复状态：未开始。
- 验证情况：待阶段 1 完成后复核文档一致性。

---

## 目标 2：调整 workspace 与目录结构

### 目标效果

- 通用业务、平台实现、应用入口和开发工具分组清晰。
- `deskhud-xtask` 通过 `tools/` 维护，不属于运行时 crate。

### 实现范围

```text
deskhud/
├── Cargo.toml
├── assets/                         # 全局图标、清单和字体资源
├── locales/                         # 外壳翻译资源
├── packs/                           # 内置宠物包与 HUD 包
├── crates/                          # 通用业务 crate
│   ├── deskhud-core/
│   ├── deskhud-engine/
│   ├── deskhud-runtime/
│   ├── deskhud-package/
│   ├── deskhud-ui/
│   └── deskhud-sdk/
├── platform/                        # 平台抽象与原生实现
│   ├── deskhud-platform/
│   ├── deskhud-platform-windows/
│   ├── deskhud-platform-macos/
│   └── deskhud-platform-linux-gtk/
├── apps/
│   ├── deskhud-egui/                # 迁移期 legacy 设置页参考实现
│   └── deskhud-native/
└── tools/
    └── deskhud-xtask/
```

`platform/` 是分类目录；每个子目录都是独立 Cargo crate。`deskhud-platform` 只定义抽象，不能依赖 Windows、AppKit 或 GTK。

### 验收标准

- [x] workspace members 包含 `crates/*`、`platform/*`、`apps/*` 和 `tools/*`。
- [x] 根目录 `assets/` 与 `locales/` 全局资源目录整理完成，内置字体统一使用 `assets/fonts/Inter.ttc` 单文件。
- [x] `deskhud-xtask` 的 Cargo alias 和文档路径更新。
- [x] `deskhud-egui` 暂时保留且运行行为不变。
- [x] `cargo metadata --format-version 1 --no-deps` 通过。
- [x] `cargo check --workspace --all-targets` 通过。
- [x] `cargo test --workspace` 通过。
- [x] `cargo pack-builtins` 通过。
- [x] `cargo run -p deskhud-egui` 启动 legacy 设置页验证通过。
- [x] `cargo run -p deskhud-native` 启动 native app 入口验证通过。

### 验证状态

状态：已完成

- 验证平台：当前开发环境及 CI 三平台目标。
- 已验证：当前目录规划已确定。
- 已验证：目录调整后的完整构建、测试和内置包导出。

### 已知问题

- 问题：目录调整可能影响 CI、Cargo alias、文档和发布脚本路径。
- 修复状态：已处理。
- 验证情况：metadata、workspace check、workspace test、pack-builtins 均已通过。
- 迁移问题：旧字体 prefs ID 仍引用 JetBrainsMono，已兼容迁移到 Inter。
- 迁移问题：Inter 不包含部分旧侧栏字体图标字形，已改为 egui 几何图标。
- 运行时问题：Windows 设置页重复激活 GL 上下文会产生临时 `[7d4]` 告警，已移除重复激活。
- 工具链问题：`cargo metadata --no-deps` 未指定格式会产生兼容性警告，验证命令已统一为 `--format-version 1`。
- 遗留问题：`.po/.mo` 翻译文件支持后续实现；当前外壳翻译使用 TOML。

---

## 目标 3：抽离平台无关 UI 模型

### 目标效果

- `deskhud-ui` 可以在没有 egui 的情况下编译和测试。
- egui 页面只负责绘制模型和转换用户操作。

### 实现范围

- 抽离设置页状态、页面导航、表单模型和操作命令。
- 统一 `SettingsModel`、`SettingsCommand` 等平台无关类型。
- 统一设置页导航顺序和草稿重置规则（保留窗口几何与宠物视图模式）。
- 抽离主题偏好解析，统一 `UiTheme` 与系统主题的解析结果；具体 toolkit 的视觉样式仍由后端负责。
- 抽离字体扫描、文件名分类、家族/样式合并和字体选择数据；字体注册与加载保持在 egui/平台后端。
- 建立字体集合（TTC）face 枚举基础设施，使用字体内部元数据识别 family/style/face index，避免按文件名误判。
- 保留 `deskhud-ui-egui-legacy` 或等价兼容实现用于对照。
- prefs、Locale、CatalogStore 和包信息逻辑不得散落在具体页面。

### 验收标准

- [x] `deskhud-ui` 不依赖 egui。
- [x] 设置模型和命令可以独立单元测试。
- [x] 设置导航顺序与草稿重置规则可独立测试。
- [x] 字体扫描、分类、家族合并和主题解析不依赖 egui。
- [x] TTC/字体集合内部 face 可独立枚举并保留真实样式信息。
- [x] 字体 face/家族数据结构与宠物卡片布局计算已下沉到 `deskhud-ui`。
- [x] 设置页导航到 i18n key 的映射已下沉到通用设置模型。
- [x] 关于页产品信息已整理为通用 `AboutInfo` 模型。
- [x] 设置会话应用/取消/重置已形成通用 `SettingsEffect` 生命周期契约并完成行为验收。
- [x] 字体加载、字体图集和视觉样式不被错误下沉到通用 crate；egui 字体加载支持 TTC face index。
- [x] egui 运行路径只保留绘制、事件转换和具体字体注册。
- [x] 现有设置页行为不回归。
- [x] 新增 shell 文案键已具备英文/中文覆盖测试。

### 验证状态

状态：已完成；目标 3 工作内容、验收标准和人工验证均已完成

- 验证平台：平台无关 Rust 单元测试和 Windows egui legacy。
- 已验证：现有 `CatalogStore` 和 prefs 作为迁移基础。
- 已验证：`deskhud-ui` 无 egui 依赖，SettingsModel/SettingsEffect、主题解析、字体目录模型和文案覆盖具备独立单元测试。
- 已验证：TTF/OTF/TTC face 枚举、family/style/PostScript/face index 读取、FontCatalog 家族合并和内置字体优先。
- 已验证：egui 字体加载器消费 TTC face index；语言筛选、字体回退缓存、字号、主题、性能、宠物选择和设置生命周期正常。
- 已验收：Inter 样式、设置页重开性能、应用/取消/重置、语言/主题/字体/字号和文案显示。
- 已完成：字体分类、样式归一化、家族合并已移入 `deskhud-ui`；egui 运行路径只保留绘制、事件转换和字体注册。
- 遗留边界：HUD 布局编辑器当前由 `#[cfg(any())]` 禁用，不属于目标 3 当前运行路径；未来重新启用时单独抽离其编辑状态。

### 已知问题

- 问题：HUD 布局编辑器当前由 `#[cfg(any())]` 禁用。
- 修复状态：目标 3 范围内已确认不影响当前运行路径；重新启用时单独处理。
- 验证情况：目标 3 已通过工作区检查、测试和人工验收。

---

## 目标 4：建立平台抽象与平台 crate

### 目标效果

- 平台窗口、菜单、覆盖层、输入和显示器能力拥有清晰边界。
- 通用 crate 不感知 HWND、NSWindow、GTK Widget 等平台类型。

### 实现范围

- 新增 `deskhud-platform`。
- 新增 Windows、macOS、Linux GTK 独立 crate。
- 按能力定义 `SettingsHost`、`OverlayHost`、`MenuHost`、`WindowHost`、`DisplayHost` 和 `InputHost`。
- 不设计包含所有平台职责的巨大 `Platform` trait。

### 验收标准

- [ ] 平台抽象只使用中性数据结构。
- [ ] 三个平台 crate 可以分别检查。
- [ ] 平台类型没有进入 engine、runtime、package 或 SDK。
- [ ] Linux 只引入 GTK，不引入 Qt。
- [ ] 既有 Windows 原生覆盖层行为不回归。

### 执行记录

- 状态：进行中
- 开始日期：2026-08-15
- 实际改动：
  - 建立 `deskhud-platform` 中性数据结构与 `SettingsHost`、`OverlayHost`、`MenuHost`、`WindowHost`、`DisplayHost`、`InputHost` 能力边界。
  - 建立 Windows、macOS、Linux GTK 三个平台 crate 的最小后端骨架。
  - 增加不含平台类型的几何与事件契约测试。
  - 增加可观察的中性窗口生命周期、菜单锚点、显示器快照和输入事件队列；三个后端均实现六类能力 trait。
  - Windows 后端通过 `windows-sys` 接入当前光标所在显示器的真实屏幕范围与工作区查询，仍复用既有覆盖层的 Win32 几何语义。
  - 将真实 Win32 窗口生命周期接入 `apps/deskhud-native`，不依赖 `deskhud-egui`。
  - 首个窗口保留系统标题栏和边框，客户区使用色键实现透明背景，并在 `WM_PAINT` 中保持调整大小后的透明重绘。
  - Windows 命中策略调整为客户区穿透、标题栏和系统按钮保留、边缘八方向调整大小命中由平台壳显式恢复。
  - 为色键透明窗口增加 8px 原生边缘命中带，避免透明像素在系统命中测试前被跳过导致二次调整大小失效。
  - （迁移验证，非最终方案）Windows 曾接入 Win32 自绘透明壳；该实现不作为 Windows 正式 UI，后续替换为 WinUI 3 Host。
  - `deskhud-native` 按当前目标平台选择 Windows / macOS / Linux GTK 后端，不再把 Windows 作为入口级默认实现。
- 验证命令：
  - `cargo fmt --all`
  - `cargo check -p deskhud-platform -p deskhud-platform-windows -p deskhud-platform-macos -p deskhud-platform-linux-gtk`
  - `cargo test -p deskhud-platform`
  - `cargo test -p deskhud-platform-windows -p deskhud-platform-macos -p deskhud-platform-linux-gtk`
  - `cargo clippy -p deskhud-platform -p deskhud-platform-windows -p deskhud-platform-macos -p deskhud-platform-linux-gtk --all-targets -- -D warnings`
  - `cargo run -p deskhud-native`
- 验证结果：通过
- Windows App SDK Runtime：已在开发机安装并核验 Windows App Runtime 2.4.0 x64/x86 包，安装器退出码为 0。
- Windows WinUI 3 Host：已将 `deskhud-platform-windows` 的临时 Win32 自绘窗口替换为 `windows-rs` WinUI 3 Host 接入骨架；当前仍待完成 `windows-rs` 依赖获取与首次编译验证，未宣称可运行。
- 遗留问题：WinUI 3 Host 的依赖获取/编译验证尚未完成；macOS AppKit、Linux GTK 已接入入口选择但尚未实现真实窗口生命周期。

- 验证平台：Windows、macOS、Linux GTK 编译目标。
- 已验证：现有 `deskhud-engine::overlay` 可作为契约边界。
- 已验证：临时 Windows Win32 验证窗口生命周期；
- 未验证：Windows WinUI 3 Host 的正式窗口生命周期。
- 未验证：macOS AppKit 的正式窗口生命周期。
- 未验证：Linux GTK 的正式窗口生命周期。

### 已知问题

- 问题：不同平台的窗口生命周期和事件循环差异较大。
- 修复状态：进行中。
- 验证情况：Windows 待切换 WinUI 3；macOS/Linux 待接入各自正式原生实现。

---

## 目标 5：实现 Windows WinUI 设置页

### 目标效果

- Windows 使用原生设置窗口，不再使用 egui 设置页。
- 常规、宠物、插件和关于页面功能完整。

### 实现范围

- 使用 `SettingsModel` 和 `SettingsCommand`。
- 依次迁移常规、宠物、插件、关于页面。
- 验证 WinUI 与 Rust 的窗口生命周期、DPI、字体、多语言和菜单桥接。
- 设置窗保持普通非置顶窗口；宠物/HUD 继续使用现有原生覆盖层。

### 验收标准

- [ ] Windows 不再启动 egui 设置页。
- [ ] prefs、语言、宠物、插件和关于页面功能完整。
- [ ] 设置窗与宠物窗层级符合约束。
- [ ] 运行态宠物、气泡和 HUD 不回退到 egui。
- [ ] Windows 实机完成 DPI 和多显示器验证。

### 验证状态

状态：未开始

- 验证平台：Windows 10/11，至少验证普通 DPI 和高 DPI。
- 已验证：现有 Windows 原生覆盖层路径。
- 未验证：WinUI 设置页。

### 已知问题

- 问题：WinUI Rust 绑定、窗口生命周期和透明合成能力需要先做技术验证。
- 修复状态：未开始。
- 验证情况：待最小 WinUI 窗口和设置页原型完成。

---

## 目标 6：实现 macOS AppKit 设置页

### 目标效果

- macOS 使用 AppKit 原生设置窗口。
- 设置窗口、菜单和宠物窗口的生命周期与层级稳定。

### 实现范围

- 使用 AppKit 实现设置窗口。
- 处理 NSWindow 生命周期、主线程、菜单、DPI 和多显示器。
- 复用通用 `SettingsModel`，不复制业务逻辑。

### 验收标准

- [ ] macOS 不依赖 egui 设置页。
- [ ] AppKit 类型不进入通用 crate。
- [ ] 设置窗、菜单和宠物窗层级符合现行约束。
- [ ] macOS 实机完成菜单、关闭、重开和多显示器验证。

### 验证状态

状态：未开始

- 验证平台：macOS 实机，包含不同缩放设置。
- 已验证：现有 macOS 菜单迁移基础。
- 未验证：AppKit 设置页。

### 已知问题

- 问题：AppKit 要求 UI 操作在主线程执行，窗口生命周期与 Rust 事件循环需要协调。
- 修复状态：未开始。
- 验证情况：待 AppKit 设置窗口原型完成。

---

## 目标 7：实现 Linux GTK 设置页

### 目标效果

- Linux 使用 GTK 原生设置窗口。
- 明确 X11 与 Wayland 的透明、命中和窗口能力边界。

### 实现范围

- 使用 GTK 实现设置窗口。
- 先完成 X11 基础支持，再评估 Wayland 能力。
- GTK 依赖只存在于 Linux 平台 crate。
- 不引入 Qt。

### 验收标准

- [ ] Linux GTK 设置页功能完整。
- [ ] X11 设置窗口、菜单和输入行为通过验证。
- [ ] 文档明确 Wayland 支持范围和限制。
- [ ] Linux 平台 crate 不污染通用 crate。

### 验证状态

状态：未开始

- 验证平台：Linux GTK，分别记录 X11/Wayland 环境。
- 已验证：暂无。
- 未验证：GTK 设置页和桌面环境兼容性。

### 已知问题

- 问题：Wayland 对透明窗口、局部命中和桌面层级的支持可能与 X11 不同。
- 修复状态：未开始。
- 验证情况：先完成 X11，再记录 Wayland 实机结果。

---

## 目标 8：迁移剩余 UI 能力并移除 egui

### 目标效果

- 所有正式用户可见 UI 都由平台原生后端提供。
- egui 不再承担正式功能，也不再成为运行时依赖。

### 实现范围

- 迁移 HUD 布局编辑器、原生菜单、对话气泡、宠物配置控件、插件条目开关和关于页完整信息。
- 宿主继续负责窗口几何、生命周期、命中和屏幕避让。
- 删除 `deskhud-egui`、egui、egui_glow 和相关 winit-egui 依赖。
- 更新 CI、发布文档、架构文档、README 和 Cargo.lock。

### 验收标准

- [ ] 正式 UI 全部由平台原生后端提供。
- [ ] 运行态 HUD 仍然每屏一个合成窗、同层绘制。
- [ ] 宠物包、HUD 插件和 engine 不包含平台专有类型。
- [ ] 三个平台完成实机验收。
- [ ] 以下命令全部通过：

```bash
cargo fmt --check
cargo check --workspace --all-targets
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo pack-builtins
```

### 验证状态

状态：未开始

- 验证平台：Windows WinUI、macOS AppKit、Linux GTK。
- 已验证：当前原生覆盖层迁移基础。
- 未验证：完全移除 egui 后的三平台完整产品流程。

### 已知问题

- 问题：一次性删除 egui 可能暴露隐藏的 UI 逻辑耦合和平台回退问题。
- 修复状态：未开始。
- 验证情况：必须在逐页迁移完成后再删除依赖。

## 问题记录

| 编号 | 日期 | 阶段 | 问题 | 影响 | 当前处理 | 状态 |
|---|---|---|---|---|---|---|
| — | — | — | 暂无记录 | — | — | — |

问题状态：`待处理`、`调查中`、`已解决`、`不采用`。

## 决策记录

| 编号 | 日期 | 决策 | 原因 | 影响范围 |
|---|---|---|---|---|
| D-001 | 2026-08-14 | Linux 选择 GTK | 先固定单一 Linux toolkit，降低迁移复杂度 | Linux 平台后端 |
| D-002 | 2026-08-14 | 平台 crate 放在根目录 `platform/` | 与通用业务 crate、开发工具和应用入口分组隔离 | workspace 目录 |
| D-003 | 2026-08-14 | `deskhud-xtask` 放到 `tools/` | 它是开发工具，不属于运行时或平台后端 | 构建与打包流程 |
| D-004 | 2026-08-14 | egui 作为迁移期 legacy 保留 | 便于逐页对照和回退，最终移除 | 设置页迁移 |

## 变更记录

| 日期 | 变更 | 状态 |
|---|---|---|
| 2026-08-14 | 建立原生 UI 迁移计划、状态表、问题记录和决策记录 | 待推进 |
| 2026-08-14 | 完成目标 1：冻结现状并统一迁移边界；明确 egui 为迁移期 legacy，确认 Windows WinUI、macOS AppKit、Linux GTK 与 `platform/` / `tools/` 目录决策 | 已完成 |
| 2026-08-15 | 完成目标 3：下沉设置模型、字体扫描与 TTC face 元数据、字体分类与家族合并、主题解析、通用设置写入和文案覆盖测试；egui 运行路径收敛为绘制、事件转换与字体注册 | 已完成 |

### 执行记录

- 状态：已完成
- 开始日期：2026-08-14
- 完成日期：2026-08-14
- 实际提交：工作区当前提交（文档变更待提交）
- 实际改动：
  - 更新 `docs/agent/CONSTRAINTS.md`，明确 egui legacy 边界。
  - 更新 `docs/architecture.md`，补充平台原生 UI 分层与迁移规则。
  - 更新 `docs/agent/MEMORY.md`，记录目标 1 决策。
- 验证命令：
  - `cargo check --workspace --all-targets`
  - `cargo test --workspace`
  - `cargo fmt --check`
- 验证结果：通过（格式检查、全工作区编译与测试均通过）
- 遗留问题：无

## 阶段完成记录模板

## 当前推进顺序（2026-08-14 修订）

- 目标 1：已完成。
- 目标 2：已完成；保留 `apps/deskhud-egui` 作为 legacy 参考，并完成目录规划、workspace 约束、资源迁移和运行验证。
- 目标 3：目标 2 完成后执行，抽离平台无关的 `SettingsModel` / `SettingsCommand`，并确保 `deskhud-ui` 不依赖 egui。

目标 2 验收结果：`cargo metadata --format-version 1 --no-deps`、`cargo check --workspace --all-targets`、`cargo test --workspace`、`cargo pack-builtins`、`cargo fmt --check` 均通过。

每个目标完成后，在对应章节末尾补充：

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

提交标签保留英文，描述使用中文，例如：

```text
docs: 确定原生 UI 迁移方向与分层边界
build: 整理 workspace 目录并迁移开发工具
architecture: 建立跨平台窗口与原生 UI 契约
refactor: 抽离平台无关的设置模型与 UI 状态
feat: 接入 Windows 原生设置页面
fix: 修复迁移过程中的问题
chore: 移除 egui 依赖并完成原生 UI 迁移
```
