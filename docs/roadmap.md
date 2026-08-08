# DeskHud 路线图

## Phase 0 — 底座规划（当前）

- [x] 产品四点与技术栈拍板
- [x] crate / 目录重规划与骨架
- [x] 文档与规则与代码一致（含 [`extension-guide.md`](./extension-guide.md)）

## Phase 1 — 包与契约

- 定稿 `manifest.toml` 与 `.deskhud` 读写（`deskhud-package`）
- 扩展 `PetKind`：`tick` / `on_event` / `PetFrame`
- 扩展 `Plugin`：`HudFrame`；prefs 增加插件级开关
- 本地扫描 `packages/` 并注册（先支持「清单 + 原生内置映射」，WASM 可后接）

## Phase 2 — 国际化合并

- `CatalogStore`：多源目录合并与回退
- 包内 `i18n/*.toml` 扫描
- 设置页语言切换作用于 shell + 已加载包文案

## Phase 3 — 社区 WASM

- `deskhud-runtime` 接入 wasmtime
- `deskhud-sdk` 可编译 `wasm32-unknown-unknown`
- 示例：`examples/community-pet-idle`、`examples/community-hud-clock`
- 打包脚本 → `.deskhud`

## Phase 4 — 体验打磨

- [x] prefs 持久化
- 更多内置宠
- HUD 布局与可读性

## 明确后置

- 在线商店 / 签名分发
- 插件权限（文件系统、网络等）
- 非 Windows 平台专项优化
