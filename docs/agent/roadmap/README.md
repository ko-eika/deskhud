# DeskHud 迁移路线图与验收状态

本目录是窗口框架迁移的计划与验收状态唯一入口。各平台文件同时记录该平台的宠物、菜单、气泡、HUD 合成和 HUD 布局计划与验收状态。

## 总体状态

| 里程碑 | 状态 | 文档 |
|---|---|---|
| 01 引擎约定与迁移底座 | 进行中 | [`01-engine-contracts.md`](01-engine-contracts.md) |
| 02 Windows 原生窗口 | 进行中 | [`02-windows-native.md`](02-windows-native.md) |
| 03 macOS 原生窗口 | 菜单已迁移；其它窗口待推进 | [`03-macos-native.md`](03-macos-native.md) |
| 04 Linux 原生窗口 | 待推进 | [`04-linux-native.md`](04-linux-native.md) |
| 05 宠物包协议与规范 | 进行中 | [`05-pet-package-protocol.md`](05-pet-package-protocol.md) |
| 06 插件包协议与规范 | 进行中 | [`06-plugin-package-protocol.md`](06-plugin-package-protocol.md) |

## 迁移边界

- 原生迁移对象：宠物主窗口、菜单、气泡/对话框、HUD 合成窗口、HUD 布局窗口。
- 设置窗口暂保留 `winit + egui_glow`。
- 引擎、宠物包和 HUD 插件只使用平台无关契约，不接触 HWND、AppKit、X11 或 Wayland 类型。
- 版本号只在公开功能、配置契约或扩展兼容性发生变化时调整；迁移适配本身不按 `0.x.0` 自动切版本。

## 状态约定

- `待推进`：尚未开始。
- `进行中`：已有实现，但范围或验收未闭环。
- `代码完成 / 待验收`：代码与静态检查完成，缺少目标平台实机验证。
- `已验收`：验收矩阵完成并记录结果。
