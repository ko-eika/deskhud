# DeskHud — Agent 记忆（决策时间线）

> **现行约束**以 [`CONSTRAINTS.md`](./CONSTRAINTS.md) 为准。本文件只记「何时拍板了什么 + 为什么」，供追溯。  
> 入口见 [`AGENTS.md`](../../AGENTS.md)；目录说明见 [`README.md`](./README.md)。

## 决策

| 日期 | 决策 | 理由 |
|------|------|------|
| 2026-08-07 | 新项目 `deskhud`，仅 egui UI | 用户要求重开，去掉托盘与领域层 |
| 2026-08-07 | 沿用 PetKind / Plugin / HudContribution | 扩展底座 |
| 2026-08-07 | 右键菜单：设置 / 退出 | 配置集中到统一设置窗 |
| 2026-08-07 | 统一设置窗侧栏：宠物 / HUD / 常规（后改为常规→宠物→插件→关于） | 参考单页多分区 |
| 2026-08-07 | 默认宠 `pet.deskhud.specs` 眼睛跟全局指针 | 首个内置皮肤；勿用 RGN 裁剪 |
| 2026-08-07 | 社区扩展用 WASM，不做社区 dll | 宠物含行为 + 可沙箱 |
| 2026-08-07 | 现阶段只做开发者底座，不做商店 | 先稳定包格式 / SDK / 本地加载 |
| 2026-08-07 | crate 拆为 ui / package / engine / runtime / sdk / egui | 包格式、加载、契约、UI、Guest 解耦 |
| 2026-08-07 | i18n 扫描合并 shell+pet+plugin 目录 | 语言可配置且包可自带文案 |
| 2026-08-08 | prefs 落盘 `%APPDATA%/DeskHud/prefs.toml` | 恢复语言/宠/HUD/位置/尺寸 |
| 2026-08-08 | 贴边 / 拖拽 / 键鼠经壳→`PetEvent`/`PetPaintCtx`；包不碰 HWND | 社区宠可移植 |
| 2026-08-08 | 跨平台 MVP：`platform` + CI 三端 | 非 Win 降级 |
| 2026-08-08 | `.deskhud` + `CatalogStore` + 设置页消费 | 包发现与多源 i18n |
| 2026-08-08 | 常规页主题/字体；中英 README；设置「关于」 | 壳体验 |
| 2026-08-09 | **铁律**：宠置顶只跟 prefs；开设置时宠点击穿透 | 禁 AlwaysOnTop/owner/取消宠置顶循环 |
| 2026-08-09 | prefs 分组 `[settings]`/`[theme]`/`[font]`/`[pet]`/`[hud]`；全局键 `pet.global.*` / `hud.global.*` | 可读与兼容 |
| 2026-08-09 | 包 `version`/`engine` 门闸；`api_version` 为 ABI | 见 `docs/versioning.md` |
| 2026-08-09 | 出厂包在仓库根 `packs/`；引擎空注册表 + runtime 引导 | 与 `packages/` 扫描根区分 |
| 2026-08-09 | HUD 布局编辑：截图底模拟半透明；`HudSlotLayout` 仅 x/y/scale | Glow 子窗勿真透明 |
| 2026-08-09 | `hud.master.enable`；启用= master∧plugin∧item | 总开关关则无 HUD |
| 2026-08-09 | 多窗 AlwaysOnTop / Close+WindowLevel 易 AV；HUD 勿每帧 WindowLevel | Win 置顶与关窗竞态 |
| 2026-08-09 | 产品 0.4.0 / engine 族 0.4；图标字段 `icon` / `preview`；SVG | 包可感知契约 MINOR |
| 2026-08-09 | 运行态 HUD=每屏一合成窗同层绘制；编辑仍单窗 | 消除并排 DWM 互投影 |
| 2026-08-09 | Glow **deferred 子窗无法真透明**（GL config）；合成窗保持不透明底 | 上游 egui#3632；主窗可透明 |
| 2026-08-09 | 产品 PATCH 0.4.1；内置包 manifest `version` 跟程序 | 壳层修 bug，engine 族仍 0.4 |
| 2026-08-10 | Agent 文档迁 `docs/agent/`；`AGENTS.md` 为跨工具唯一入口 | 多智能体协作防规则漂移 |
| 2026-08-10 | 现行约束拆出 `docs/agent/CONSTRAINTS.md`；与 AGENTS 同级必读 | 入口变薄；约束单独演进 |
| 2026-08-10 | 为 Codex 在 `AGENTS.md` 增加开局必读提示与高风险约束摘要 | Codex 自动注入入口但不会自动打开链接，先阻止 Glow 子窗透明、HUD 合成、分层与置顶误改 |

## 已知上游限制（勿当「本仓库可修」）

详见现行约束 [`CONSTRAINTS.md`](./CONSTRAINTS.md)「已知上游限制」。摘要：Glow deferred 子窗无法真透明；多屏真透底勿靠第二 Glow 窗。

## 下一步（产品）

- [ ] 更多 `PetEvent` / `HudFrame`
- [ ] WASM runtime + SDK 示例
- [ ] 非 Win 透明/贴边加深
