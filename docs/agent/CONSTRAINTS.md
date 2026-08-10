# DeskHud — 现行实现约束

> **动手改代码前必读。** 与 [`AGENTS.md`](../../AGENTS.md) 同级必读。  
> 决策时间线见 [`MEMORY.md`](./MEMORY.md)；不要把已推翻的尝试写成下列现行规则。

变更本文件中的约束时：同步在 `MEMORY.md` 追加一行，并确认 `AGENTS.md`「读哪里」仍指向本文。

## 通用工程

- 一能力一目录；职责变多时拆分模块。薄 `lib.rs`、短 `error.rs` 可例外。
- Rust `edition = "2024"`；公共 API 写明「为什么」；生产路径避免 `unwrap`；依赖 `{ workspace = true }`。
- 第三方版本只在根 `[workspace.dependencies]`；变更后 `cargo fmt`，尽量 `clippy --workspace --all-targets -- -D warnings`。
- 产品版本见根 `Cargo.toml`；内置包 `manifest.toml` 的 `version` **跟程序**；`engine` 跟兼容族（[`docs/versioning.md`](../versioning.md)）。
- 改架构、窗口行为或包契约前，先读近期提交说明（`git log --oneline`）及相关文件历史（`git log -- <path>`）；提交记录仅供追溯，和本文或现行代码冲突时以后两者为准。

## 分层边界（不可跨越）

- `deskhud-egui` 唯一 UI；禁止第二套 UI、托盘、UI 依赖 `git2`。
- `deskhud-engine` 仅契约，禁止依赖 `deskhud-sdk`。
- `deskhud-runtime` 发现/加载/注册；`deskhud-package` manifest/IO/包内 i18n；`deskhud-ui` 零 egui。
- 内置 = 原生 crate；社区 = WASM + `deskhud-sdk`（仅 examples/社区包）。

## UI / 窗口 / HUD

- 设置侧栏顺序：**常规 / 宠物 / 插件 / 关于**，默认常规；宠尺寸来自 `PetKindInfo`；设置预览用静态 `preview`/`icon`，不实时 `paint`。
- **主宠窗** Glow + `with_transparent(true)`；设置 / 菜单 / **HUD 合成窗**用 deferred、铺满**不透明**底；**禁止**子窗 `with_transparent(true)`（Windows Glow：`GL config does not support`）。
- `show_viewport_deferred` 必须在 `App::ui` 调用（勿放 `logic`）。
- 勿窗口 RGN 塑形；Win 透明：`DwmEnableBlurBehindWindow` + `DWMSBT_NONE`，勿 `ExtendFrame(-1)`。拖窗用手移 `SetWindowPos`。
- 子类化：换 HWND 前还原旧 WndProc；禁 `PREV_WNDPROC` 自引用；NC 白线用 `WM_NCCALCSIZE`/`NCACTIVATE`/`NCPAINT`。
- **铁律**：宠置顶只跟 prefs；开设置时宠可点击穿透，勿用 AlwaysOnTop/owner 循环。
- 运行态 HUD：**每屏一个合成窗、同层绘制**；启用 = 总开关 ∧ 插件 ∧ 条目；`HudSlotLayout` 仅 `x/y/scale`。
- HUD / 多窗置顶：勿每帧对合成窗 `WindowLevel`；同帧 Close+置顶易 AV。

## 行为与跨平台

- 贴边/拖拽/几何在壳；包只读 `DockState`/`DragState`/`PetEvent`/`PetPaintCtx`，不碰 HWND。
- 键鼠经 `PetEvent`/`MouseState`；非 Win 可降级；平台码在 `platform/`。
- i18n：`shell.*` / `pet.<id>.*` / `plugin.<id>.*`；ID：`pet|hud.<组织>.<标识>`。
- 原生桌面覆盖层迁移以 `deskhud-engine::overlay` 的平台无关契约为边界；包、插件和引擎契约不得出现 HWND 或任一 OS 专有类型。正式能力以平台后端报告为准，见 [`docs/overlay-migration.md`](../overlay-migration.md)。

## 已知上游限制

- Windows + eframe Glow：主视口可透明；`show_viewport_deferred` 子视口通常无法真透明（见 egui#3632）。
- 真透底多屏叠层：优先单主窗铺虚拟桌面，或原生 layered；不要依赖第二个 Glow 透明子窗。
