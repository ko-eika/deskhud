# 原生 UI 架构迁移

- 状态：待推进

## 总目标

DeskHud 从现有 `deskhud-egui` 设置页逐步迁移到平台原生 UI：Windows 使用 WinUI，macOS 使用 AppKit，Linux 使用 GTK。迁移期间保留 egui 作为 legacy 参考实现，最终移除 egui 及相关依赖。

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

- [ ] 当前约束明确原生 UI 迁移边界。
- [ ] 记录 Linux 选择 GTK、平台 crate 目录和 xtask 目录决策。
- [ ] `cargo check --workspace --all-targets` 通过。
- [ ] `cargo test --workspace` 通过。
- [ ] `cargo fmt --check` 通过。

### 验证状态

状态：未开始

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
│   └── deskhud-app/
└── tools/
    └── deskhud-xtask/
```

`platform/` 是分类目录；每个子目录都是独立 Cargo crate。`deskhud-platform` 只定义抽象，不能依赖 Windows、AppKit 或 GTK。

### 验收标准

- [ ] workspace members 包含 `crates/*`、`platform/*`、`apps/*` 和 `tools/*`。
- [ ] `deskhud-xtask` 的 Cargo alias 和文档路径更新。
- [ ] `deskhud-egui` 暂时保留且运行行为不变。
- [ ] `cargo metadata --no-deps` 通过。
- [ ] `cargo check --workspace --all-targets` 通过。
- [ ] `cargo test --workspace` 通过。
- [ ] `cargo pack-builtins` 通过。

### 验证状态

状态：未开始

- 验证平台：当前开发环境及 CI 三平台目标。
- 已验证：当前目录规划已确定。
- 未验证：目录调整后的完整构建。

### 已知问题

- 问题：目录调整可能影响 CI、Cargo alias、文档和发布脚本路径。
- 修复状态：未开始。
- 验证情况：待 workspace 调整后统一检查。

---

## 目标 3：抽离平台无关 UI 模型

### 目标效果

- `deskhud-ui` 可以在没有 egui 的情况下编译和测试。
- egui 页面只负责绘制模型和转换用户操作。

### 实现范围

- 抽离设置页状态、页面导航、表单模型和操作命令。
- 统一 `SettingsModel`、`SettingsCommand` 等平台无关类型。
- 保留 `deskhud-ui-egui-legacy` 或等价兼容实现用于对照。
- prefs、Locale、CatalogStore 和包信息逻辑不得散落在具体页面。

### 验收标准

- [ ] `deskhud-ui` 不依赖 egui。
- [ ] 设置模型和命令可以独立单元测试。
- [ ] egui 只保留绘制与事件转换。
- [ ] 现有设置页行为不回归。
- [ ] 新增文案仍同步进入各语言目录。

### 验证状态

状态：未开始

- 验证平台：平台无关 Rust 单元测试和 Windows egui legacy。
- 已验证：现有 `CatalogStore` 和 prefs 作为迁移基础。
- 未验证：完整无 egui 的 `deskhud-ui` 编译。

### 已知问题

- 问题：当前页面逻辑与 egui 绘制逻辑可能存在耦合。
- 修复状态：未开始。
- 验证情况：待抽离后补充模块级测试。

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

### 验证状态

状态：未开始

- 验证平台：Windows、macOS、Linux GTK 编译目标。
- 已验证：现有 `deskhud-engine::overlay` 可作为契约边界。
- 未验证：新平台 crate 的实际窗口生命周期。

### 已知问题

- 问题：不同平台的窗口生命周期和事件循环差异较大。
- 修复状态：未开始。
- 验证情况：待建立最小平台骨架后确认。

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

## 阶段完成记录模板

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
