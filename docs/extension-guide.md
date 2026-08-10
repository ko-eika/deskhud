# DeskHud 扩展指南：宠物包与 HUD 插件

面向社区 / 内置扩展作者。引擎契约在 [`deskhud-engine`](../crates/deskhud-engine)；UI 壳只做几何与输入转发，**扩展代码不要依赖 HWND、屏幕坐标或 egui**。

当前阶段：

- **内置扩展**：在 `packs/` 实现 `PetKind` / `Plugin`（目录名 `pet-*` / `hud-*`），由 `deskhud-runtime` 引导注册进空的 `EngineRegistry`。
- **社区扩展（规划）**：`.deskhud` 包 + WASM Guest（`deskhud-sdk` / `deskhud-runtime`），语义与下文 host 契约对齐。

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
| `dock` | [`DockState`](../crates/deskhud-engine/src/pet/dock_state.rs) 贴边（四边可组合） |
| `drag` | [`DragState`](../crates/deskhud-engine/src/pet/drag_state.rs) 是否在拖窗 |
| `mouse` | [`MouseState`](../crates/deskhud-engine/src/pet/mouse_state.rs) 局部悬停/按下 + **全局**按键 |

#### 全局 vs 局部（重要）

| 能力 | 范围 | 典型用途 |
|------|------|----------|
| `pointer_dir` | 桌面全局光标 | 默认宠「眼睛跟着鼠标」 |
| `mouse.global_*_down` / `GlobalMousePressed`/`Released` | 桌面全局按键 | 任意处按下时的紧张/反应 |
| `GlobalMouseWheel` | 桌面全局滚轮 | 滚轮刻度（正上负下） |
| `GlobalKeyPressed`/`Released` | 桌面全局键盘采样 | 修饰键 / 字母数字 / 标点 / F 键等（非完整钩子） |
| `mouse.hovering` / `Mouse*`（无 Global 前缀） | 仅宠可点区域 | 点宠、悬停高亮 |
| `Key*` | 宠窗获焦 | 焦点内按键（透明窗常难获焦，优先用 GlobalKey） |

**默认宠物 `pet.deskhud.specs`**：全局光标跟眼；短提示气泡（`左键` / `Ctrl+Shift+X` / `滚轮↑`）；悬停高亮。其它包可选用或忽略。

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
| `KeyPressed` / `KeyReleased` | 键盘（**需宠窗焦点**；透明 TOOLWINDOW 上常不可靠） |

键鼠类型：`PetMouseButton`、`PetKey`、`PetModifiers`（均为中性枚举，无虚拟键码）。

**键盘说明**：完整全局热键钩子不在本契约内。壳对修饰键（Ctrl/Shift/Alt/Win）、字母数字、常用标点、F1–F12、方向与编辑键等做 `GetAsyncKeyState` 边沿采样并派发 `GlobalKey*`；获焦时另有 `Key*`。

外观：`PetPaint.bubble_text` 可选短句；壳按宠窗宽度换行（最多约 3 行，过长加 `…`）。

### 1.5 贴边与拖动（壳行为，包只读状态）

- 松手靠近或拖出工作区边缘 → 壳吸附并更新 `dock`。
- 切宠导致窗尺寸变化 → 壳按原贴边边用新尺寸重锚定。
- 包内根据 `ctx.dock` / `Drag*` 做姿势、音效、表情即可。

### 1.6 包格式与导出 `.deskhud`

出厂包源在仓库根 [`packs/`](../packs/)（`pet-*` / `hud-*`），目录布局与分发包一致：

```text
packs/pet-deskhud-specs/   # 示例
  Cargo.toml               # 原生实现（compile-in；不会打进 .deskhud）
  manifest.toml
  assets/preview.svg
  i18n/zh-CN.toml
  i18n/en.toml
  src/lib.rs
```

导出 **仅** 打包 `manifest.toml` + `assets/` + `i18n/`（不含 `src/` / `Cargo.toml`）：

```bash
# 导出 packs/ 下全部出厂包 → target/packages/*.deskhud
cargo pack-builtins

# 导出单个目录名（packs/ 下的文件夹名）
cargo pack-builtin pet-deskhud-specs
cargo pack-builtin hud-deskhud-demo

# 自定义输出目录
cargo pack-builtins --out path/to/out
```

说明：

- 别名定义在 [`.cargo/config.toml`](../.cargo/config.toml)，实际由 `deskhud-xtask` 执行。
- 运行时仍以原生 crate **compile-in**；导出的 `.deskhud` 用于规范校验 / 对照社区包布局。
- 社区 WASM 包另含 `guest.wasm`（Phase 3）；本地扫描根为 [`packages/`](../packages/)。
- 默认输出在 `target/packages/`（随 `cargo clean` 清理）；需要长期保留可用 `--out`。

`manifest` 字段见 `deskhud-package` / [`docs/versioning.md`](./versioning.md)。

### 1.7 内置演示

- `pet.deskhud.specs`：全局跟鼠标看；短提示（`左键` / `Ctrl+Shift+X` / `滚轮↑`）；悬停高亮；贴边变色。
- `pet.deskhud.blob`：贴边 / 拖动 / 悬停轻量反馈。

---

## 2. HUD 插件（HUD plugin）

### 2.1 职责

插件可贡献 **0..N** 条 HUD；用户可关整个插件意向条目，也可关单条（prefs）。

当前 host 已实现：按插件折叠 + **插件总开关** + 条目开关；壳在宠窗底部画演示条。**真实 HUD 帧数据**仍在路线图中。

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

### 2.3 规划中的帧数据

后续将增加类似：

- `Plugin::hud_frame(...)` → 中性 `HudFrame`（文本 / 进度等）

社区 WASM 侧对应 `deskhud-sdk::PluginGuest` / `HudItem`（`icon` 为包内相对路径）。

### 2.4 包格式与导出

与宠物包相同，用 `cargo pack-builtins` / `cargo pack-builtin <目录名>` 从 `packs/` 导出。HUD 示例：

```text
packs/hud-deskhud-demo/
  manifest.toml    # kind = "plugin"
  assets/icon.svg
  assets/icon_clock.svg
  assets/icon_tip.svg
  i18n/…
  src/lib.rs       # 不进入 .deskhud
```

社区目标形态（WASM，规划）额外包含 `guest.wasm`：

```text
my-hud.deskhud/
  manifest.toml    # kind = "plugin"
  guest.wasm
  assets/icon.svg
  assets/clock.svg
  i18n/en.toml
```

`manifest.toml` 示例：

```toml
id = "hud.acme.clock"
kind = "plugin"
version = "1.0.0"
engine = "0.2"
api_version = 1
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
| 宠物包 | `pet.<组织>.<标识>` | `pet.deskhud.specs` |
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

分类落盘（键序稳定）；旧 `[shell]` / `[pet.config]` / `[hud.config]` 启动时自动迁移：

```toml
[settings]
width = 720.0
height = 520.0

[theme]
mode = "system"
locale = "zh-cn"

[font]
id = "JetBrainsMono-Regular"
family = "jetbrainsmono"
style = "Regular"
size = 13.0

[pet]
"pet.global.kind" = "pet.deskhud.specs"
"pet.global.width" = 140.0
"pet.global.height" = 140.0
"pet.global.topmost" = true
"pet.global.picker_mode" = "grid"
"pet.deskhud.specs.follow_eyes" = true
"pet.deskhud.specs.key_tips" = false

[hud]
"hud.global.enable" = true
"hud.deskhud.demo.enable" = true
"hud.deskhud.demo.clock.enable" = true
"hud.deskhud.demo.tip.enable" = false
"hud.deskhud.demo.tip.display" = "primary"
"hud.deskhud.demo.tip.x" = 0.54
"hud.deskhud.demo.tip.y" = 0.82
"hud.deskhud.demo.tip.scale" = 1.0
```

节顺序固定：`[settings]` → `[theme]`（含 mode / locale）→ `[font]` → `[pet]` → `[hud]`。  
`[pet]` / `[hud]`：先写全部 `*.global.*`，再按引擎注册的宠/插件顺序写出；同包内 `id` / `enable` 优先，其余按包内配置定义序，布局属性（`display` / `x` / `y` / `scale`）靠后。
| 分区 / 键 | 含义 |
|-----------|------|
| `[ui]` | 主题、字体、设置窗几何 |
| `[pet]` | 当前宠、尺寸/位置/置顶 + 包选项扁平键 |
| `[hud]` | 插件/条目开关 + 布局扁平键 |
| `pet.<组织>.<标识>.<键>` | 宠物包自定义配置（由 `PetKind::config_options` 声明） |

宠配置在设置 → 宠物页下部「当前宠物行为」；设置打开时草稿会即时预览。

### 3.4 注意

- 完整插件/宠物 id **勿互为前缀**（不要同时注册 `hud.acme` 与 `hud.acme.clock` 两个插件）。  
- i18n 前缀与全 ID 对齐（规划中）。  
- 旧 id（`builtin.specs`、`demo.hud` 等）加载 prefs 时会迁移/兼容读取。

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
3. HUD：设置 → **插件** 按插件折叠列表开关条目；演示条画在宠窗底部。
4. 宠物：设置 → 宠物页切换包，并验证「当前宠物行为」开关（跟眼 / 提示等）。

更细架构与路线图：[`architecture.md`](./architecture.md)、[`roadmap.md`](./roadmap.md)。

版本与适配政策见 [`versioning.md`](./versioning.md)。
