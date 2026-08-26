# DeskHud 0.6.x 路线图（当前版本 0.6.16）

当前产品状态：引擎约定与迁移底座已建立；Windows 原生窗口迁移已在实机验证主要功能，仍有 HUD 生命周期、重绘、穿透和功能补齐事项。平台计划详见 [`docs/agent/roadmap/README.md`](agent/roadmap/README.md)。

## Phase 0 — 底座规划

- [x] 产品四点与技术栈拍板
- [x] crate / 目录重规划与骨架
- [x] 文档与规则与代码一致（含 [`extension-guide.md`](./extension-guide.md)）

## Phase 1 — 包与契约

- [x] 定稿 `manifest.toml` 与 `.deskhud` 目录/zip 读写（`deskhud-package`）
- [x] 本地扫描 `packages/`（目录 + `.deskhud`）并引导注册（清单 + 原生内置映射；WASM 后接）
- [~] 引擎化 / 版本契约 / 内置 crate 化 / HUD 全屏布局（0.3）
- 扩展 `PetKind`：更多 `PetEvent` / 中性 `PetFrame`
- [x] HUD 全屏 overlay + 布局 prefs + Windows 主屏原生布局编辑器（运行态帧合成见 Phase 4）
- [x] 扩展 `Plugin`：`HudFrame` 真实帧数据并接入运行态筛选与绘制

## Phase 1b — 跨平台 MVP（与 Phase 1 并行，基础完成）

- [x] `deskhud-egui`：统一 `OverlayBackend` 抽象（Windows 原生 GPU + macOS 基础接入 + Linux 能力降级）
- [x] `OverlayScreenArea`：活动区/系统禁区契约，Windows 使用 `MONITORINFO`，macOS 使用 `NSScreen.visibleFrame`
- [x] 宠物行为归属宠物包；宿主只转发中性 `PetEvent` / `PetPaintCtx`
- [~] macOS：右键菜单已迁移但功能待验证；宠物、气泡、HUD 和布局窗口原生迁移待推进
- [x] 非 Win：视口拖移；全局键鼠降级；CJK 字体候选扩展
- [x] CI：`windows` / `ubuntu` / `macos` `cargo check`
- 后续：非 Win 透明 / 贴边像素级对齐（非本阶段硬目标）

## Phase 2 — 国际化合并

- [x] `CatalogStore`：多源目录合并与回退
- [x] 包内 `i18n/*.toml` 扫描（经 runtime 装配）
- [x] 设置页语言切换作用于 shell + 已加载包文案（宠/插件/配置项 + 字体来源后缀）

## Phase 3 — 社区 WASM（0.6.x 后续）

- `deskhud-runtime` 接入 wasmtime
- `deskhud-sdk` 可编译 `wasm32-unknown-unknown`
- 示例：`examples/community-pet-idle`、`examples/community-hud-clock`
- 打包脚本 → `.deskhud`

## Phase 4 — 体验打磨（0.6.x）

- [x] prefs 持久化
- 更多内置宠
- [~] HUD 真实帧数据与每屏原生合成：egui 运行态已扫描 registry contribution、按三级开关绘制实际 `HudFrame`；Windows 原生合成、布局模式生命周期、退出重绘、鼠标穿透和多屏补齐仍继续推进
- [~] 主屏 HUD 布局编辑：基本功能可用；退出后的 HUD 刷新、选中 HUD 重置大小待完善
- [ ] HUD 运行态布局与可读性打磨
- [~] macOS 菜单迁移：代码已接入，功能待验收；宠物、气泡、HUD 和布局窗口原生迁移尚未开始（见 `docs/agent/roadmap/02-macos-native.md`）

## 明确后置

- 在线商店 / 签名分发
- 插件权限（文件系统、网络等）
- 非 Windows 完整体验对齐（全局钩子、Acrylic/Mica 等）

版本与适配政策见 [`versioning.md`](./versioning.md)。
