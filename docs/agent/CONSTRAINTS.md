# DeskHud — 现行实现约束

> **动手改代码前必读。** 与 [`AGENTS.md`](../../AGENTS.md) 同级必读。  
> 决策时间线见 [`MEMORY.md`](./MEMORY.md)；不要把已推翻的尝试写成下列现行规则。

变更本文件中的约束时：同步在 `MEMORY.md` 追加一行，并确认 `AGENTS.md`「读哪里」仍指向本文。

## 通用工程

- 一能力一目录；职责变多时拆分模块。薄 `lib.rs`、短 `error.rs` 可例外。
- Rust `edition = "2024"`；公共 API 写明「为什么」；生产路径避免 `unwrap`；依赖 `{ workspace = true }`。
- workspace 只管理依赖版本与依赖配置，不会自动把依赖引入任何成员 crate；第三方版本只在根 `[workspace.dependencies]`，成员按需显式使用 `{ workspace = true }` 继承；变更后 `cargo fmt`，尽量 `clippy --workspace --all-targets -- -D warnings`。
- **文本文件统一 UTF-8**：源码、文档与配置一律使用 UTF-8（配合根目录 `.gitattributes` / `.editorconfig`），禁止以 ANSI/GBK 等其它编码写入，避免乱码；二进制资产在 `.gitattributes` 声明为 `binary`。
- 产品版本见根 `Cargo.toml`；内置包 `manifest.toml` 的 `version` **跟程序**；`engine` 跟兼容族（[`docs/versioning.md`](../versioning.md)）。
- 改架构、窗口行为或包契约前，先读近期提交说明（`git log --oneline`）及相关文件历史（`git log -- <path>`）；提交记录仅供追溯，和本文或现行代码冲突时以后两者为准。

## 分层边界（不可跨越）

- `deskhud-egui` 是当前第一版运行态和设置页实现，用于验证宠物引擎；禁止新增第二套 egui UI、托盘、或 UI 依赖 `git2`。原生平台后端后置评估，不是当前宠物引擎前置条件。
- `deskhud-engine` 仅契约，禁止依赖 `deskhud-sdk`。
- `deskhud-runtime` 发现/加载/注册；`deskhud-package` manifest/IO/包内 i18n；`deskhud-ui` 零 egui。
- 内置 = 原生 crate；社区 = WASM + `deskhud-sdk`（仅 examples/社区包）。

## UI / 窗口 / HUD

- UI 组件必须遵循统一的视觉与交互风格：布局、间距、字号、颜色、控件状态和反馈优先复用现有组件/样式，不为单个页面另起一套风格。
- 所有用户可见 UI 文案必须国际化；“关于”页及版本、作者、许可证、技术栈、主页等真实信息也必须通过国际化目录提供对应语言，不得把面向用户的语言硬编码在页面逻辑中。新增 UI 文案必须同时补齐中英文（或当前支持的全部语言）目录与缺键回退。
- 设置侧栏顺序：**常规 / 宠物 / 插件 / 关于**，默认常规；宠尺寸来自 `PetKindInfo`；设置预览用静态 `preview`/`icon`，不实时 `paint`。
- 第一版由 `winit + egui_glow` 托管运行态宠物和设置页；宠物绘制必须解释平台无关的 `PetScene`，不得由 egui 绘制器内置宠物特征。禁止恢复 eframe、deferred viewport 或第二套 egui 菜单 UI。
- 原生菜单无边框、不可缩放、由平台负责层级与失焦关闭；设置页暂由 egui 控制窗承载，有边框、可缩放、保存几何且**始终是普通非置顶窗口**。菜单不得复用设置控制窗。
- 透明命中不能靠全屏 UI 窗、窗口 RGN 或 `ExtendFrame(-1)` 模拟；Windows 覆盖层使用 DirectComposition，拖动和命中留在平台壳。
- **铁律**：宠物置顶只跟 prefs；设置窗不跟随置顶，菜单显示期间可以临时处于宠物之上；勿用 owner 或临时取消宠物置顶形成循环。
- 运行态 HUD：**每屏一个合成窗、同层绘制**；启用 = 总开关 ∧ 插件 ∧ 条目；`HudSlotLayout` 使用 `x/y/size` 控制位置和独立宽高，`size` 为 `[width, height]` 二元组，不再读取旧 `scale` 配置。
- HUD 定义、实例、组分离：持久化布局与成员引用以宿主生成的稳定实例 ID 为主键，标题不得作为身份；一个实例最多属于一个组，组不得嵌套。插件或 contribution 暂缺时保留实例与组关系，单项损坏不得阻止其它 HUD 配置加载。
- HUD / 多窗置顶：勿每帧对合成窗 `WindowLevel`；同帧 Close+置顶易 AV。

## 行为与跨平台

- 贴边/拖拽/几何在壳：按下后跨过移动阈值才进入拖动，普通单击不得闪现 `DragStarted`；拖动中允许越出工作区，松手才吸附/修正、保存位置并派发 `DockChanged`。包只读 `DockState`/`DragState`/`PetEvent`/`PetPaintCtx`，不碰 HWND。
- 键鼠经 `PetEvent`/`MouseState`；非 Win 可降级；平台码在 `platform/`。
- 对话气泡使用**宿主管理的独立透明工具窗**（逻辑子窗，不是受父客户区裁剪的 `WS_CHILD`）；包只通过后续 `PetFrame` 中性契约描述位置、皮肤、透明度、尾巴与文字，禁止接触 HWND。壳负责屏幕避让、层级、穿透和生命周期，不以频繁扩缩宠物窗代替。
- i18n：`shell.*` / `pet.<id>.*` / `plugin.<id>.*`；ID：`pet|hud.<组织>.<标识>`。
- 跨平台编码铁律：平台专属符号必须用**正确且互补**的 cfg 门。macOS 专属项唯一正确的泛化目标是 `#[cfg(not(windows))]`（macOS ⊆ not(windows)，其调用点多在 not(windows) 分支内）；新增平台几何等边界符号（如 `main_display_bounds_px`/`main_display_work_area_px`）时必须为 `platform/fallback.rs`（或对应平台）补齐同签名实现并在 `platform/mod.rs` 按平台 re-export，保证任一平台 `cargo check --workspace --all-targets` 通过、不被其它平台写的代码打破。禁止只加 `#[cfg(target_os = "macos")]` / `#[cfg(windows)]` 而遗漏互补平台的调用点。
- 后续原生桌面覆盖层以 `deskhud-engine::overlay` 和 `PetScene` 的平台无关契约为边界；包、插件和引擎契约不得出现 HWND 或任一 OS 专有类型。原生实现须等宠物引擎场景协议稳定后再评估。

## 透明合成边界

- 不透明 egui 控制窗不承担透明宠物/HUD；真透明与局部命中由各平台覆盖层后端实现。
- 多屏按能力协商；Windows 运行态 HUD 仍须每屏一个逻辑合成层，不退回第二个 Glow 透明窗或全屏输入窗。
