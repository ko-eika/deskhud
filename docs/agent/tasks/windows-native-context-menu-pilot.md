# 任务：Windows 原生右击菜单试点 + CONSTRAINTS 变更（计划 2 第一步）

> 供编码 agent 执行。这是"宠物/HUD/对话框/右击菜单全面原生化"计划的第一个试点：先在 Windows 用系统原生菜单验证收益与接线，同时更新约束文档。

## 背景与前提

- 用户长期目标：宠物、HUD 合成窗、对话框、右击菜单均采用各平台原生实现，更好融入系统生态。
- **现状**：只有 Windows 宠物/HUD 是原生（D3D11/D2D/DirectComposition）；macOS 宠物仍是 egui_glow 回退、Linux 纯回退。右击菜单目前是自绘控制窗（egui），需要人为屏幕避让（`fit_popup_pos_points`）。
- **试点范围**：仅 Windows，把右键菜单换成系统原生 `TrackPopupMenu` / `HMENU` + `WM_CONTEXTMENU`。macOS/Linux 不受影响，仍用现有 egui 菜单。

## 必须先改文档（试点的前提，编码前先更新）

1. `docs/agent/CONSTRAINTS.md`：当前铁律"egui 仍是唯一菜单/设置 UI"（`CONSTRAINTS.md:28`、`:39`）。需追加/调整一条：**允许 Windows 原生右击菜单试点**，其余平台仍用 egui；设置页不变。改约束时同步在 `docs/agent/MEMORY.md` 追加一行。
2. `docs/overlay-migration.md` 与 `docs/agent/CONSTRAINTS.md` 中"设置与菜单继续由 egui 提供"的表述，补充 Windows 菜单试点例外。

## 实现要点

1. 在 Windows `gpu_overlay_probe.rs` 的右键路径上，用系统 `TrackPopupMenu` 弹出菜单，替换当前自绘控制窗菜单。
2. 处理菜单项与现有动作（打开设置 / 退出等）的映射，事件仍通过现有 `OverlayControlBus` / 引擎契约派发，不把 `HMENU` 泄漏进包/引擎。
3. i18n：菜单项文案仍从 `CatalogStore` 取，禁止硬编码（`CONSTRAINTS.md:27`）；原生菜单文字由系统渲染，中英文排版由系统保证。
4. 系统原生菜单自带屏幕避让/贴边翻转，可去掉/不再依赖人为 `fit_popup_pos_points` 限制。
5. 主题：评估与现有明/暗主题联动（能力范围内，避免硬套）。

## 验收标准（Windows 实机）

- 右键宠物弹出**系统原生菜单**，中英文正常、无文字错位。
- 菜单项动作（打开设置 / 退出）与原菜单一致；菜单可超出宠物窗口范围、自动保证在屏幕内。
- 不影响 macOS/Linux 现有 egui 菜单；`cargo fmt`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo check` 全绿。

## 明确后置（不在本任务内）

- HUD 合成窗、对话框、宠物本身的各平台原生化；macOS/Linux 原生后端按既有 roadmap 走。
