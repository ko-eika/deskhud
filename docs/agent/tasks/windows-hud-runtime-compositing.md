# 任务：Windows 运行态 HUD 合成窗（计划 1）

> 供编码 agent 执行。目标：让 HUD 条目在运行态真正合入 Windows 原生覆盖层，正常显示。

## 背景

- 引擎契约已就绪：`HudFrame` / `HudVisual::{Text, Panel}`（`crates/deskhud-engine/src/plugin/hud_contribution.rs`），插件 `hud_frame()` 产出真实帧。
- Windows 已有**布局编辑器**：`layout_editor_scene`（`gpu_overlay_probe.rs:860`）、`layout_window_proc`（`gpu_overlay_probe.rs:1140`），支持选中/移动/右下角缩放/持久化。
- **缺口**：主 GPU 循环 `GpuOverlayRenderer::render`（`gpu_overlay_probe.rs:184`）目前只合成 **宠物 + 气泡**，未把 HUD 条目合入运行态合成窗。`layout_editor_scene` 只在布局编辑器那个独立线程里使用。

## 目标

- 运行态窗口分层（三类**独立窗口**，不合并、不互相塞内容）：
  - **宠物窗**：只画宠物（`OVERLAY_HWND`）。
  - **气泡窗**：独立透明工具窗，画对话气泡（`DIALOGUE_HWND`），由宿主管理。
  - **HUD 合成窗**：**每屏一个**独立合成窗，同屏所有启用的 HUD 条目合入**这一个** HUD 层绘制。
- 明确：HUD 场景**不得**并入宠物主窗口，也不得"每条 HUD 一个窗"。
- 启用条件 = 总开关 ∧ 插件 ∧ 条目（`prefs.hud.is_active(...)`，规则见 `gpu_overlay_probe.rs:947-951` 的用法）。
- HUD 文本/面板能正确按 `slot_layout`（`x/y/scale`）定位与缩放。

## 实现要点

1. 参考 `layout_editor_scene`（`gpu_overlay_probe.rs:970-1013`）中把 `HudVisual::{Text,Panel}` 转成 `OverlayVisual::{Text,RoundedRect}` 的映射逻辑，抽成可复用函数，供运行态与编辑器共用，避免两套映射漂移。
2. **新建一个独立的 HUD 合成窗**（属于每屏一个，Windows 用原生窗口 + `GpuCompositor`），与宠物窗/气泡窗并列，各自持有自己的合成资源（参考 `OVERLAY_HWND`/`DIALOGUE_HWND` 的分窗模式）。不要往宠物窗里塞 HUD。
3. 在 `GpuPetRuntime` 中遍历 `registry.all_hud_contributions()` + `prefs.hud.is_active(...)`，收集当前启用的 HUD 条目帧，合入**HUD 合成窗**对应的场景（不是宠物场景）。
4. `GpuOverlayRenderer::render`（`gpu_overlay_probe.rs:184`）依次渲染宠物窗、气泡窗、HUD 合成窗三个独立层。
5. 布局定位用 `slot_layout(plugin_id, contribution.id, index)` 得到的 `x/y/scale`，坐标参照活动区（`WORK_*`），与 `layout_editor_scene` 一致。
6. HUD 合成窗勿每帧设置 `WindowLevel`；`OverlayScene` 的目标为当前显示器。

## 验收标准（Windows 实机）

- 打开 HUD 总开关 + 启用某插件条目后，HUD 在配置位置正常显示（文本/面板不缺失、不闪烁）。
- 布局编辑器里调整的位置/缩放，运行态生效。
- 宠物 / 气泡 / 多个 HUD 条目同层显示，层级稳定，无白窗黑窗残留。
- `cargo fmt`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo check` 全绿。
