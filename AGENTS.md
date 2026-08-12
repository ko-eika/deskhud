# DeskHud — Agent 工作手册

> **所有智能体的唯一入口。** 启动时应先读本文；在改代码、改窗口行为、改 crate 依赖或改包契约前，必须先完整打开 [`docs/agent/CONSTRAINTS.md`](./docs/agent/CONSTRAINTS.md)。下列仅为防踩坑摘要，不能替代全文。
> 架构 [`docs/architecture.md`](./docs/architecture.md) · 覆盖层迁移 [`docs/overlay-migration.md`](./docs/overlay-migration.md) · 扩展 [`docs/extension-guide.md`](./docs/extension-guide.md) · 版本 [`docs/versioning.md`](./docs/versioning.md) · 发版 [`docs/release.md`](./docs/release.md)。

## 开局护栏（摘要，动手前仍须读全文）

- 唯一 UI 是 `deskhud-egui`；禁止第二套 UI、托盘和 UI 依赖 `git2`。
- `deskhud-engine` 只放契约，不能依赖 `deskhud-sdk`；社区扩展只能是 WASM + SDK，不能分发原生 DLL。
- egui 仅负责菜单和设置，由 `winit + egui_glow` 直接托管不透明控制窗；宠物/HUD 透明合成属于平台覆盖层，禁止恢复 eframe/deferred 双路径。
- 透明命中不能靠全屏 UI 窗或窗口 RGN 模拟；平台覆盖层只消费中性场景/命中契约，OS 类型不得进入包和引擎契约。
- UI 组件风格必须统一；所有用户可见文案（包括关于页的版本、作者、许可证、技术栈等真实信息）必须国际化，新增文案同步补齐各语言目录，禁止页面硬编码。
- 宠物置顶只跟 prefs，设置窗始终不置顶；菜单打开时可临时位于宠物之上，禁止用 owner / 临时取消宠物置顶形成层级循环。
- 运行态 HUD 必须**每屏一个合成窗、同层绘制**；启用条件是总开关 ∧ 插件 ∧ 条目，且合成窗不要每帧设置 `WindowLevel`。
- 贴边、拖拽和 HWND 几何由壳处理（拖动可越界，松手再吸附/修正）；包只消费 `DockState`、`DragState`、`PetEvent`、`PetPaintCtx`，不直接操作 HWND。
- 对话气泡由宿主独立透明工具窗承载并负责屏幕避让；包只描述中性样式/位置，不创建 `WS_CHILD`、HWND 或平台窗口。
- 第三方版本只在根 `[workspace.dependencies]`；包兼容性与版本改动先读 `docs/versioning.md`。
- 改架构、窗口行为或包契约前，先读近期提交说明及相关文件历史；提交记录只提供上下文，冲突时以 CONSTRAINTS 与现行代码为准。

完整、可演进的规则以 [`docs/agent/CONSTRAINTS.md`](./docs/agent/CONSTRAINTS.md) 为唯一真相源；Cursor 路径不是 Codex 的真相源。

## 读哪里（多智能体）

| 优先级 | 路径 | 用途 |
|--------|------|------|
| **1** | **`AGENTS.md`（本文件）** | 产品、架构概览、范围、命令 |
| **1** | [`docs/agent/CONSTRAINTS.md`](./docs/agent/CONSTRAINTS.md) | **现行实现约束（动手前必读）** |
| 2 | [`docs/agent/MEMORY.md`](./docs/agent/MEMORY.md) | 决策时间线；非现行全文 |
| 3 | [`docs/agent/README.md`](./docs/agent/README.md) | Agent 文档索引与变更约定 |
| — | `.cursor/rules/*.mdc` | 仅 Cursor 薄指针；冲突以 CONSTRAINTS / 本文件为准 |
| — | `.cursor/MEMORY.md` | 跳转到 `docs/agent/MEMORY.md` |

改产品/架构叙述 → 更新本文件。改硬约束 → 更新 **`docs/agent/CONSTRAINTS.md`**，并在 `MEMORY.md` 追加一行。

## 一句话

**DeskHud**：可切换 **宠物包**（皮肤 + 行为）与可配置 **HUD 插件** 的**桌宠引擎**；支持社区打包扩展与多语言。

## 产品要点（优先级）

1. **宠物包**：一套包 = 外观 + 行为；切换包即切换皮肤与行为。
2. **HUD 插件**：插件可贡献多条 HUD；用户可关整个插件，也可开关单条 HUD。
3. **社区扩展**：他人开发并打包的宠物包 / HUD 插件可本地安装加载（商店后置）。
4. **国际化**：扫描并合并 **外壳 + 宠物包 + HUD 插件** 的文案目录，语言可配置。

## 技术栈（已拍板）

| 领域 | 选择 | 说明 |
|------|------|------|
| UI | egui + winit / egui_glow | 唯一 UI；平台覆盖层负责透明合成，无 eframe、无托盘、无第二套框架 |
| 内置扩展 | 原生 Rust `PetKind` / `Plugin` | 性能好、调试方便 |
| 社区扩展 | **WASM**（wasmtime）+ `deskhud-sdk` | 可带行为逻辑且可沙箱，适合下载分发 |
| 包格式 | `.deskhud`（目录或 zip）+ `manifest.toml` | 宠物包 / HUD 插件同构，靠 `kind` 区分 |
| 配置 / 清单 | `serde` + `toml` | prefs、manifest、包内 i18n |
| i18n | 多源 TOML 目录合并 | `shell.*` / `pet.<id>.*` / `plugin.<id>.*`，缺键回退 |

**不做（本阶段）**：插件商店、原生 dll 社区包、插件直接使用 egui、UI 依赖 `git2`。

## 架构与 crate

```
deskhud-egui        UI 壳（透明宠窗 / 菜单 / 设置：常规·宠物·插件·关于）
       │
       ▼
deskhud-runtime     本地发现包 → 加载（packs 原生 / WASM）→ 注册
       │
       ├── deskhud-engine      PetKind / Plugin / EngineRegistry（仅契约 + 空表）
       ├── deskhud-package   manifest、包 IO、包内 i18n 扫描
       └── deskhud-ui        Locale、prefs、目录合并与查询

deskhud-sdk         社区作者用 Guest SDK（编译为 wasm32）
```

分层硬约束见 [`docs/agent/CONSTRAINTS.md`](./docs/agent/CONSTRAINTS.md)。

## 目录

```
AGENTS.md                 ← 入口（产品 / 架构 / 命令）
docs/agent/CONSTRAINTS.md ← 现行实现约束（必读）
docs/agent/MEMORY.md      ← 决策时间线
docs/agent/README.md      ← 本目录索引
docs/                     架构、覆盖层迁移、扩展指南、版本、发版、路线图
.codex/                   Codex 项目配置预留（非规则真相源）
crates/ … packs/ … packages/ … examples/
.cursor/rules/            Cursor 薄指针（勿当第二真相源）
```

## 当前范围（初始化后演进）

- [x] 透明桌宠窗 + 拖动 + 右键菜单（设置 / 退出）
- [x] 统一设置窗（侧栏：常规 / 宠物 / 插件 / 关于；默认打开常规）+ 宠窗尺寸跟宠物包
- [x] prefs 持久化（语言 / 宠物 / HUD / 位置 / 尺寸 / 宠行为配置）
- [x] 贴边状态检测 + 松手吸附 + `PetEvent::DockChanged` / `PetPaintCtx.dock`（供宠物包行为）
- [x] 拖拽状态 + `PetEvent::DragStarted`/`DragEnded` / `PetPaintCtx.drag`
- [x] 键鼠事件 + `MouseState` / `PetKey` + 扩展指南 `docs/extension-guide.md`
- [x] 全 ID 约定 `pet|hud.<组织>.<标识>` + `[pet|hud.config]`；宠 `PetConfigOption` / 插件图标
- [x] 包格式（目录/zip）+ 本地扫描引导；跨平台 MVP（`platform` + CI 三端）
- [x] `CatalogStore` 多源 i18n 合并 + 设置页接线（宠/插件/配置项/字体来源后缀）
- [x] 桌宠引擎化（`deskhud-engine`）+ 包 `version`/`engine` 适配门闸
- [x] 内置宠/插件独立 crate + `cargo pack-builtins`
- [x] HUD 全屏布局（多屏归一化矩形 + 设置页调整布局）；运行态每屏一合成窗同层绘制
- [x] Windows 原生 GPU 覆盖层正式运行路径（透明、局部命中、拖拽与置顶已完成单显示器验收）
- [ ] 宠物行为事件完善（更多 `PetEvent`）与更中性绘制帧
- [ ] HUD 插件真实帧数据（prefs 插件级/条目级开关已具备）
- [ ] WASM runtime + SDK + 示例包

## 常用命令

```bash
cargo check
cargo test
cargo run -p deskhud-egui
cargo build -p deskhud-egui --release   # 产物见 docs/release.md
cargo pack-builtins                     # packs/ → target/packages/*.deskhud
cargo pack-builtin <dir>                # 单个，如 pet-deskhud-specs
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```

## 许可证

Apache-2.0
