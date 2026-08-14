# DeskHud 架构

## 目标模型

用户安装 / 选择：

- **宠物包（pet pack）**：皮肤资源 + 行为逻辑；同一时间一个激活宠。
- **HUD 插件（hud plugin）**：可贡献 0..N 条 HUD；prefs 支持「插件总开关」与「单条 HUD 开关」。

社区作者用 `deskhud-sdk` 编写逻辑，打成 `.deskhud`；引擎用 `deskhud-runtime` 本地加载。内置实现走原生 Rust，但实现同一套 `deskhud-engine` 契约，对 UI 无感。

## 依赖方向（禁止反向）

```
deskhud-egui
  → deskhud-runtime
       → deskhud-engine
       → deskhud-package
       → deskhud-ui
deskhud-sdk          （仅示例 / 社区包依赖；引擎不依赖 sdk）
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
- `version`：包自身 SemVer（展示 / 更新比较）
- `engine`：引擎兼容族（加载门闸；见 [`versioning.md`](./versioning.md)）
- `api_version`：与引擎 Guest ABI 对齐
- `display_name` / `description` / `author` / `homepage`
- `icon`：包图标相对路径；插件另可 `[[hud]]` 声明条目 `id` + `icon`
- `preview`：设置页预览图（宠物）
- `entry`：wasm 入口相对路径（社区包）

## 宠物：皮肤 + 行为

Host 契约（当前）：

- `tick(dt)`：自主状态 / 动画（默认空）
- `on_event(PetEvent)`：贴边 / 拖拽 / 键鼠（`DockChanged`、`Drag*`、`Mouse*`、`Key*`）
- `config_options()` / `apply_config`：声明布尔行为开关；prefs 键 `{pet_id}.{key}`
- `paint(PetPaintCtx)`：输出 `PetPaint`；可读 `dock` / `drag` / `mouse` / `config` / 已解析的 `theme`
- UI 壳负责工作区几何、拖动吸附与输入转发；宠物包不读 HWND / 屏幕坐标
- 对话气泡由宿主管理的独立透明工具窗承载，避免频繁扩缩宠物合成面；未来包通过中性视觉字段自定义首选方位、背景、透明度、尾巴与文字样式，壳负责屏幕避让、穿透和生命周期。它是逻辑子窗而非受父客户区裁剪的 `WS_CHILD`，包不创建平台窗口

作者向说明见 [`extension-guide.md`](./extension-guide.md)。

演进目标：更完整的中性 `PetFrame`；社区 WASM Guest 实现同一形状。

## HUD 插件

- 声明 `HudContribution[]`（id、默认开、标签、可选 `icon`：svg/png/jpeg/gif/webp）
- `PluginInfo.icon`：插件图标；条目图标按 contribution id 对应；设置页由壳栅格化 SVG
- 每帧或按需产出 `HudFrame`（已启用条目的展示数据，仍在路线图）
- prefs：
  - `[ui]`：主题 / 字体 / 设置窗几何
  - `[pet]`：当前宠、尺寸位置置顶 + `pet.<org>.<id>.*` 选项
  - `[hud]`：`hud.<org>.<id>[.<item>].enable` 与布局 `display/x/y/scale`
  - 插件关 → 其下全部不显示
  - 可继续加同前缀自定义键，便于扩展
- 布局编辑：关闭设置后打开 Windows 原生 D3D11 + DirectComposition 全屏编辑视口；按任务栏工作区绘制半透明安全边界，选中 HUD 使用半透明实线边框，右下角显示三角缩放角标。当前先支持主屏，布局修改写入 `HudSlotLayout`；运行态 HUD 使用每屏一个原生合成窗。

## 原生桌面覆盖层迁移

当前 Windows 运行态由原生 GPU 宠物、菜单、气泡和 HUD 窗口组成，设置页暂由 egui 控制窗承载。各平台只通过平台无关的覆盖层契约接入；阶段、降级原则和验收条件见 [`overlay-migration.md`](./overlay-migration.md)。不再以全屏 Glow 主窗或 eframe 回退路径替代原生后端。

## 原生窗口迁移边界

平台后端最终负责宠物主窗口、气泡/对话框、HUD 合成窗口、HUD 布局编辑窗口和右键菜单的窗口生命周期、透明合成、命中与输入。`deskhud-engine` 及扩展仍只消费平台无关场景和事件契约。

设置窗暂保留 `winit + egui_glow`，明确标记为迁移期 legacy 控制 UI，而不是最终 UI 架构。菜单不再复用设置控制窗，也不再由 egui 绘制；菜单使用平台原生菜单后端，并通过壳层命令桥打开设置、切换 HUD、修改置顶或退出。后续原生设置页必须复用平台无关的设置模型与命令，不得把 egui 类型带入通用 crate。

原生 UI 迁移按以下边界推进：

1. `deskhud-ui` 只承载设置模型、命令、偏好、目录和国际化，不依赖 egui 或平台类型。
2. `deskhud-egui` 只作为迁移期 legacy 适配层，负责把设置模型绘制成现有设置页；不得新增正式业务能力。
3. `platform/` 下的平台 crate 负责原生设置窗口、窗口生命周期、输入和显示器能力；Windows 使用 WinUI，macOS 使用 AppKit，Linux 使用 GTK。
4. 宠物、气泡、HUD、布局编辑和菜单继续由平台覆盖层承载，不回退到 egui。

## 设置窗

侧栏顺序：**常规 → 宠物 → 插件 → 关于**；从右键菜单打开时默认落在 **常规**。

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

版本与适配政策见 [`versioning.md`](./versioning.md)。
