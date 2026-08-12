# DeskHud 0.6.x 路线图

当前产品状态：Windows 宠物覆盖层已可用；HUD 真实帧数据与每屏原生合成仍是 0.6.x 的主要缺口。

## Phase 0 — 底座规划

- [x] 产品四点与技术栈拍板
- [x] crate / 目录重规划与骨架
- [x] 文档与规则与代码一致（含 [`extension-guide.md`](./extension-guide.md)）

## Phase 1 — 包与契约

- [x] 定稿 `manifest.toml` 与 `.deskhud` 目录/zip 读写（`deskhud-package`）
- [x] 本地扫描 `packages/`（目录 + `.deskhud`）并引导注册（清单 + 原生内置映射；WASM 后接）
- [~] 引擎化 / 版本契约 / 内置 crate 化 / HUD 全屏布局（0.3）
- 扩展 `PetKind`：更多 `PetEvent` / 中性 `PetFrame`
- [x] HUD 全屏 overlay + 布局 prefs + Windows 主屏原生布局编辑器（帧数据仍待）
- 扩展 `Plugin`：`HudFrame` 真实帧数据

## Phase 1b — 跨平台 MVP（与 Phase 1 并行，基础完成）

- [x] `deskhud-egui`：统一 `OverlayBackend` 抽象（Windows GPU + macOS 原生窗口 + Linux 回退）
- [x] `OverlayScreenArea`：活动区/系统禁区契约，Windows 使用 `MONITORINFO`，macOS 使用 `NSScreen.visibleFrame`
- [x] 宠物行为归属宠物包；宿主只转发中性 `PetEvent` / `PetPaintCtx`
- [x] macOS：独立宠物、菜单/设置窗口与全局键鼠接线
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
- [ ] HUD 真实帧数据与每屏原生合成
- [x] 主屏 HUD 布局编辑：安全区、选中边框、移动、右下角缩放、持久化
- [ ] HUD 运行态布局与可读性打磨
- [ ] macOS 菜单文字错位与字形渲染异常（见 `docs/issues/macos-menu-text-offset.md`）
- [ ] macOS 多窗口重绘生命周期与 GL 资源释放（见 `docs/issues/macos-multi-window-repaint.md`）

## 明确后置

- 在线商店 / 签名分发
- 插件权限（文件系统、网络等）
- 非 Windows 完整体验对齐（全局钩子、Acrylic/Mica 等）

版本与适配政策见 [`versioning.md`](./versioning.md)。
