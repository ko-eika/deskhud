# DeskHud 架构

## 目标模型

用户安装 / 选择：

- **宠物包（pet pack）**：皮肤资源 + 行为逻辑；同一时间一个激活宠。
- **HUD 插件（hud plugin）**：可贡献 0..N 条 HUD；prefs 支持「插件总开关」与「单条 HUD 开关」。

社区作者用 `deskhud-sdk` 编写逻辑，打成 `.deskhud`；宿主用 `deskhud-runtime` 本地加载。内置实现走原生 Rust，但实现同一套 `deskhud-host` 契约，对 UI 无感。

## 依赖方向（禁止反向）

```
deskhud-egui
  → deskhud-runtime
       → deskhud-host
       → deskhud-package
       → deskhud-ui
deskhud-sdk          （仅示例 / 社区包依赖；宿主不依赖 sdk）
```

- UI 不得依赖 `git2`、不得引入第二套 UI。
- 领域能力只通过 `Plugin` / 宠物包进入。
- 第三方版本只写在根 `[workspace.dependencies]`。

## 包格式（约定）

```text
my-cool-pet.deskhud/
  manifest.toml
  guest.wasm          # 社区包；内置可无此文件而由 host 原生注册
  assets/             # 可选皮肤资源
  i18n/
    en.toml
    zh-CN.toml
```

`manifest.toml` 核心字段：

- `id`：稳定全 ID — 宠物 `pet.<组织>.<标识>`，插件 `hud.<组织>.<标识>`
- `kind`：`pet` | `plugin`
- `api_version`：与宿主 ABI 对齐
- `display_name` / `description` / `author` / `homepage`
- `icon`：包图标相对路径；插件另可 `[[hud]]` 声明条目 `id` + `icon`
- `preview`：设置页预览图（宠物）
- `entry`：wasm 入口相对路径（社区包）

## 宠物：皮肤 + 行为

Host 契约（当前）：

- `tick(dt)`：自主状态 / 动画（默认空）
- `on_event(PetEvent)`：贴边 / 拖拽 / 键鼠（`DockChanged`、`Drag*`、`Mouse*`、`Key*`）
- `config_options()` / `apply_config`：声明布尔行为开关；prefs 键 `{pet_id}.{key}`
- `paint(PetPaintCtx)`：输出 `PetPaint`；可读 `dock` / `drag` / `mouse` / `config`
- UI 壳负责工作区几何、拖动吸附与输入转发；宠物包不读 HWND / 屏幕坐标

作者向说明见 [`extension-guide.md`](./extension-guide.md)。

演进目标：更完整的中性 `PetFrame`；社区 WASM Guest 实现同一形状。

## HUD 插件

- 声明 `HudContribution[]`（id、默认开、标签、可选 `icon_png`）
- `PluginInfo.icon_png`：插件图标；条目图标按 contribution id 对应
- 每帧或按需产出 `HudFrame`（已启用条目的展示数据，仍在路线图）
- prefs（`[hud.config]`）：
  - `hud.<org>.<id>.enable`：插件总开关
  - `hud.<org>.<id>.<item>.enable`：单条开关
  - 插件关 → 其下全部不显示

## 设置窗

侧栏顺序：**常规 → 宠物 → 插件**；从右键菜单打开时默认落在 **常规**。

## 国际化

1. 启动 / 加载包时扫描：
   - 外壳：`deskhud-ui` 内置目录
   - 每个宠物包 / 插件包：`i18n/<locale>.toml`
2. 合并进 `CatalogStore`，键命名空间：
   - `shell.*`
   - `pet.<pack_id>.*`
   - `plugin.<pack_id>.*`
3. 用户改语言 → 只切换查询 locale，目录已加载则无需重编译。
4. 回退：请求 locale → `en` → 键名本身。

## 安全边界（社区包）

- 默认仅 WASM 能力（算逻辑 + 读包内资源）。
- 不开放任意 OS / 网络，除非日后做显式权限模型。
- 禁止社区原生 dll。
