# 引擎约定与迁移底座

- 状态：进行中

## 总目标

建立平台无关、可扩展、可验收的引擎契约和迁移基础，保证宠物包、HUD 插件与各平台窗口后端解耦，并为 Windows、macOS、Linux 原生实现提供统一边界。

## 目标清单

- 目标 1：crate 分层与依赖方向
- 目标 2：宠物与 HUD 平台无关契约
- 目标 3：原生覆盖层窗口契约
- 目标 4：包格式、版本和兼容门闸
- 目标 5：国际化与配置约定
- 目标 6：跨平台能力协商与验收基础
- 目标 7：宠物包协议与规范（详见 [`05-pet-package-protocol.md`](05-pet-package-protocol.md)）
- 目标 8：插件包协议与规范（详见 [`06-plugin-package-protocol.md`](06-plugin-package-protocol.md)）

---

## 目标 1：crate 分层与依赖方向

### 目标效果

引擎契约、运行时、包 IO、UI 状态、平台窗口和扩展 SDK 职责清晰，后续平台迁移不把 OS 类型或 UI 依赖泄漏到引擎和扩展。

### 验收标准

- [x] `deskhud-engine` 只包含契约与注册表，不依赖 `deskhud-sdk`。
- [x] `deskhud-runtime` 负责发现、加载和注册。
- [x] `deskhud-package` 负责 manifest、包 IO 和包内 i18n。
- [x] `deskhud-ui` 不依赖 egui。
- [x] 社区扩展采用 WASM + SDK，不分发原生 DLL。

### 验证状态

- 状态：已验收
- 已验证：workspace crate 划分和主要依赖方向已建立。
- 未验证：WASM runtime 完整接入仍未完成。

### 已知问题

- 问题：社区 WASM runtime 和 SDK 示例尚未完成。
- 修复状态：计划中。
- 验证情况：不影响当前内置原生扩展路径。

---

## 目标 2：宠物与 HUD 平台无关契约

### 目标效果

宠物包和 HUD 插件只描述行为、场景、视觉原语和输入语义，不接触 HWND、AppKit、X11、Wayland、egui 或屏幕物理坐标。

### 验收标准

- [x] `PetEvent`、`PetPaintCtx`、`PetPaint`、`DockState`、`DragState` 和 `MouseState` 已建立。
- [x] `HudFrame` / `HudVisual` 已建立。
- [x] 宠物包不直接操作窗口、屏幕坐标或平台 API。
- [x] HUD 启用条件为总开关 ∧ 插件 ∧ 条目。
- [ ] 更完整的中性 `PetFrame` 契约完成。

### 验证状态

- 状态：进行中
- 已验证：Windows 宠物包行为和 HUD 基础帧可被原生后端消费。
- 未验证：所有平台完整消费同一契约；WASM Guest ABI 尚未闭环。

### 已知问题

- 问题：对话气泡和 HUD 视觉契约仍在演进。
- 修复状态：进行中。
- 验证情况：当前以现有中性原语和宿主窗口边界为准。

---

## 目标 3：原生覆盖层窗口契约

### 目标效果

平台后端独立负责宠物、菜单、气泡、HUD 合成和 HUD 布局窗口；设置窗口暂由 `winit + egui_glow` 承载。

### 验收标准

- [x] `OverlayDisplayTarget`、`OverlayScene`、命中区域和能力报告已建立。
- [x] 引擎契约不包含 HWND、Cocoa、X11 或 Wayland 类型。
- [x] Windows 宠物和 HUD 使用原生 GPU 合成路径。
- [x] 运行态 HUD 约定每屏一个合成窗。
- [ ] macOS、Linux 正式后端完成。

### 验证状态

- 状态：进行中
- 已验证：Windows 原生路径已接入；macOS 菜单迁移代码已接入。
- 未验证：macOS 宠物/HUD/布局与 Linux 原生后端。

### 已知问题

- 问题：平台后端仍有占位和降级实现，能力声明需要与真实实现持续对齐。
- 修复状态：进行中。
- 验证情况：详见 [`02-windows-native.md`](02-windows-native.md)、[`03-macos-native.md`](03-macos-native.md)、[`04-linux-native.md`](04-linux-native.md)。

---

## 目标 4：包格式、版本和兼容门闸

### 目标效果

包能通过 manifest 声明自身版本、引擎兼容族和 ABI 版本，加载器可以拒绝不兼容包；迁移适配不随意触发产品版本升级。

### 验收标准

- [x] `.deskhud` 目录/zip 格式已建立。
- [x] `manifest.toml` 的 `version`、`engine`、`api_version` 约定已建立。
- [x] 引擎兼容族加载门闸已建立。
- [x] 版本策略已记录在 `docs/versioning.md`。
- [ ] WASM ABI 与 Guest runtime 完整接入。

### 验证状态

- 状态：进行中
- 已验证：内置包打包、版本字段和兼容门闸已使用。
- 未验证：社区 WASM ABI 的端到端兼容测试。

### 已知问题

- 问题：WASM runtime、SDK Guest 和示例包仍未完成。
- 修复状态：未完成。
- 验证情况：当前只覆盖内置 Rust 扩展。

---

## 目标 5：国际化与配置约定

### 目标效果

外壳、宠物包和 HUD 插件文案统一通过目录合并和回退查询；prefs、布局、插件开关和窗口几何保持稳定。

### 验收标准

- [x] `CatalogStore` 多源 i18n 合并和回退已建立。
- [x] `shell.*`、`pet.*`、`plugin.*` 命名空间已建立。
- [x] HUD 总开关、插件/条目开关和 `HudSlotLayout` 已建立。
- [x] 设置页、关于页和包元数据文案遵循国际化要求。
- [ ] 所有平台菜单原生文案与 locale 切换完整联动。

### 验证状态

- 状态：进行中
- 已验证：设置页和内置包目录合并路径已接入。
- 未验证：Windows/macOS/Linux 原生菜单的完整语言切换。

### 已知问题

- 问题：原生菜单的主题、字体和 locale 联动仍需逐平台验收。
- 修复状态：进行中。
- 验证情况：Windows 菜单待实机验收，macOS 菜单功能待验收，Linux 未开始。

---

## 目标 6：跨平台能力协商与验收基础

### 目标效果

平台只报告真实能力；不支持局部穿透、虚拟桌面或 Wayland 特性的环境必须明确降级，不伪装成完整支持。

### 验收标准

- [x] `OverlayBackendCapabilities` 已建立。
- [x] 逻辑坐标与平台物理像素/DPI 转换边界已约定。
- [x] Windows 工作区、macOS 安全区基础接入。
- [ ] Linux X11/Wayland 能力报告和验收矩阵完成。
- [ ] 多显示器、热插拔、混合 DPI 统一验收完成。

### 验证状态

- 状态：进行中
- 已验证：Windows 与 macOS 基础几何能力已接入。
- 未验证：Linux 和完整多显示器矩阵。

### 已知问题

- 问题：不同平台窗口生命周期、穿透和合成能力尚未统一闭环。
- 修复状态：进行中。
- 验证情况：平台细节分别记录在各 native 文档。

## 变更记录

| 日期 | 变更 | 状态 |
|---|---|---|
| 2026-08-13 | 新增引擎约定与迁移底座目标清单，逐项记录完成度 | 进行中 |
