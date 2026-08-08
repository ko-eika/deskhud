# DeskHud — Agent 工作手册

> Agent / 协作者入口。细则见 `.cursor/rules/`；笔记见 `.cursor/MEMORY.md`；架构见 [`docs/architecture.md`](./docs/architecture.md)；**扩展指南**见 [`docs/extension-guide.md`](./docs/extension-guide.md)。

## 一句话

**DeskHud**：可切换 **宠物包**（皮肤 + 行为）与可配置 **HUD 插件** 的桌面宠物宿主；支持社区打包扩展与多语言。

## 产品要点（优先级）

1. **宠物包**：一套包 = 外观 + 行为；切换包即切换皮肤与行为。
2. **HUD 插件**：插件可贡献多条 HUD；用户可关整个插件，也可开关单条 HUD。
3. **社区扩展**：他人开发并打包的宠物包 / HUD 插件可本地安装加载（商店后置）。
4. **国际化**：扫描并合并 **外壳 + 宠物包 + HUD 插件** 的文案目录，语言可配置。

## 技术栈（已拍板）

| 领域 | 选择 | 说明 |
|------|------|------|
| UI | egui / eframe（Glow） | 唯一 UI；无托盘、无第二套框架 |
| 内置扩展 | 原生 Rust `PetKind` / `Plugin` | 性能好、调试方便 |
| 社区扩展 | **WASM**（wasmtime）+ `deskhud-sdk` | 可带行为逻辑且可沙箱，适合下载分发 |
| 包格式 | `.deskhud`（目录或 zip）+ `manifest.toml` | 宠物包 / HUD 插件同构，靠 `kind` 区分 |
| 配置 / 清单 | `serde` + `toml` | prefs、manifest、包内 i18n |
| i18n | 多源 TOML 目录合并 | `shell.*` / `pet.<id>.*` / `plugin.<id>.*`，缺键回退 |

**不做（本阶段）**：插件商店、原生 dll 社区包、插件直接使用 egui、UI 依赖 `git2`。

## 架构与 crate

```
deskhud-egui        UI 壳（透明宠窗 / 菜单 / 设置：常规·宠物·插件）
       │
       ▼
deskhud-runtime     本地发现包 → 加载（原生内置 / WASM）→ 注册
       │
       ├── deskhud-host      PetKind / Plugin / HostRegistry（契约 + 内置）
       ├── deskhud-package   manifest、包 IO、包内 i18n 扫描
       └── deskhud-ui        Locale、prefs、目录合并与查询

deskhud-sdk         社区作者用 Guest SDK（编译为 wasm32）
```

## 目录

```
crates/
  deskhud-ui/         壳 prefs + i18n 合并引擎（零 egui）
  deskhud-package/    包格式与清单
  deskhud-host/       宿主契约 + 内置宠/演示插件
  deskhud-runtime/    包加载与 WASM 适配
  deskhud-sdk/        社区 Guest SDK
  deskhud-egui/       可执行 UI
packages/             本地已安装 / 开发用包（扫描根）
examples/             社区开发示例（宠物包 / HUD 插件）
docs/                 架构与路线图
```

## 当前范围（初始化后演进）

- [x] 透明桌宠窗 + 拖动 + 右键菜单（设置 / 退出）
- [x] 统一设置窗（侧栏：常规 / 宠物 / 插件；默认打开常规）+ 宠窗尺寸跟宠物包
- [x] prefs 持久化（语言 / 宠物 / HUD / 位置 / 尺寸 / 宠行为配置）
- [x] 贴边状态检测 + 松手吸附 + `PetEvent::DockChanged` / `PetPaintCtx.dock`（供宠物包行为）
- [x] 拖拽状态 + `PetEvent::DragStarted`/`DragEnded` / `PetPaintCtx.drag`
- [x] 键鼠事件 + `MouseState` / `PetKey` + 扩展指南 `docs/extension-guide.md`
- [x] 全 ID 约定 `pet|hud.<组织>.<标识>` + `[pet|hud.config]`；宠 `PetConfigOption` / 插件图标
- [ ] 包格式 + 本地加载
- [ ] 宠物行为事件完善（更多 `PetEvent`）与更中性绘制帧
- [ ] HUD 插件真实帧数据（prefs 插件级/条目级开关已具备）
- [ ] 多源 i18n 扫描合并
- [ ] WASM runtime + SDK + 示例包

## 常用命令

```bash
cargo check
cargo test
cargo run -p deskhud-egui
cargo fmt
cargo clippy --workspace --all-targets -- -D warnings
```

## 许可证

Apache-2.0
