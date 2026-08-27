# Windows HUD 原生窗口鼠标穿透

状态：待解决（待开源社区分析）

## 现象

- Windows HUD 运行态需要独立的原生合成窗口。
- HUD 窗口会遮挡其覆盖区域，下面的其他应用无法正常点击和操作。
- `WM_NCHITTEST` 返回 `HTTRANSPARENT` 或 `HTNOWHERE`、以及 `WS_EX_TRANSPARENT`、`WS_EX_NOACTIVATE`、分层窗口组合，在当前 DirectComposition 路径下均未稳定实现跨进程鼠标穿透。
- HUD 窗口还需要继续遵循 HUD 条目包围盒尺寸和 `hud.global.layer` 层级策略。

## 已确认结论

微软文档说明，`HTTRANSPARENT` 只会把命中继续交给同一 GUI 线程的下层窗口，不保证跨进程穿透：

- [WM_NCHITTEST](https://learn.microsoft.com/en-us/windows/win32/inputdev/wm-nchittest)
- [Extended Window Styles](https://learn.microsoft.com/en-us/windows/win32/winmsg/extended-window-styles)

`WS_EX_TRANSPARENT` 主要影响同线程兄弟窗口的绘制顺序，并不是通用的鼠标穿透 API。DirectComposition 顶级窗口的透明像素也不等于输入区域透明。

## Winhole 参考

[Winhole（窗口洞洞波）介绍](https://www.appinn.com/winhole/)说明其基于 AutoHotkey 的 `winhole 17` 脚本，采用“窗口打洞”思路：临时调整窗口区域/层级，并在需要时处理或暂时最小化上层窗口，使下层应用可操作。它不是普通透明顶级窗口持续返回 `HTTRANSPARENT`。

目前没有确认 Winhole 本体具有公开源码和明确许可证；若复用原始脚本或实现细节，需要先确认授权。

## 后续候选方案

1. 研究 Windows 窗口区域（`SetWindowRgn`/等价裁剪）与 DirectComposition 的兼容性，让 HUD 窗口只覆盖实际绘制区域。
2. 研究 Winhole 类“临时窗口打洞”流程，明确其对当前活动窗口、窗口层级和拖拽输入的影响。
3. 评估系统级输入穿透/鼠标转发方案；不能把跨进程 `HTTRANSPARENT` 继续当作可靠方案。
4. 保持宠物、气泡、HUD 信息、HUD 布局四类窗口相互独立，不将 HUD 合并进宠物窗口，也不让 HUD 长期覆盖整个显示器。

## 当前处理

本轮失败的 HUD 穿透实验代码已移除，恢复到实验前的实现基线；后续在明确可行方案后再单独实现和验收。

---

## 供 GitHub 发布（可直接粘贴到 issues）

> 建议另开一个公开 issue（可翻译成英文附带此中文）。中文草稿如下：

### 标题
Windows: how to make an always-on-top composition window click-through to the window below (D3D11 + DirectComposition HUD)

### 正文

**Context / 背景**
I'm building a desktop-pet engine ("DeskHud"). Its Windows HUD layer is a top-level native overlay window rendered with D3D11 + Direct2D + DirectComposition (one composited window per monitor, sized to the bounding box of the HUD items, following an always-on-top preference). The HUD is display-only and must be **fully click-through**: mouse events should reach whatever window is underneath (often another process), while the HUD content remains visible.

**What I tried (all failed to give reliable cross-process click-through)**
- `WM_NCHITTEST` returning `HTTRANSPARENT` / `HTNOWHERE`.
- `WS_EX_TRANSPARENT` + `WS_EX_NOACTIVATE` (+ layered-window combos).
- Microsoft docs confirm `HTTRANSPARENT` only forwards hit-testing to a *lower window on the same GUI thread* — it is **not** a cross-process passthrough. `WS_EX_TRANSPARENT` mostly affects paint order of same-thread sibling windows, not generic hit-testing. DirectComposition's transparent pixels also do **not** map to transparent input regions for a top-level window.

**Constraints**
- HUD window is sized to the HUD-items bounding box (not full screen).
- Must stay always-on-top per user pref.
- Pets / bubbles / HUD are separate windows; I do **not** want to merge HUD into the pet window.
- Want display-only HUD with zero input interception, over other processes.

**Question / 求助**
What is the reliable Windows mechanism for a top-level, always-on-top, hardware-composited window to be fully click-through to the window below it **across processes**? Candidates I'm evaluating:
1. `SetWindowRgn` / clipping so the HWND only covers painted pixels — does that interact badly with DirectComposition?
2. A "window hole"/temporary z-order trick (à la Winhole) — but I need it continuously, not temporarily.
3. A system-level input redirection / mouse-forwarding approach.
4. Anything on modern Windows 10/11 (UWP Visual layer? `DwmExtendFrameIntoClientArea`? SetCapture/global hook?) that gives true per-pixel input passthrough.

Any pointer to the correct primitive or a known-good minimal repro is appreciated.

### 采纳后
明确可行方案后，本 issue 更新方案与验收记录。附：项目分层铁律要求 OS 类型不进引擎/包契约，方案建议尽量留在平台后端层。
