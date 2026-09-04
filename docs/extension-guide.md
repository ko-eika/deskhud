# DeskHud 扩展指南：宠物包与 HUD 插件

面向社区 / 内置扩展作者。引擎契约在 [`deskhud-engine`](../crates/core/deskhud-engine)；UI 壳只做几何与输入转发，**扩展代码不要依赖 HWND、屏幕坐标或 egui**。

当前阶段：

- **内置扩展**：在 `crates/packs/` 实现 `PetKind` / `Plugin`（目录名 `pet-*` / `hud-*`），由 `deskhud-runtime` 引导注册进空的 `EngineRegistry`。
- **社区扩展**：`.deskhud` 包 + WASM Component Guest（`deskhud-sdk` / `deskhud-runtime`），语义与下文 host 契约对齐。

社区宠物使用 `crates/core/deskhud-sdk/wit/pet-guest.wit` 生成 Guest 绑定：实现
`deskhud_sdk::Guest` 的 `info`、`tick`、`on_event` 与 `render`，并编译为 WASM
Component。宿主只提供中性输入和场景数据；不会提供 WASI、文件系统、网络、egui、窗口句柄或操作系统 API。
`render` 返回的 Sprite/atlas、路径、基础图形、文字、气泡和命中区域会统一转换为
`deskhud_engine::PetScene`，再由宿主校验与渲染。
路径的 `stroke-width` 与路径点使用相同的场景坐标单位，因此窗口缩放时描边会与图形同比缩放。

---

## 1. 宠物包（Pet pack）

### 1.1 职责

一套宠物包 = **外观（paint）+ 行为（tick / on_event）**。切换包即切换皮肤与行为。

### 1.2 实现入口：`PetKind`

```rust
use deskhud_engine::{PetEvent, PetKind, PetKindInfo, PetPaint, PetPaintCtx};

struct MyPet;

impl PetKind for MyPet {
    fn info(&self) -> PetKindInfo { /* id / 显示名 / 作者 / 窗尺寸 / 预览 */ }

    fn tick(&self, dt_secs: f32) { /* 自主动画、冷却计时 */ }

    fn on_event(&self, event: PetEvent) { /* 状态跳变 */ }

    fn paint(&self, ctx: PetPaintCtx<'_>) -> PetPaint { /* 每帧外观 */ }
}
```

- `PetKind` 方法是 `&self`：需要可变状态时用内部 `Mutex` / `Atomic*`。
- 预览图：`PetKindInfo.preview` 为图片字节（**svg**/png/jpeg/gif/webp）；设置页 cover 裁切显示（SVG 由壳栅格化）。
- 来源：`author` / 可选 `homepage`；设置页展示「作者 …」，悬停可看主页。

### 1.3 每帧上下文：`PetPaintCtx`

| 字段 | 含义 |
|------|------|
| `time_secs` | 运行时间 |
| `pointer_dir` | **全局**光标相对宠心方向（约 `[-1,1]`）；不要求悬停在宠上 |
| `status_line` | 引擎短文案（可空） |
| `dock` | [`DockState`](../crates/core/deskhud-engine/src/pet/dock_state.rs) 贴边（四边可组合） |
| `drag` | [`DragState`](../crates/core/deskhud-engine/src/pet/drag_state.rs) 是否在拖窗 |
| `mouse` | [`MouseState`](../crates/core/deskhud-engine/src/pet/mouse_state.rs) 局部悬停/按下 + **全局**按键 |

| `theme` | 宿主已解析的 `PetTheme::Light` / `PetTheme::Dark`；不暴露 egui 或平台主题对象 |

#### 全局 vs 局部（重要）

| 能力 | 范围 | 典型用途 |
|------|------|----------|
| `pointer_dir` | 桌面全局光标 | 默认宠「眼睛跟着鼠标」 |
| `mouse.global_*_down` / `GlobalMousePressed`/`Released` | 桌面全局按键 | 任意处按下时的紧张/反应 |
| `GlobalMouseWheel` | 桌面全局滚轮 | 滚轮刻度（正上负下） |
| `GlobalKeyPressed`/`Released` | 桌面全局键盘采样 | 修饰键 / 字母数字 / 标点 / F 键等（非完整钩子） |
| `mouse.hovering` / `Mouse*`（无 Global 前缀） | 仅宠可点区域 | 点宠、悬停高亮 |
| `Key*` | 宠窗获焦 | 焦点内按键（透明窗常难获焦，优先用 GlobalKey） |

**内置宠物 `pet.deskhud.mochi` / `pet.deskhud.sesame`**：分别提供沉稳与灵巧的吉祥物行为，包括眼神跟随、悬停、拖拽和贴边反馈。其它包可选用或忽略。

优先用上下文字段做「稳态」表现；用 `on_event` 做「边沿」触发。

### 1.4 事件：`PetEvent`

由 egui 壳在适当时机调用 `on_event`：

| 事件 | 何时 |
|------|------|
| `DragStarted` / `DragEnded` | 开始 / 结束拖动宠窗 |
| `DockChanged { from, to }` | 贴边变化（含松手吸附、切宠改尺寸后重锚定） |
| `MouseHover { inside }` | 指针进入 / 离开宠可点区域（局部） |
| `MousePressed` / `MouseReleased` | 在宠上按下 / 抬起（局部） |
| `MouseClicked` / `MouseDoubleClicked` | 单击 / 双击（局部；右键仍开引擎菜单） |
| `GlobalMousePressed` / `GlobalMouseReleased` | 桌面任意处鼠标键边沿（全局） |
| `GlobalMouseWheel { delta }` | 桌面滚轮（全局低层钩子；正=上） |
| `GlobalKeyPressed` / `GlobalKeyReleased` | 桌面键盘子集边沿（全局采样；含空格） |
| `KeyCombinationPressed` | 引擎根据修饰键状态提升的组合键按下事件 |
| `KeyPressed` / `KeyReleased` | 键盘（**需宠窗焦点**；透明 TOOLWINDOW 上常不可靠） |

键鼠类型：`PetMouseButton`、`PetKey`、`PetModifiers`（均为中性枚举，无虚拟键码）。

**键盘说明**：完整全局热键钩子不在本契约内。壳对修饰键（Ctrl/Shift/Alt/Win）、主键区字母数字、常用标点、F1–F12、方向与编辑键、小键盘数字/运算符/NumLock/扩展 Enter、PrintScreen、ScrollLock、Pause 和 ContextMenu 等做低级 Hook，并以 `GetAsyncKeyState` 边沿采样回退，派发 `GlobalKey*`；引擎会根据修饰键状态额外派发 `KeyCombinationPressed`；获焦时另有 `Key*`。

外观：`PetPaint.bubble_text` 可选短句；当前 Windows 原生后端把它绘制在宿主管理的独立透明工具窗中，超长截断，并按工作区自动选择宠物上方或下方、限制在屏幕内。后续 `PetFrame` 会增加平台无关的首选方位、气泡皮肤、透明度、尾巴与文字样式；包不得直接创建平台窗口。

宠物可以自发显示对话：在 `tick(dt)` 中累计提醒计时，到点后更新包内状态；随后由 `paint(...)` 把提示写入 `bubble_text`。宿主每帧调用 `tick` / `paint`，并自动显示、定位和隐藏对话窗。当前没有全局“定时提醒器”替包决定内容；提醒周期、随机性、冷却与文案属于宠物行为。WASM Guest 通过 `render` 返回同一中性场景契约。

`PetPaint.bubble_style` 默认 `FollowTheme`，宿主会按 `ctx.theme` 选择高对比度浅/深配色。包也可填写 `PetBubbleStyle::Custom` 的背景 RGBA、文字 RGBA 与圆角，完全使用自定义样式；包仍不得创建平台窗口。

### 1.5 贴边与拖动（壳行为，包只读状态）

- 松手靠近或拖出工作区边缘 → 壳吸附并更新 `dock`。
- 切宠导致窗尺寸变化 → 壳按原贴边边用新尺寸重锚定。
- 包内根据 `ctx.dock` / `Drag*` 做姿势、音效、表情即可。

### 1.6 包格式与导出 `.deskhud`

出厂包源在 [`crates/packs/`](../crates/packs/)（`pet-*` / `hud-*`），目录布局与分发包一致：

```text
crates/packs/pet-deskhud-mochi/   # 糯米团包源码
  Cargo.toml               # 原生实现（compile-in；不会打进 .deskhud）
  manifest.toml
  assets/preview.svg
  i18n/zh-CN/info.po
  i18n/zh-CN/config.po
  i18n/en-US/info.po
  i18n/en-US/config.po
  src/lib.rs
```

外部 WASM 包的源目录不需要提交 `guest.wasm`。打包时会自动编译 crate、生成 WASM Component，并导出 `manifest.toml` + `guest.wasm` + `assets/` + `i18n/`（不含 `src/` / `Cargo.toml`）：

```bash
# 导出 crates/packs/ 下所有带 entry 的外部 WASM 包 → target/<profile>/packages/*.deskhud
cargo pack-builtins

# 导出一个外部 WASM 包（也可传仓库外的目录）
cargo pack-external crates/packs/pet-your-pack

# 发布构建；Guest 也按 release profile 编译
cargo pack-external --release

# 导出单个目录名（crates/packs/ 下的文件夹名）
cargo pack-builtin pet-deskhud-mochi
cargo pack-builtin hud-deskhud-demo

# 自定义输出目录
cargo pack-builtins --out path/to/out
```

说明：

- 别名定义在 [`.cargo/config.toml`](../.cargo/config.toml)，实际由 `deskhud-xtask` 执行。
- `pack-external` / `pack-builtins` 只处理 manifest 声明了 `entry` 的外部包；内置包仍可用 `pack-builtin` 显式导出。
- 当前编译进程序的内置宠物清单在 [`deskhud-runtime/src/bootstrap.rs`](../crates/core/deskhud-runtime/src/bootstrap.rs) 的 `BUILTIN_PETS`；只有列在该数组中的宠物才属于内置。要外置某个宠物，需要从该数组移除，并提供带 `entry = "guest.wasm"` 的 WASM Component 包。
- 包自身可在 `manifest.toml` 中写 `load = "builtin"` 或 `load = "external"`；旧清单不写时按 `entry` 自动判断。`load` 控制打包/发现分类，但不能让外部包凭清单获得 Rust 内置实现。
- 运行时优先扫描可执行文件旁的 `packages/`，因此 debug 使用 `target/debug/packages/`，release 使用 `target/release/packages/`。
- 默认输出在当前 profile 的 `target/<profile>/packages/`（随 `cargo clean` 清理）；需要长期保留可用 `--out`。
- `pack-external` 需要本机安装 `wasm-tools`；Guest 中间产物位于 `target/deskhud-guest/`，不会写回源码包目录。

`manifest` 字段见 `deskhud-package` / [`docs/versioning.md`](./versioning.md)。

### 1.7 内置演示

- `pet.deskhud.mochi`：沉稳的鼠标跟随、眨眼、气泡、悬停、拖拽和贴边反馈。
- `pet.deskhud.sesame`：轻盈摆动、灵活鼠标跟随、拖拽回弹和方向性贴边姿态。

---

## 2. HUD 插件（HUD plugin）

### 2.1 职责

插件可贡献 **0..N** 条 HUD；用户可关整个插件意向条目，也可关单条（prefs）。

当前 host 已实现：按插件折叠 + **插件总开关** + 默认实例开关；运行态从稳定 HUD 实例解析贡献，按“总开关 ∧ 插件开关 ∧ 实例开关 ∧（若有）组开关”调用 `hud_frame_for_instance()` 并绘制中性 `HudVisual`。

### 2.2 实现入口：`Plugin`

```rust
use deskhud_engine::{HudContribution, Plugin, PluginInfo};

struct MyPlugin;

impl Plugin for MyPlugin {
    fn info(&self) -> PluginInfo {
        PluginInfo {
            id: "hud.acme.clock",
            display_name: "时钟",
            description: "…",
            author: "acme",
            homepage: None,
            version: "1.0.0",
            engine: "0.2",
            // 与包一并打包；缺省则设置页用默认首字图标
            icon: Some(include_bytes!("../assets/icon.svg")),
        }
    }

    fn hud_contributions(&self) -> &'static [HudContribution] {
        &[HudContribution {
            id: "clock",
            label: "时钟",
            default_enabled: true,
            // 条目图标按 id 对应；`None` → 程序默认图标
            icon: Some(include_bytes!("../assets/clock.svg")),
        }]
    }
}
```

- `HudContribution.id`：**插件内**唯一短名（如 `clock`）；prefs 键为 `hud.<org>.<id>.enable` / `….clock.enable`。
- 壳层用 `UiPreferences.hud.is_active(plugin_id, contribution_id, default)` 决定是否展示。
- **图标**：插件 `PluginInfo.icon` + 每条 `HudContribution.icon` 随包分发（**svg**/png/jpeg/gif/webp）；缺省用壳内默认图（插件=首字徽章，条目=简易板图标）。

### 2.3 HUD 实例契约

宿主已经用平台无关的 `HudSourceId` 区分定义来源，并用稳定的 `HudInstanceId`
区分用户实例。旧版每个 `plugin_id + contribution_id` 静态条目会映射为确定性的默认
实例；插件或 contribution 暂时缺失时，实例配置及组成员关系仍会保留。实例标题只供
显示，不能作为插件数据或持久化引用的主键。

`HudFrameCtx` 已预留当前实例、来源与宿主单调时间；运行态按实例请求帧。宿主测量帧后，
使用中性的 `HudGroupLayout::compose` 得到横向、纵向或网格排列的成员矩形和裁剪矩形，
或通过 `compose_free` 应用宿主持久化的自由布局矩形，再将整个组作为单一屏幕槽渲染；
布局模式负责创建组、拖放成员和调整实例样式，插件不会接触 egui、窗口句柄或 OS 类型。实例配置以及
社区 Guest HUD ABI 仍在后续阶段接入。现有原生插件继续使用：

- `Plugin::hud_frame(...)` → 中性 `HudFrame`（文本 / 进度等）

社区 HUD Guest 侧仍对应 `deskhud-sdk::PluginGuest` / `HudItem`（`icon` 为包内相对路径）；HUD ABI 尚未纳入阶段 F。

### 2.4 包格式与导出

与宠物包相同，用 `cargo pack-builtins` / `cargo pack-builtin <目录名>` 从 `crates/packs/` 导出。HUD 示例：

```text
crates/packs/hud-deskhud-demo/
  manifest.toml    # kind = "plugin"
  assets/icon.svg
  assets/icon_clock.svg
  assets/icon_tip.svg
  i18n/…
  src/lib.rs       # 不进入 .deskhud
```

社区分发包的最终形态包含生成的 `guest.wasm`：

```text
my-hud.deskhud/
  manifest.toml    # kind = "plugin"
  guest.wasm
  assets/icon.svg
  assets/clock.svg
  i18n/en-US/info.mo
  i18n/en-US/config.mo
```

扩展使用 gettext 资源：源码目录按职责拆分为 `i18n/<locale>/info.po` 和
`i18n/<locale>/config.po`，发布包对应为 `.mo`。PO 必须是 UTF-8；每个
`msgid` 使用 DeskHud 的 i18n 键，空的 `msgstr` 会安全地视为缺失翻译。语言标签
支持 `en-US`、`en`、`zh-CN`、`zh_CN` 等写法，运行时会自动发现并在设置页中
显示新语言。

从 0.9.0 起，包内国际化只支持上述 PO/MO 目录格式；旧版
`i18n/<locale>.toml` 文件不再读取。

`manifest.toml` 示例：

```toml
id = "hud.acme.clock"
kind = "plugin"
version = "1.0.0"
engine = "0.9"
api_version = 4
display_name = "时钟"
icon = "assets/icon.svg"

[[hud]]
id = "clock"
icon = "assets/clock.svg"
```

`[[hud]].id` 必须与 Guest / `HudContribution.id` 一致，引擎据此加载条目图标。

---

## 3. 标识（ID）与配置键

### 3.1 全 ID（元数据必填）

| 类型 | 格式 | 示例 |
|------|------|------|
| 宠物包 | `pet.<组织>.<标识>` | `pet.deskhud.mochi` |
| HUD 插件 | `hud.<组织>.<标识>` | `hud.deskhud.demo` |
| HUD 条目 | 插件内短名（再拼到全 ID 后） | `clock` → 配置键见下 |

`PetKindInfo.id` / `PluginInfo.id` / `manifest.toml` 的 `id` **必须**写全 ID；显示名可重复，全 ID 全局唯一。

### 3.2 怎么取，避免和别人冲突

1. **组织（org）**  
   - 用你能长期控制的名字：GitHub 用户/组织、域名去掉点（`example.com` → `example_com`）、或作者 handle。  
   - 仅小写字母数字与 `_` `-`。  
   - **保留**：`deskhud` 仅官方内置包使用；社区包不要占。

2. **标识（id）**  
   - 在你的组织下唯一：包用途短名，如 `specs`、`cool_cat`、`cpu_meter`。  
   - 可以和别人叫一样的 `demo` / `clock`，只要 **组织不同** 就不会撞：`hud.acme.demo` ≠ `hud.deskhud.demo`。

3. **HUD 条目短名**  
   - 只在**本插件内**唯一（`clock`、`tip`），不要再写组织前缀。

4. **加载冲突**  
   - 当前注册表同全 ID 会后写覆盖；正式加载社区包时应拒绝重复并提示。

### 3.3 用户配置（prefs）

分类落盘（键序稳定）；仅读取当前配置格式，无法读取的字段使用默认值：

```toml
[prefs]
"settings.size" = [720.0, 520.0]

[theme]
mode = "system"
locale = "zh-cn"

[font]
id = "Inter"
family = "jetbrainsmono"
style = "Regular"
size = 13.0

[pet]
"pet.global.kind" = "pet.deskhud.mochi"
"pet.global.size" = [140.0, 140.0]
"pet.global.layer" = "top"
"pet.global.picker_mode" = "grid"
"pet.deskhud.mochi.follow_eyes" = true
"pet.deskhud.mochi.key_tips" = false

[hud]
"hud.global.enable" = true
"hud.deskhud.demo.enable" = true
"hud.deskhud.demo.clock.enable" = true
"hud.deskhud.demo.tip.enable" = false
"hud.deskhud.demo.tip.display" = "primary"
"hud.deskhud.demo.tip.x" = 0.54
"hud.deskhud.demo.tip.y" = 0.82
"hud.deskhud.demo.tip.size" = [1.0, 1.0]
```

节顺序固定：`[settings]` → `[theme]`（含 mode / locale）→ `[font]` → `[pet]` → `[hud]`。  
`[pet]` / `[hud]`：先写全部 `*.global.*`，再按引擎注册的宠/插件顺序写出；同包内 `id` / `enable` 优先，其余按包内配置定义序，布局属性（`display` / `x` / `y` / `size`）靠后，其中 `size` 为 `[width, height]`。
| 分区 / 键 | 含义 |
|-----------|------|
| `[theme]` / `[font]` / `[prefs]` | 主题、字体、设置窗几何 |
| `[pet]` | 当前宠、尺寸/位置/层级 + 包选项扁平键 |
| `[hud]` | HUD 层级、插件/条目开关 + 布局扁平键 |
| `pet.<组织>.<标识>.<键>` | 宠物包自定义配置（由 `PetKind::config_options` 声明） |

宠配置在设置 → 宠物页下部「当前宠物行为」；设置打开时草稿会即时预览。

### 3.4 注意

- 完整插件/宠物 id **勿互为前缀**（不要同时注册 `hud.acme` 与 `hud.acme.clock` 两个插件）。  
- i18n 前缀与全 ID 对齐（规划中）。  
- 配置中的未知字段和旧格式不会迁移，读取不到当前字段时使用默认值。

---

## 4. 硬约束（必读）

1. **禁止**在扩展里依赖 egui、`git2`、任意原生 dll 分发（社区默认 WASM）。
2. **禁止**引擎 crate 依赖 `deskhud-sdk`；sdk 仅 examples / 社区包使用。
3. 第三方 crate 版本只写在仓库根 `[workspace.dependencies]`。
4. UI 是唯一壳：`deskhud-egui`；无托盘、无第二套 UI 框架。
5. 一能力一目录；新建模块优先独立目录（见仓库根 `AGENTS.md`）。

---

## 5. 本地开发检查清单

```powershell
cargo check -p deskhud-engine
cargo test -p deskhud-engine
cargo run -p deskhud-egui
```

1. 实现 `PetKind` 或 `Plugin`，由 runtime / 包加载调用 `register_pet` / `register_plugin`。
2. 宠物：验证贴边 / 拖动 / 悬停 / 单击获焦后键盘。
3. HUD：设置 → **插件** 按插件折叠列表开关条目；运行态验证启用贡献返回的 `HudFrame` 内容、位置与缩放。
4. 宠物：设置 → 宠物页切换包，并验证「当前宠物行为」开关（跟眼 / 提示等）。

更细架构与路线图：[`architecture.md`](./architecture.md)、[`roadmap.md`](./roadmap.md)。

版本与适配政策见 [`versioning.md`](./versioning.md)。
