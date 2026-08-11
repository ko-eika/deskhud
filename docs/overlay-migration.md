# 原生桌面覆盖层迁移计划

## 目标与边界

目标是让宠物和可交互 HUD 消费指针输入，而被动 HUD 与空白区域把输入交给其他应用。这个效果依赖窗口系统，不能由单个全屏 UI 窗口可靠表达。

迁移不改变以下边界：`deskhud-engine`、宠物包、HUD 插件与 SDK 只表达场景和输入语义，不得依赖 HWND、Cocoa、Wayland 或渲染器；`deskhud-egui` 仍是唯一设置与菜单 UI。

Windows 默认路径使用原生 GPU 宠物窗与直接托管的 egui 控制窗；旧 eframe 运行路径已经移除，避免维护两套窗口生命周期。非 Windows 当前只保留可编译的控制宿主骨架，透明宠物/HUD 必须由对应平台后端接管后才算可用。HUD 原生合成尚未接管，因此 `0.5.0` 仍是迁移版本，而不是全部后端完成。

## 统一契约

`deskhud-engine::overlay` 定义：

- `OverlayDisplayTarget`：单显示器或虚拟桌面目标；
- `OverlayScene`：一帧的目标、绘制原语与命中区域；
- `OverlayHitRegion`：稳定 ID、矩形或圆形的逻辑命中形状及 `Interactive` / `Passthrough` 语义；
- `OverlayBackendCapabilities`：透明、局部穿透、单显示器和虚拟桌面能力。

当前最小绘制帧支持 `OverlayVisual::Circle`，Windows 探针已消费该原语；宠物包和 HUD 的真实绘制帧仍待后续接线。在此之前，不让任意平台 API 渗入宠物与插件接口。

## 分阶段交付

1. **契约与迁移基线（完成）**：撤回全屏 Glow 试验；建立无 OS 类型的目标、命中与能力模型。
2. **Windows 探针（单显示器通过）**：由 `DESKHUD_OVERLAY_PROBE=1` 显式启动的 native layered window 不进入正常运行路径。探针的宠物窗只覆盖宠物包围区、随宠物移动，避免每帧提交整块显示器位图；透明合成、局部命中、空白跨应用穿透、拖拽、右键、DPI 变化、工作区与置顶已通过人工验收。副显示器、负坐标、锁屏恢复和热插拔仍待设备可用时验证。
3. **GPU 可视探针（单显示器通过）**：`DESKHUD_GPU_PROBE=1` 只确认硬件 D3D11 与 DirectComposition 能力；`DESKHUD_GPU_OVERLAY_PROBE=1` 在独立原生小窗中使用当前宠物绘制帧、拖拽与 `PetEvent`。Direct3D / Direct2D / DirectComposition 资源已抽为 Windows 平台 `gpu_compositor`，支持 DWM 节奏同步、按需诊断与设备丢失重建。它仍不影响 GDI 探针或默认路径。
4. **场景与输入接线（宠物完成）**：`PetPaint` 的身体、眼睛、呼吸、对话气泡与身体命中区已接入 Windows GPU 运行态；全局键鼠事件、拖拽、松手吸附和 `DockChanged` 已恢复。HUD 场景与合成仍待完成。
5. **单显示器正式版**：默认选择一台显示器，按其工作区布局；设置页展示能力与降级原因。失败时明确报告平台后端不可用，不恢复旧 eframe 路径。
6. **多显示器与其他平台**：先支持分别覆盖的显示器，再考虑虚拟桌面。macOS 和 Linux 各自实现后端；Wayland 不支持时必须明确降级，不能模拟成已支持。

## 跨平台原则

- 以能力协商而非 `cfg` 分支决定产品行为：不支持局部穿透的平台仍能显示宠物/HUD，但须采用不遮挡或用户可关闭的降级模式。
- 坐标在领域层使用目标显示器的逻辑坐标；后端独自处理物理像素、DPI、负坐标、显示器热插拔与工作区。
- `VirtualDesktop` 只有后端明确报告支持，且混合 DPI、屏幕空洞和热插拔测试完成后才能启用。
- 原生后端可使用系统窗口/合成 API，但不得成为第二套产品 UI；设置与菜单继续由 egui 提供。

## Windows 探针验收

PowerShell 启动：

```powershell
$env:DESKHUD_OVERLAY_PROBE = '1'
cargo run -p deskhud-egui
Remove-Item Env:DESKHUD_OVERLAY_PROBE
```

探针默认置顶；用 `DESKHUD_OVERLAY_PROBE_TOPMOST=0` 启动可验证非置顶模式。该环境变量只服务于探针，正式运行态仍必须由 prefs 决定置顶。

GPU 能力探针：

```powershell
$env:DESKHUD_GPU_PROBE = '1'
cargo run -p deskhud-egui
Remove-Item Env:DESKHUD_GPU_PROBE
```

GPU 可视探针（按 Escape 退出）：

```powershell
$env:DESKHUD_GPU_OVERLAY_PROBE = '1'
cargo run -p deskhud-egui
Remove-Item Env:DESKHUD_GPU_OVERLAY_PROBE
```

验收时确认：只有圆形内容可见、窗体其余部分透明，圆形持续呼吸；任务管理器中的该进程有持续 GPU 使用记录。此探针尚不承诺鼠标交互或真实宠物画面。

探针把宠物限制在主显示器的 **work area**（不覆盖任务栏），但原生透明窗本身只覆盖宠物包围区：可拖拽宠物、按 Escape 退出。圆形外应可继续操作其他应用；这是一项待人工确认的实验结果，而非已承诺的平台能力。

运行态 HUD 仍保持“每屏一个逻辑合成层”。在 GDI 路径中不得以整屏动态透明位图作为默认实现；待引入 GPU/DirectComposition 合成或可证明的局部提交后，再接管高频动态 HUD。少量跨区域且独立交互的 HUD 可作为受控例外使用小型原生透明窗，不以“每条 HUD 一个窗”为默认策略。

- 启动、点击、拖拽、右键菜单开关期间始终保持透明；
- 宠物和交互 HUD 能接收鼠标，空白与被动 HUD 可操作其下的另一进程窗口；
- 主/副显示器、125%/150% 缩放、负坐标、锁屏恢复及显示器热插拔不遗留黑/白窗；
- 置顶只由 prefs 改变；退出后不残留窗口或输入钩子。
