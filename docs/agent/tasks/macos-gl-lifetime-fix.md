# 任务：修复 macOS 菜单文字错位与多窗口重绘冻结

> **状态：已按本页 3 个 Bug 完成代码修复（Bug 1 context 切换、Bug 2 repaint 接线、Bug 3 坐标单位），`cargo check`/`fmt` 通过；因暂无 macOS 实机，**待实机验收**。**

> 供编码 agent 使用的口述思路整理。基于对 `crates/deskhud-egui/src/native_host.rs` 的静态定位完成，需在 macOS 实机验收。

## 背景速览

- 架构：`native_host.rs` 用 `winit + egui_glow` 托管不透明控制窗。macOS 为宠物、菜单、设置**各建一个 `GlutinWindow`（独立原生窗口 + 独立 GL context + 独立 `EguiGlow`）**，见 `overlay_surface.rs` 与 `native_host.rs:96-106`。
- 症状：① 菜单中英文文字错位/字形拼接/hover 时跳变；② 菜单 hover、勾选状态与宠物动画在多窗口间冻结。
- 根因不在 egui 本体排版/绘字能力，而是在 macOS 的多窗口 GL context 生命周期管理。Windows 用同一套 egui_glow 画菜单却不坏，正因为那些表面复用**同一个** GL context。

## Bug 分类与修复

### Bug 1：三套 GL context，绘制时不切换/不保证 current（文字错位主因）

**现状**：`GlutinWindow` 每个 surface 各自创建**独立的 GL context 和 GL surface**（`OverlaySurface::new` → `GlutinWindow::new`）。`make_current` 只在创建时调用一次（`native_host.rs:1508`），之后绘制时 `draw_gl_window:820`、`draw_menu_surface:851`、`draw_settings_surface:894` 都**直接**用各自 `Arc<glow::Context>` 画，从不重新 `make_current`。

一轮事件循环把 3 个 context 都用一遍时，实际生效的只有最后 `make_current` 过的那一个 → 绘制进错误 surface、纹理/字形 atlas 错乱。现有注释 `native_host.rs:416` 已提到 "GL_INVALID_VALUE when another surface is current"。

**修复步骤**
1. 给 `GlutinWindow` 增加 `fn make_current(&self)`，内部 `unsafe { self.context.make_current(&self.surface) }`。注意 `context` 字段当前是 `PossiblyCurrentContext`（`native_host.rs:1437`），按当前 glutin 版本 API 决定如何处理（可能需要改字段类型或按返回值重存 current context）。
2. 在三个绘制函数**开头**、任何 `gl.*` 或 `egui.paint` 之前调用：
   - `draw_gl_window`（约 `native_host.rs:810-834`，`gl.clear_color`/`gl.clear` 之前）
   - `draw_menu_surface`（`native_host.rs:851-890`，`egui.run` 之后 `surface.gl.clear_*` 之前）
   - `draw_settings_surface`（`native_host.rs:894-911`，同样位置）
3. 建议把三个 surface 的「make_current + 清屏 + `egui.run` + `egui.paint` + swap_buffers」收敛成可复用例程，避免三处各写一遍而漏掉 make_current。

**注意**：`egui.paint(window)` 内部用 window 查 scale。切换 context 后第一个 `gl.clear` 与 `egui.paint` 必须发生在同一 context 上，中间不要产生跨 context 的 GL 调用。

### Bug 2：菜单/设置 surface 的 egui 重绘回调没接线（→ hover/勾选冻结）

**现状**：`set_request_repaint_callback` 只在 `resumed()` 对**主宠物窗的 `self.egui`** 设置（`native_host.rs:936-939`）。`OverlaySurface::new`（`overlay_surface.rs:31`）创建的菜单/设置 `EguiGlow` **没有** repaint 回调 → 它们内部想要的主动重绘（hover 动画、计时刷新）永不发出 `UserEvent::Repaint`，只能靠 winit 事件 `request_redraw` 兜底。事件一停就冻结。

**修复步骤**
1. menu_surface / settings_surface 是按需创建的（菜单 `native_host.rs:226-240`、设置 `native_host.rs:338-355`），所以 repaint 回调要在**创建 surface 后立即**接线，别只写在 `resumed()`。建议新写 `fn setup_surface_repaint(surface: &OverlaySurface, proxy: &EventLoopProxy)` 复用。
2. 回调发 `UserEvent::Repaint(info.delay)`。确认主事件循环把 `Repaint` 转成 `request_redraw`（现有机制见 `native_host.rs:937-939`）。
3. 与现有每帧 `request_redraw` 的兜底逻辑（`native_host.rs:1244-1271`）协调，避免重复/多余重绘。

**注意**：别在 `resumed()` 提前创建菜单/设置 surface；保持懒加载，只补回调接线。

### Bug 3：菜单指针坐标单位不一致 → hover 偏位/文字跳变（次因）

**现状**：`draw_menu_surface`（`native_host.rs:859-870`）把光标算成本地点：`local = pos2((cursor_x - origin.x)/scale, (cursor_y - origin.y)/scale)`。`cursor` 来自 `cursor_screen_px()`（`platform/macos.rs:122`，Quartz `CGEvent.location()`，**单位是 point**），而 `window.outer_position()` 在 macOS 返回 **physical pixel**，再除以 `scale_factor` 就双重缩放 → hover 命中偏位，间接造成 hover 前后文字位置变化。

**修复步骤**
1. 核对 macOS 上 winit `outer_position()` 返回逻辑(point)还是物理(pixel)。若返回物理像素，应 `(cursor - origin)` **不用除 scale**（两边同单位 point）；若逻辑则不改。总之让两个值同单位再相减。
2. 修复后固定窗口、固定字体、无动画的最小菜单下验证 hover 命中与系统光标一致。
3. 若 Bug 1 修完文字错位已消失，本项主要是命中精准度；仍要改，否则 hover 对不准。

## 推荐执行顺序（一次提交做完并整体验收）

1. Bug 1 先改（文字错位主因），单独跑一次看文字是否已正常。
2. Bug 2（冻结）——独立改、独立验证。
3. Bug 3（坐标单位）——最后微调命中。
4. `cargo fmt`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo check` 全绿。

## 验收标准（须在 macOS 实机）

- 菜单首次唤出、重复唤出、hover 切换、开关设置后，中/英文字形完整、基线一致、位置稳定；无 GL 错误/无资源生命周期警告。
- 宠物右键开菜单后**宠物动画持续播放**；菜单开设置后菜单 hover 不冻结、设置控件状态持续更新；关设置/菜单后仍能再次右键打开新窗。
- 鼠标 hover 菜单项命中与实际位置对齐。
- Windows/Linux 行为不回归。

## 别再踩的提醒

- 绘制前必须切对 context（Bug 1）——最可能是最终根因。
- 菜单/设置 surface 别在 `resumed()` 提前建，保持懒加载，只补 repaint 接线。
- 只改 macOS 分支或可安全共用处，别动 Windows 覆盖层（`gpu_overlay_probe.rs`）与引擎契约；不新增用户可见文案。
