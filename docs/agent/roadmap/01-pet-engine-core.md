# 宠物引擎核心实施计划

状态：阶段 E 已完成
当前版本：0.6.25
首个运行态后端：`deskhud-egui`

## 目标

建立一套真正由宠物包定义外观和行为的引擎：

- 引擎不认识眼睛、身体、翅膀等特征；
- 宠物包输出平台无关的 `PetScene`；
- `deskhud-egui` 解释 `PetScene` 并完成首版运行态；
- 内置 Rust 与未来 WASM 宠物共享同一套输入、行为和场景语义。

### 动画范围

首版优先稳定两种动画方式：

- **矢量动画**：宠物包通过基础路径/图形、`Transform2D`、颜色和透明度，在每帧计算并输出场景；
- **序列帧动画**：宠物包使用独立图片、Sprite sheet 或 atlas frame，按状态和时间选择当前帧。

动画状态机、播放进度、循环、事件响应和状态切换由宠物包行为管理，宿主只负责校验和渲染 `PetScene`。2D 骨骼、网格蒙皮、粒子、物理、复杂动画混合和 3D 模型先记录为后续扩展，不进入首版场景契约；必要时可先将 3D 动画预渲染为序列帧。

## 阶段 A：清理现有契约

- [x] 删除 `PetPaint` 的 `eye_*`、`draw_eyes`、`bounce` 等固定外观字段。
- [x] 将 `PetKind` 的职责收敛为宠物程序生命周期契约。
- [x] 明确 `PetPaintCtx`（输入快照）与 `PetEvent` 的边界；`PetCommand` 留待场景/宿主命令阶段定义。
- [x] 修复 egui 路径中 `pointer_dir`、`drag`、`mouse` 使用固定空值的问题；贴边仍由窗口壳在后续阶段接入。
- [x] 使用实际 `dt` 驱动更新，不依赖固定 `1.0 / 60.0`。

验收：引擎契约和 egui 绘制器中不再出现眼睛专用字段；真实指针、拖拽、贴边和鼠标状态能到达宠物程序。

## 阶段 B：建立中性场景帧

- [x] 增加 `PetScene`、`SceneNode`、`AssetId`、`Transform2D`、颜色和透明度。
- [x] 第一版支持 Sprite、atlas frame、基础路径、基础图形、文字和 z-index。
- [x] 明确首版动画表达：序列帧由 Sprite/atlas frame 表达，矢量动画由路径/图形参数和 `Transform2D` 的逐帧变化表达。
- [x] 增加气泡和局部命中区域的中性描述。
- [x] 场景帧定义节点数量、字符串长度、坐标和浮点数校验规则。
- [x] 引擎契约不依赖 egui、窗口句柄或任何 OS 类型。

验收：可以构造一个没有眼睛的宠物场景，也可以构造多个眼睛节点；引擎对两者没有特殊分支；至少能用序列帧和矢量节点表达一个带状态切换的简单动画。

## 阶段 C：迁移内置宠物

- [x] 将大眼球的身体、眼白、瞳孔和眨眼帧改为包内资源或普通场景节点。
- [x] 将眨眼、跟随指针、拖拽反馈、贴边反馈保留在宠物包行为中。
- [x] 用宠物包自己的状态和时间控制首版动画，不在宿主内置宠物动画状态机。
- [x] 清理 `deskhud-egui` 中固定的两眼绘制逻辑。
- [x] 内置 Blob 宠物也输出同一种 `PetScene`。
- [x] 设置页预览继续只使用包的 `preview` / `icon`。

验收：删除或替换眼睛资源后，宿主不会自动补画眼睛；切换宠物不会继承前一个宠物的视觉状态。

## 阶段 D：完成 egui 首版运行链

- [x] 新增 `EguiSceneRenderer`，只负责解释 `PetScene`。
- [x] 接通真实指针方向、局部鼠标、全局输入、贴边和拖拽状态。
- [x] 统一 tick、事件、render 的调用顺序和线程模型。
- [x] 验证透明窗口、命中区域、点击、拖动、贴边和气泡。
- [x] 为场景节点、资源缺失和异常宠物增加测试/日志。

验收：同一个内置宠物包能够在 egui 运行态正常加载、动画、交互和切换。

## 阶段 E：包与运行时闭环

- [x] manifest 增加或明确资源索引和入口约束。
- [x] 明确序列帧/atlas 资源的声明、帧引用、尺寸边界和加载失败行为；首版不要求宿主理解骨骼或 3D 资源。
- [x] 校验包内路径，拒绝路径穿越、未声明资源和损坏资源。
- [x] 宠物实例切换时清理旧实例状态和资源引用。
- [x] 宠物异常时隔离错误，不阻塞宿主主循环。
- [x] 增加目录包、zip 包、缺失资源和不兼容包测试。

验收：坏包不会导致宿主崩溃；包切换、重载、禁用和恢复行为可重复验证。

## 阶段 F：社区 WASM Guest

- [ ] 用 WIT 定义宠物 Guest/Host 接口。
- [ ] 用 Wasmtime Component Model 加载 Guest。
- [ ] `deskhud-sdk` 只生成 Guest 侧绑定，不让 engine 依赖 SDK。
- [ ] 默认关闭文件系统、网络和任意 WASI 能力。
- [ ] 增加 fuel、内存、节点数量和调用耗时限制。
- [ ] 内置 Rust 宠物和 WASM 宠物都转换为相同的 `PetScene`。
- [ ] 确认 WASM 宠物可以在资源和节点限制内驱动矢量动画与序列帧动画；骨骼/网格/粒子/物理/3D 留作后续 ABI 扩展。

验收：一个最小社区宠物包可在不接触 egui、窗口和操作系统 API 的情况下完成动画和绘制。

## 阶段 G：原生后端评估

此阶段不承诺实现，只在前面协议稳定后评估：

- [ ] 统计 `PetScene` 基础节点在 Windows/macOS/Linux 的覆盖率。
- [ ] 确认 egui 与原生后端的资源、文字和透明度语义一致。
- [ ] 按平台独立实现 `PetScene` renderer，不能修改宠物包协议来适配单个平台。
- [ ] 明确 Linux X11/Wayland 的透明和命中能力差异。

## 调用顺序

```text
采集宿主状态 → apply_config → dispatch events → update(dt)
             → render(input) → validate PetScene → EguiSceneRenderer
```

## 当前不做

- 不在宠物包中使用 egui。
- 不让宠物包创建窗口或操作平台句柄。
- 不把 Bevy、wgpu 等完整渲染框架暴露给宠物包。
- 不在首版支持宿主直接播放 2D 骨骼、网格蒙皮、粒子、物理或 3D 模型；这些方式只保留扩展空间，3D 可先通过预渲染序列帧接入。
- 不先做原生设置页或原生宠物窗。

## 执行记录

- 状态：阶段 E 已完成
- 开始日期：2026-08-27
- 完成日期：2026-08-27
- 实际改动：新增 EguiSceneRenderer、局部命中与事件派发、阈值拖拽、跨平台贴边与 macOS 全局指针/鼠标输入；运行态按 config→events→tick→scene→validate→renderer 运行；气泡由独立透明工具窗承载并按工作区避让。
- 验证命令：`cargo fmt --all`、`cargo check -p deskhud-egui -p pet-deskhud-specs -p pet-deskhud-blob`、`cargo test -p deskhud-engine`、`cargo run -p deskhud-egui`（启动后手动中断）。
- 阶段 E 改动：新增 `PackResource`/atlas 帧索引、包内路径与入口门闸、位图损坏/尺寸/帧边界校验；目录包和 ZIP 在发现、打包、打开三个入口统一校验；新增 `PackInstance`/`PetInstanceSlot`，切换失败会清理旧实例；坏包按单包失败隔离。
- 阶段 E 验证命令：`cargo fmt --all`、`cargo test -p deskhud-package -p deskhud-runtime`、`cargo check -p deskhud-egui`。
- 遗留问题：WASM Guest runtime 属于阶段 F；全 workspace 检查仍受既有 windows-future/windows-core 依赖错配影响。
