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

- [ ] Phase 1：manifest + Pet/Plugin 行为契约  
- [ ] Phase 2：CatalogStore 多源 i18n  
- [ ] Phase 3：wasmtime + sdk 示例  
