# DeskHud — Agent 记忆

## 决策

| 日期 | 决策 | 理由 |
|------|------|------|
| 2026-08-07 | 新项目 `deskhud`，仅 egui UI | 用户要求重开，去掉托盘与领域层 |
| 2026-08-07 | 沿用 PetKind / Plugin / HudContribution | 扩展底座 |
| 2026-08-07 | 右键菜单：设置 / 退出 | 配置集中到统一设置窗 |
| 2026-08-07 | 统一设置窗侧栏：宠物 / HUD / 常规 | 参考 Cursor 单页多分区 |
| 2026-08-08 | prefs 落盘 `%APPDATA%/DeskHud/prefs.toml` | 恢复语言/宠/HUD/位置/尺寸 |
| 2026-08-08 | 语言下拉箭头与插件同款描边 chevron | 去掉 egui 默认实心三角 |
| 2026-08-08 | 程序图标用 EI：`icon.png` 运行时 + `icon.ico` 嵌 exe | `winresource`；release 无控制台 |
| 2026-08-08 | 预览框 1:1 + cover 裁切；image 开 jpeg/gif/webp | 避免变形；多格式预览 |
| 2026-08-08 | 设置打开时临时取消宠窗置顶 | 设置不 AlwaysOnTop 时否则被宠挡住难点 |
| 2026-08-08 | 贴边：壳吸附+`DockState`/`DockChanged`；包只读中性状态 | 社区宠勿碰 HWND；大眼球演示姿势 |
| 2026-08-08 | 拖拽：`DragState` + DragStarted/Ended + `ctx.drag` | 与贴边并列，供包扩展行为 |
| 2026-08-08 | 键鼠：`Mouse*`/`Key*` + `MouseState`；写 extension-guide | 行为扩展面；作者文档 |
| 2026-08-08 | 明确全局指针/`GlobalMouse*`；默认宠用 pointer_dir 跟眼 | 与局部点宠事件分层 |
| 2026-08-08 | 子类化：HWND 变更须还原旧 WndProc；防 PREV 自引用；Focus 仅未获焦 | 否则 CallWindowProc → 0xC0000005 |
| 2026-08-08 | `GlobalKey*` 子集采样 + `PetPaint.bubble_text` 对话气泡 | 透明窗难获焦；空格/全局键鼠提示 |
| 2026-08-08 | 设置默认页=常规；侧栏常规→宠物→插件 | 右键打开落常规 |
| 2026-08-07 | 默认宠 `pet.deskhud.specs` 眼睛跟全局指针 | 首个内置皮肤；勿用 RGN 裁剪 |
| 2026-08-07 | 社区扩展用 WASM，不做社区 dll | 宠物含行为 + 可沙箱，利于日后下载 |
| 2026-08-07 | 现阶段只做开发者底座，不做商店 | 先稳定包格式 / SDK / 本地加载 |
| 2026-08-07 | crate 拆为 ui / package / host / runtime / sdk / egui | 包格式、加载、契约、UI、Guest 解耦 |
| 2026-08-07 | i18n 扫描合并 shell+pet+plugin 目录 | 产品要求语言可配置且包可自带文案 |
| 2026-08-08 | 跨平台 MVP 与 Phase1 并行：`platform` + CI 三端 | 非 Win 拖移/降级全局输入；完整体验后置 |
| 2026-08-08 | `.deskhud` zip IO + `PackageLoader` 扫目录/归档 | 原生内置映射；WASM 后接 |
| 2026-08-08 | `CatalogStore` + runtime `build_catalog_store` | 包键前缀 `pet|hud.<id>.`；内置 `seed_builtin_packs` |
| 2026-08-08 | 设置页消费 `CatalogStore` | 宠/插件/配置项/字体后缀随草稿语言切换 |
| 2026-08-08 | 常规：主题+可搜索字体；置顶迁宠物页 | prefs `ui_theme`；主页 `ko-eika/deskhud` |
| 2026-08-08 | 字体家族/样式/大小 + 深色控件色板 | 雅黑等友好名与别名搜索；分段按钮修边 |
| 2026-08-08 | 常规页 UI 字体：内置 Noto SC + JetBrains Mono（OFL）+ 扫系统 | prefs `shell.ui_font_id`；设置即时预览 |
| 2026-08-08 | 默认 UI 字体 JetBrains Mono / Regular / 13；列表按名排序 | 去掉内置置顶与 FontSuffix 文案 |
| 2026-08-08 | README 徽章规范化；作者 KO.EIKA；版本 0.2.0 | workspace 版本与关于页 / 徽章对齐 |
| 2026-08-08 | 中英双 README：`README.md` + `README_EN.md` | 标题 Markdown；正文加详；互链 |
| 2026-08-09 | 内置 Noto Sans SC 补齐 7 字重（~56MB） | Thin…Black；DemiLight 样式解析 |
| 2026-08-09 | 补充发版文档 `docs/release.md` | README 中英同步「发布」摘要 |
| 2026-08-08 | 设置侧栏增加「关于」页 | 展示版本 / 作者 / 许可证 / 主页 |

| 2026-08-08 | 内置字体多样式嵌入 + 与系统家族互补；设置左标签右下拉 | build 扫 assets/fonts；弹层内搜索防闪 |

## 注意

- Glow + `with_transparent(true)`；debug `opt-level=1` + `package.*=3`。  
- 设置用 deferred 子窗；勿 `show_viewport_immediate` 预热堵主线程。  
- **`show_viewport_deferred` 必须在 `App::ui` 调用**，放 `logic` 会导致子窗无法交互。  
- `App::clear_color` 对所有视口相同；菜单/HUD 须 `CentralPanel` 铺满不透明底，否则下半截发黑。  
- 禁止按外形做窗口 RGN；透明用 `DwmEnableBlurBehindWindow` + `DWMSBT_NONE`，**勿** `ExtendFrame(-1)`（会毛玻璃方框）。  
- 子窗勿 `with_transparent(true)`（Glow 常报 GL config 不支持，并可能搞坏主宠透明）。菜单 Acrylic / HUD Mica 绑 HWND 时必须排除宠窗；宠窗每帧强制 `DWMSBT_NONE`。  
- 获焦白条：子类化 `WM_NCCALCSIZE/NCACTIVATE/NCPAINT`；**勿** `WS_EX_NOACTIVATE`；拖窗用手移 `SetWindowPos`，勿 `StartDrag`/`HTCAPTION`。HWND 变化时先还原旧窗 WndProc，再装新；`PREV_WNDPROC` 勿存自引用。  
- 宿主不依赖 `deskhud-sdk`；sdk 仅给社区包 / examples。

## 下一步

- [ ] 更多 `PetEvent` / `HudFrame`  
- [ ] Phase 3：wasmtime + sdk 示例  
- [ ] 非 Win 透明/贴边加深  
