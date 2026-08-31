# DeskHud — Agent 记忆（决策时间线）

> **现行约束**以 [`CONSTRAINTS.md`](./CONSTRAINTS.md) 为准。本文件只记「何时拍板了什么 + 为什么」，供追溯。  
> 入口见 [`AGENTS.md`](../../AGENTS.md)；目录说明见 [`README.md`](./README.md)。

## 决策

| 2026-08-27 | 清理原生 UI、平台窗口和插件包旧路线图，宠物引擎改为当前唯一主动路线；第一版运行态使用 `deskhud-egui` 验证 `PetScene`，原生平台后端仅在中性协议稳定后评估 | 避免在引擎核心未稳定前并行维护多套窗口与渲染路径；宠物包先获得统一的跨后端契约 |

| 2026-08-14 | 内置字体收敛为单个 `Inter.ttc`，替换 JetBrains Mono 与 Noto Sans SC；保留旧 prefs 字体 ID 到 Inter 的回退映射 | epaint/egui 当前字体解析链支持 TrueType Collection，Inter 覆盖中英文并显著减少字体文件数量 |
| 2026-08-14 | 外壳翻译改用根目录 `locales/en.toml` 与 `locales/zh-CN.toml`；`.po/.mo` 支持延期 | TOML 与现有包内 i18n 格式一致，先保持轻量编译与运行时链路 |
| 2026-08-30 | 本地化按职责拆分 PO/MO；统一使用 `i18n/<locale>/`，外壳为 `interface/info/settings`，宠物与 HUD 为 `info/config` | 按键提示也使用 settings PO 键；0.9.0 起不再读取旧 TOML 国际化包 |

| 2026-08-14 | 原生 UI 迁移目标 1 完成：将 `deskhud-egui` 定义为迁移期 legacy 设置页；`deskhud-ui` 保持平台无关，正式设置页由 Windows WinUI、macOS AppKit、Linux GTK 提供 | 统一现行约束、架构说明与迁移路线图，避免把当前过渡实现误认为最终 UI 架构 |

| 2026-08-13 | 产品版本 PATCH 升至 `0.6.5`，engine 兼容族保持 `0.6` | 完善引擎、宠物包、插件包协议文档与迁移路线图，不改变 ABI 或包加载契约 |
| 2026-08-14 | 产品版本 PATCH 升至 `0.6.6`，engine 兼容族保持 `0.6` | 完成 macOS 目标 1 基础验收、统一宠物包贴边反馈与跨平台渲染契约审计 |
| 2026-08-14 | 跨平台编译护栏：macOS 专属项泛化为 `#[cfg(not(windows))]`；Linux/fallback 补齐缺省几何符号；任一平台保证 `cargo check` 通过；产品版本 PATCH 升至 `0.6.7`，engine 族保持 `0.6` | macOS/Windows 单平台编码曾破坏 Linux CI，需互补 cfg 与 fallback 实现 |
| 2026-08-14 | 产品版本 PATCH 升至 `0.6.8`，engine 兼容族保持 `0.6` | 完成 Windows 目标 1/2 多窗口、多显示器回归；修复 native UI GL surface 生命周期竞态日志 |
| 2026-08-15 | 产品版本 PATCH 升至 `0.6.9`，engine 兼容族保持 `0.6`；阶段 1 / 目标 2 完成验收 | 完成 workspace 分层、根目录 fonts/locales 资源迁移、旧字体兼容、设置页几何图标与 Windows GL 上下文修复；下一步进入目标 3 |
| 2026-08-15 | 产品版本 PATCH 升至 `0.6.10`，engine 兼容族保持 `0.6`；阶段 2 / 目标 3 完成验收 | 抽离平台无关设置模型、字体扫描与 TTC face 元数据、字体分类与家族合并、主题解析、通用设置写入和文案覆盖测试；egui 运行路径收敛为绘制、事件转换与字体注册 |
| 2026-08-23 | 产品版本 PATCH 升至 `0.6.11`，engine 兼容族保持 `0.6` | 完成 egui 应用与全局资源目录迁移；统一使用 `assets/fonts/Inter.ttc`；修复 macOS 原生宠物 hover、指针方向、点击与全局键鼠事件；修正 Windows 包缓存并发告警 |
| 2026-08-28 | 产品与内置包 PATCH 升至 `0.6.26`，engine 兼容族保持 `0.6`；修复三平台 CI 与跨平台字体家族 ID | Windows 依赖仅在 Windows target 引入；Source Han Sans 家族 ID 跨平台统一；CI 与当前依赖最低 Rust 版本同步为 1.95 |
| 2026-08-29 | 产品与内置包 MINOR 升至 `0.7.0`，engine 兼容族升至 `0.7`，Guest `api_version` 升至 2 | 阶段 F 完成社区 WASM Guest；外部包在 `pack-external` 时自动构建 Component；Guest 提供预览图与配置项元数据，蓝点不再以内置 Rust 宠物注册 |
| 2026-08-30 | 产品与内置包 MINOR 升至 `0.8.0`，engine 兼容族升至 `0.8`，Guest `api_version` 升至 3 | 完成糯米团与芝麻豆内置宠物及 `PetScene` 渲染扩展；新增渐变路径、阴影上下文、坐标化描边和拖拽状态修复；旧 `specs` 偏好迁移至 external Dumpling |
| 2026-08-30 | 产品与内置包 PATCH 升至 `0.8.1`，engine 兼容族保持 `0.8`，Guest `api_version` 保持 3 | 改进内置宠物贴边姿态与拖拽中的气泡定位；补充 `PetScene` 非有限数值和节点数量上限校验 |
| 2026-08-30 | 产品与内置包 PATCH 升至 `0.8.2`，engine 兼容族保持 `0.8`，Guest `api_version` 保持 3 | 接通社区宠物资源渲染；加强 WASM Guest 元数据校验并修正 SDK ABI 常量；改进跨显示器贴边判定 |
| 2026-08-31 | 产品与内置包 MINOR 升至 `0.9.0`，engine 兼容族升至 `0.9`，Guest `api_version` 升至 4 | 扩展 WASM 输入事件契约；完成 PO/MO 国际化迁移、运行时语言扫描和组合键状态跟踪；移除旧 TOML 国际化包读取 |
| 2026-08-14 | 设置页/宠物菜单/图像解码与字体枚举仅 Windows/macOS 接入；Linux（宠物运行态专用）将 `settings`、`pet_menu`、`image_decode`、`fonts` 以 `#[cfg_attr(target_os="linux", allow(dead_code))]` 关闭死代码，`native_host` 相应 cfg 修正；三平台 `cargo check --workspace --all-targets -D warnings` 均通过 | Windows 死代码报错来自 `overlay_control`（`OpenMenu`/`PetDragStarted`/`PetDragEnded`），Linux 来自上述 UI 模块；非行为变更 |
| 2026-08-14 | **缺陷（Windows 原生菜单不随应用/系统主题）**：`deskhud-egui/src/native_menu.rs` 用经典 `CreatePopupMenu`/`TrackPopupMenuEx`，Win10/11 经典菜单不自动跟随暗色；已验证 uxtheme `SetPreferredAppMode`/`AllowDarkModeForWindow` + owner `DwmSetWindowAttribute` 三套钩子在本机均不生效；owner-draw 自绘主题菜单曾试制但导致菜单异常，代码已回滚。定为缺陷待后续原生方案处理，Rest on 计划 C（C0 部分试制已回滚；C1 跨平台原生视图 trait 待做），不在本次继续调整代码 | 桌宠覆盖层已是 DirectComposition（原生）；国际化/主题契约在 `deskhud-ui`（无 egui），脱离 egui 不影响 |
| 2026-08-14 | **修复（Windows native UI GL context 报错）**：设置/菜单窗口隐藏或销毁时，winit 仍可能投递一次排队的 `RedrawRequested`；此前 `native_host::draw` 直接对已不可用的 surface 调用 `make_current`，产生 Win32 error 6（句柄无效）或 error 2004（转换操作不支持），且会重复刷日志。现仅在控制窗可见时绘制，并对仍发生的短暂失败按 1 秒节流记录后跳过当前帧；错误不再按 ERROR 级别刷屏。 |

| 2026-08-13 | 启动全平台原生窗口迁移：宠物、气泡、HUD、布局编辑器和菜单改由平台后端实现；设置窗口暂保留 `winit + egui_glow` | 先迁移窗口生命周期与合成边界，避免一次性重写设置 UI；菜单不再属于 egui 控制窗 |

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
| 2026-08-10 | 撤回全屏 Glow DesktopCanvas 试验；新增平台无关覆盖层契约与迁移计划 | `HTTRANSPARENT` / 整窗样式无法可靠实现跨应用局部穿透，稳定运行态不应为未验证方案让路 |
| 2026-08-10 | 将“读取近期提交说明和相关文件历史”加入架构改动前习惯 | 让后续智能体获得决策上下文，但不以历史提交覆盖现行约束和代码 |
| 2026-08-11 | 启动 Windows native layered 覆盖层探针，默认运行路径保持不变 | 先验证跨应用局部穿透和透明稳定性，再决定是否接入正式运行态 |
| 2026-08-11 | Windows 探针主显示器基础交互验收通过；命中区域进入平台无关场景契约 | 已确认局部跨应用穿透可行，DPI、显示器变化与置顶仍须单独验收 |
| 2026-08-11 | Windows 探针单显示器验收通过；开始将 `PetPaint` 映射为原生场景帧 | 工作区、DPI 与置顶已验证，真实宠物运行时与气泡文字仍未接管原生后端 |
| 2026-08-11 | 将 Windows 原生宠物探针从主屏全尺寸位图收敛为随宠物移动的小型透明窗，并明确 HUD 的合成与性能边界 | 避免 GDI 每帧提交整屏透明位图造成掉帧，同时保留每屏一个 HUD 逻辑合成层供后续 GPU 后端实现 |
| 2026-08-11 | 新增 D3D11 + DirectComposition 硬件能力探针，默认与 GDI 路径均保持隔离 | 先以实际设备/驱动确认核显可用性与回退路径，再迁移高频宠物绘制 |
| 2026-08-11 | 新增 Direct2D + DirectComposition 可视探针，在原生小窗持续绘制 GPU 呼吸圆形 | 先验收逐像素透明、稳定合成与持续 GPU 使用，再把 `PetPaint` 和输入迁入该后端 |
| 2026-08-11 | 可视探针改为复用 D3D11 关联的 Direct2D 设备上下文与画刷 | 避免每帧创建 COM 绘制对象导致 CPU 开销和动画卡顿 |
| 2026-08-11 | Windows GPU 可视探针验收通过：透明橙色小球置顶且呼吸流畅 | D3D11 + Direct2D + DirectComposition 可作为迁移真实宠物绘制的候选后端，GDI 路径仍保留回退 |
| 2026-08-11 | 将 GPU 合成资源抽为 Windows 平台 `gpu_compositor`，探针保留运行时与窗口消息 | 正式覆盖层后端可复用已验收的 D3D11 / Direct2D / DirectComposition 链路，默认 eframe 路径仍不变 |
| 2026-08-11 | 建立原生覆盖层到 egui 设置/退出操作的中性命令桥 | 后续 GPU 窗口只发送用户意图，继续复用既有 egui deferred 菜单与设置，而不创建第二套 UI |
| 2026-08-11 | Windows 默认入口改由 `winit + egui_glow + glutin` 直接托管不透明菜单/设置窗，并启动已验收的 DirectComposition 宠物窗；eframe 降为非 Windows 临时回退 | 解除透明宠物合成与 eframe deferred 子窗生命周期的耦合，同时保留 egui 为唯一 UI；HUD 原生合成仍待后续接管 |
| 2026-08-11 | 移除 eframe 与旧 `PetApp`/deferred 菜单/HUD 双路径，精简平台适配；菜单改为置顶不透明控制面、失焦关闭，设置窗恢复缩放并按需重绘 | 避免旧窗口生命周期与直接宿主并存造成无效代码、菜单层级错误和设置持续重绘卡顿；非 Windows 改为等待独立平台后端，不再承诺 eframe 回退 |
| 2026-08-11 | Windows 原生宠物运行态补回中性对话气泡、全局键鼠事件、`DockState`/`DockChanged` 与松手吸附；设置窗固定为非置顶 | 直接宿主接管后不能丢失宠物包既有行为；置顶偏好只控制桌面覆盖层，不应把设置窗口提升到系统顶层 |
| 2026-08-11 | 产品版本与内置包升至 `0.5.0` / `engine = "0.5"` | 覆盖层契约新增圆角矩形与文本原语；按 `0.x` 政策属于包可感知的契约扩展，不能作为 `0.4.x` PATCH 发布 |
| 2026-08-13 | 建立 `OverlayScreenArea` 活动区/禁区契约，平台分别实现；宠物动画与行为归属宠物包；记录 macOS 多窗口重绘冻结问题 | 0.6 跨平台窗口与安全区整理 |
| 2026-08-11 | 对话气泡确定为宠物同一透明场景内的视觉元素，不采用包侧子窗口；未来以联合视觉边界和中性皮肤契约扩展 | 保留透明与自定义能力，同时避免 HWND/平台窗口生命周期进入宠物包和 WASM 契约 |
| 2026-08-11 | 上一条“同一合成面”方案被替换：对话气泡改为宿主管理的独立透明工具窗，包仍只提供中性视觉描述 | 避免气泡出现/消失时频繁扩缩宠物窗，并为位置、屏幕避让、自定义背景与透明皮肤预留独立承载面；不用受父区裁剪的 `WS_CHILD` |
| 2026-08-11 | 全局键盘中性契约补齐小键盘数字、运算符、NumLock、扩展 Enter 及 NumLock 关闭时的 Insert/Clear | Windows 为小键盘使用独立 VK，且小键盘 Enter 只能通过低级事件扩展标志区分；不能只复用主键区映射 |
| 2026-08-11 | 原生宠物启动先建立键盘基线；松手坐标按 DPI 回传 prefs；左键按下与跨阈值拖动拆成独立状态 | 避免启动/切窗残留的 Tab 被当作新事件，恢复上次宠物位置，并保证单击只产生点击事件而不闪现拖动行为 |
| 2026-08-11 | 为内置大眼睛宠物增加本地随机眨眼节奏，并扩展中性覆盖层契约的椭圆原语与 `PetPaint.eye_open` | 眨眼由包的 `tick` 驱动而不触及窗口后端；椭圆让各平台后端能以一致场景语义呈现自然的眼睑闭合 |
| 2026-08-11 | `PetPaintCtx` 增加已解析的浅/深主题，气泡默认跟随主题并允许 `PetBubbleStyle::Custom`；菜单与设置共用控制窗时改为设置优先 | 包可选择跟随主题或自绘气泡而不依赖 egui/平台对象；设置打开期间右键不会切换控制窗到菜单，菜单也会跳过任务栏 |
| 2026-08-12 | 新增 UI 规范：组件风格统一，所有用户可见文案及关于页真实信息必须国际化 | 避免页面出现硬编码文案或局部组件风格漂移；新增文案需同步全部支持语言 |
| 2026-08-13 | 产品及内置包恢复升至 `0.6.0` / `engine = "0.6"`，并记录 macOS 独立菜单文字错位与字形渲染异常 | 统一窗口后端与安全区契约属于包可感知的 MINOR 变更；菜单文字问题仍未解决，后续优先检查 surface 尺寸/DPI、egui painter 生命周期、glyph atlas 纹理上传和重绘时序 |
| 2026-08-13 | 约定：文本文件统一 UTF-8 编码，新增 `.gitattributes` / `.editorconfig` 强制 | 防止 ANSI/GBK 写入导致乱码及跨平台换行混乱 |
| 2026-08-13 | 尝试修复 macOS 菜单文字错位与多窗重绘冻结（GL context 切换 + repaint 接线 + 坐标单位）；产品至 `0.6.2` | 按 `macos-gl-lifetime-fix.md` 定位；因无 macOS 实机，代码已提交、验收待定，不影响 engine 族 |
| 2026-08-13 | Windows 运行态 HUD 合成窗接入 GPU 覆盖层（独立 HUD 窗、`HTNOWHERE`/`WS_EX_NOACTIVATE` 穿透、按 `slot_layout` 定位缩放）；产品至 `0.6.3` | 消费既有 `hud_frame`/`HudVisual` 契约，engine 族不变 |
| 2026-08-13 | 产品与内置包升至 `0.6.4`；统一平台后端工厂，修正 engine 兼容族为 `0.6`，同步 macOS 待验收与三平台 CI 文档 | PATCH 修复与发布元数据同步，不改变包契约 |

| 2026-08-15 | 阶段 3 / 目标 4 固定三平台正式实现：Windows 使用 WinUI 3（Windows App SDK）、macOS 使用 AppKit、Linux 使用 GTK；`deskhud-egui` 仅作 legacy 参考，临时 Win32 自绘窗口不得升级为正式 Windows UI | 防止为修复视觉或命中问题继续扩展错误的 Win32 自绘路径；Windows 下一步先准备 App SDK Runtime，再替换 `deskhud-platform-windows` Host |
| 2026-08-16 | Windows App SDK Runtime 2.4.0 已安装并核验；`deskhud-platform-windows` 已切换到 `windows-rs` WinUI 3 Host 接入骨架，尚待依赖获取和首次编译验证 | Runtime 已具备，未将网络获取超时误记为 Host 已验收 |
| 2026-08-16 | 依赖管理约定固定：workspace 只集中管理依赖版本与依赖配置，不自动引入依赖；成员 crate 按需使用 `{ workspace = true }` 继承 | 保持版本单一来源，同时避免成员无意间获得未使用依赖 |
| 2026-08-25 | 产品版本 PATCH 升至 `0.6.12`，engine 兼容族保持 `0.6`；内置字体资源、扫描与语言筛选统一收口到 `deskhud-ui`，egui 仅负责注册渲染字体 | 修复设置页未按地区/语言筛选字体的问题，并避免字体扫描与 glyph 检查在每帧重复执行 |
| 2026-08-26 | 产品版本 PATCH 升至 `0.6.13`，engine 兼容族保持 `0.6`；记录菜单、HUD、设置窗口隐藏后渲染资源仍长期持有的内存占用问题 | 已完成现状排查，确认窗口、egui Context、Painter、OpenGL Context/Surface 和字体图集存在按需创建后只隐藏不释放的路径；修复留待后续提交 |
| 2026-08-26 | 产品与内置包 PATCH 升至 `0.6.14`，engine 兼容族保持 `0.6`；收口多窗口资源、菜单外观和 registry HUD 帧渲染 | 隐藏 HUD/设置窗口延迟压缩 Surface，字体数据跨 Context 共享并限制缓存；菜单离屏预绘后淡入并恢复国际化/主题/全局字体；运行态 HUD 按三级开关消费实际 `HudFrame` |
| 2026-08-26 | 产品与内置包 PATCH 升至 `0.6.15`，engine 兼容族保持 `0.6`；恢复性能设置与独立特效配置 | 设置页恢复帧率、动画质量、性能模式和特效卡；`[graphics]` 将气泡与阴影拆为独立键，并兼容读取旧 `effects` 键；统一渲染调度接入帧率策略 |
| 2026-08-26 | 产品与内置包 PATCH 升至 `0.6.16`，engine 兼容族保持 `0.6`；分离设置草稿与窗口几何预设保存 | 窗口移动/缩放静默保存几何；关闭设置窗口不提交未应用草稿；应用/重置按钮仅由设置内容变更控制 |
| 2026-08-27 | 产品与内置包 PATCH 升至 `0.6.17`，engine 兼容族保持 `0.6`；恢复宠物设置页选择器、配置与包内国际化 | 设置页支持网格/列表宠物选择、宠物配置开关及包内文案；统一预览资源、选择状态和中英文外壳文案 |
| 2026-08-27 | 产品与内置包 PATCH 升至 `0.6.18`，engine 兼容族保持 `0.6`；统一宠物卡片与悬浮信息布局、字体缩放和设置页导航宽度 | 网格固定五列并保持扑克比例；列表预览框与三行文字严格对齐；悬浮卡片控制宽度并统一链接字号；长导航文案缩略；宠物配置行统一高度 |
| 2026-08-27 | 产品与内置包 PATCH 升至 `0.6.19`，engine 兼容族保持 `0.6`；统一桌面 SVG 图标并接入主题着色 | 设置页侧栏、宠物选择器、右击菜单统一使用桌面图标集；SVG 缺省填充在渲染时标准化为白色后按主题着色 |
| 2026-08-27 | 产品与内置包 PATCH 升至 `0.6.20`，engine 兼容族保持 `0.6`；完善本地键鼠事件与气泡窗口层级 | 气泡窗口不抢焦点并跟随宠物层级；本地左右/中键按下、释放、单击和双击事件统一派发；内置宠物显示本地鼠标提示 |
| 2026-08-28 | 产品与内置包 PATCH 升至 `0.6.21`，engine 兼容族保持 `0.6`；修复 macOS 宠物拖拽与贴边行为 | 禁止系统窗口平铺接管宠物拖拽；修复贴边状态、越界回弹及气泡跟随窗口实际位置 |
| 2026-08-28 | 产品与内置包 PATCH 升至 `0.6.22`，engine 兼容族保持 `0.6`；字体改为外置递归扫描与 `ttf-parser` 元数据解析 | 字体不再嵌入可执行文件；构建复制 `assets/fonts/` 到 `target/<profile>/fonts/`，运行时支持按字簇、语言和样式组织字体并默认优先 Source Han Sans |
| 2026-08-28 | 产品与内置包 PATCH 升至 `0.6.23`，engine 兼容族保持 `0.6`；修复配置/预设持久化、Windows 全局键鼠监听、宠物拖拽位置保存与气泡焦点问题，并完善大眼宠物的平滑跟随与点击反馈 | 启动只加载已有配置，配置和窗口预设按明确事件保存；气泡保持鼠标穿透且不主动抢焦点；大眼宠物空闲回正、点击时瞳孔短暂朝指针探出 |
| 2026-08-30 | 内置宠物改为糯米团（Mochi）与芝麻豆（Sesame），外置 `pet.deskhud.dumpling` 保留为小汤圆并同步 192×192 尺寸 | 新内置宠物统一输出中性 `PetScene`；Dumpling 继续作为旧 `specs` 偏好的迁移目标 |

## 已知上游限制（勿当「本仓库可修」）

历史上曾受 Glow deferred 子窗透明限制；现行方案已迁移为平台覆盖层，当前边界以 [`CONSTRAINTS.md`](./CONSTRAINTS.md) 为准。

## 下一步

迁移计划下一步为目标 3：抽离平台无关 UI 模型；阶段 1 / 目标 2 已完成 workspace 分层、资源迁移及完整构建验证。产品能力事项如下：

- [ ] 更多 `PetEvent` / `HudFrame`
- [ ] WASM runtime + SDK 示例
- [ ] 非 Win 透明/贴边加深
