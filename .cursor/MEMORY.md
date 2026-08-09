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
| 2026-08-09 | 改为：宠置顶保持；设置窗自身 AlwaysOnTop | 各视口独立，避免一开设置全体失效 |
| 2026-08-09 | 设置窗改回 Normal；宠/HUD 各自 AlwaysOnTop | 设置不置顶，不影响其它窗 |
| 2026-08-09 | 开设置时**仅临时**取消宠窗置顶，关闭后恢复 | Win 置顶会压住普通设置窗；非全局失效 |
| 2026-08-09 | **铁律**：宠置顶只跟 prefs；开设置时宠点击穿透 | 设置普通层可点；禁 AlwaysOnTop/owner/取消宠置顶循环 |
| 2026-08-09 | prefs 重排为 `[ui]`/`[pet]`/`[hud]` 有序扁平 | 去掉 `.config`；宠尺寸等进 `[pet]` |
| 2026-08-09 | 字体 id/family 去掉 builtin./system./fam.；宠全局键统一 `pet.global.*` | UI 不区分来源；kind/width 等与 topmost 同形 |
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
| 2026-08-09 | 包 `version`/`engine` + 产品族匹配门闸 | 见 `docs/versioning.md`；`api_version` 仍为 ABI |
| 2026-08-09 | 内置宠/插件迁 `crates/builtins/*`；引擎空注册表 | runtime 引导注册；i18n 用 PackCatalog 前缀合并 |

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
| 2026-08-09 | 0.3：host→engine、version/engine 门闸、builtins crate、HUD 全屏布局、pack-builtins | 产品称引擎；内置 compile-in + 可导出 .deskhud |
| 2026-08-09 | HUD 编辑：先截屏再开编辑窗（半透明=截图+遮罩）；工作区安全区禁入任务栏；四角缩放无手柄；松手网格吸位置+尺寸 | Glow 子窗勿真透明 |
| 2026-08-09 | 编辑器：边框内底栏、canvas 夹紧贴边、24px 网格软/硬对齐、遮罩更透 | 拖动用 egui drag_delta（点） |
| 2026-08-09 | `HudSlotLayout` 仅 `x/y/scale`；尺寸=内容基准×scale | 比例固定，拖角只改缩放 |
| 2026-08-09 | HUD 编辑取消全屏黑窗；小窗蓝框 + 顶栏小工具条 | 桌面可见；完成/取消+设置页兜底+Esc |
| 2026-08-09 | 布局 begin：同帧勿 Close 设置视口（延后一帧）；`set_window_owner` 禁 SWP_SHOWWINDOW + IsWindow | 防 0xc0000005 |
| 2026-08-09 | `hud.master.enable` 全局总开关；右键启用/禁用插件；`is_active` 需 master∧plugin∧item | 关总开关不渲染任何 HUD |
| 2026-08-09 | prefs：`[font]` 独立；全局键 `pet.global.*` / `hud.global.*`；菜单高度清零 item_spacing | 兼容旧 master.enable / ui 内字体键 |
| 2026-08-09 | 字体 id/family 去掉 builtin./system./fam.；宠 kind/width/pos 亦写 `pet.global.*` | UI 不区分字体来源；全局键同形 |
| 2026-08-09 | prefs 拆 `[theme]`/`[settings]`；写出按注册序，global/enable 优先；菜单「插件布局」；布局网格色按截图亮度 | 可读性与配置可读性 |
| 2026-08-09 | `locale` 并入 `[theme]`；总开关关则禁用布局+插件项调整；右键布局亦禁用 | 外观相关归 theme；关插件不可改 |
| 2026-08-09 | prefs `[settings]` 在 `[theme]` 前；设置无改动时禁用应用/重置，应用后关窗 | 减少误点；应用即提交并退出 |
| 2026-08-09 | 应用后关设置改由 ROOT 执行；`set_click_through` 加 FRAMECHANGED | 子视口里关窗易残留穿透导致宠假死 |
| 2026-08-09 | 置顶延后到关设置并清穿透之后再改 WindowLevel；多帧 force 清穿透 | 置顶+应用偶发与穿透抢 SetWindowPos |
| 2026-08-09 | 应用先关设置再写盘/置顶；右键菜单加宠物置顶打勾项 | 减轻置顶应用卡顿；快捷切换置顶 |
| 2026-08-09 | 改宠置顶前暂停 HUD 槽窗数帧 | 多窗 AlwaysOnTop 抢 z-order 会卡死 |
| 2026-08-09 | 开设置软隐藏 HUD；关设置/改置顶延后恢复；set_window_owner 幂等 | 关设置批量回 AlwaysOnTop + 每帧 HWND_TOP 会卡死 |
| 2026-08-09 | 全局置顶：宠/HUD/设置/菜单同一 WindowLevel；去掉设置穿透与 HUD 软隐藏 | 混用置顶会卡死；软隐藏导致 HUD 消失 |
| 2026-08-09 | 设置拖拽 16ms；关设置后延一拍改置顶；HUD 勿每帧 WindowLevel | 50ms 卡顿；同帧 Close+WindowLevel → 0xc0000005 |
| 2026-08-09 | topmost 迁 [settings]；应用先关设置再延后提交+压制 HUD | 混用分组；同帧关窗+改宠/HUD → AV |
| 2026-08-09 | 设置会话冻结置顶；HUD/宠配置不用草稿即时生效 | 草稿改 WindowLevel/拆槽窗 → 卡死与 AV |
| 2026-08-09 | 应用按影响分流：仅宠选项不拆 HUD、不改 WindowLevel | 改行为也强制关槽窗+置顶同步 → AV |
| 2026-08-09 | 应用：软提交 prefs（不发 InnerSize/WindowLevel）+ mark_closed（不发 Visible/Close）+ 延后 resize/置顶 | 宠物页任意应用 0xc0000005 |
| 2026-08-09 | 应用并关窗：先保存，错开 ~12 帧再 soft mark_closed，再冷静后改尺寸/置顶 | 应用关配置页后出现的 0xc0000005 |
| 2026-08-09 | 撤销应用后自动关设置；应用只提交+保存，窗保持打开 | 应用关窗引入 0xc0000005 |
| 2026-08-09 | 应用不再 suppress HUD；槽窗回调显式 Visible(true) | 应用后 Visible(false) 未恢复导致 HUD 消失 |
| 2026-08-09 | 出厂宠/HUD 迁仓库根 packs/（与 crates 同级）；dist→dist/packs | builtins 命名不直观；与 packages 扫描根区分 |
| 2026-08-09 | pack-builtins 默认输出 target/packages | 与 cargo 产物同树，clean 可清 |
| 2026-08-09 | 设置页图标/预览支持 SVG（resvg 栅格化）；字段改 preview_image/icon_image；packs 资源换 SVG | 清晰度；exe 图标仍 PNG/ICO |
| 2026-08-09 | 产品升 0.4.0 / engine 族 0.4 | SVG+preview_image/icon_image 为包可感知契约变更（0.x MINOR） |
| 2026-08-09 | 图标字段定名 PluginInfo/HudContribution.icon（非 icon_image） | 与 manifest icon 对齐，更简约 |
| 2026-08-09 | PetKindInfo.preview（非 preview_image） | 与 manifest preview 对齐 |
