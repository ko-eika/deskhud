# DeskHud 路线图

宠物引擎核心阶段 F 已完成；下一条主动产品路线是 HUD 组与首批正式插件。第一版继续以 `deskhud-egui` 作为唯一运行态渲染后端；Windows、macOS、Linux 原生渲染后端暂不作为当前里程碑，待中性协议稳定后再单独评估。

## 已完成目标：宠物引擎闭环

完成一条可验证的闭环：

```text
.deskhud 宠物包 → 包资源与行为加载 → 宿主输入/时间/配置
             → PetScene 中性场景帧 → deskhud-egui 渲染器
```

引擎不理解眼睛、身体、翅膀等宠物特征；宠物包自行决定外观、部件、动画和行为。

## 阶段

详细任务、验收标准和执行记录见 [`docs/agent/roadmap/README.md`](agent/roadmap/README.md)。

- [x] A：冻结当前边界并清理旧 `PetPaint` 外观字段
- [x] B：建立 `PetScene` / 资源引用 / 命中区域契约
- [x] C：让内置 Rust 宠物输出完整场景帧
- [x] D：完成 `deskhud-egui` 场景渲染器和真实输入链
- [x] E：完善包资源校验、切换、配置和错误隔离
- [x] F：以 WASM Component Model + WIT 接入社区宠物行为
- [ ] G：跨平台渲染适配器评估（后置，不承诺实现）

## 当前目标：HUD 与首批正式插件

严格按以下顺序实施：

1. HUD 组基础能力：建立 HUD 实例身份、跨插件分组、组内排版、兼容迁移和设置/布局编辑。
2. 系统插件：提供真实的系统 CPU、内存以及 DeskHud/指定应用进程性能信息。
3. 便签插件：基于动态 HUD 实例创建多个独立便签，并允许加入任意 HUD 组。

详细任务与验收标准见 [`docs/agent/roadmap/03-hud-groups-and-first-plugins.md`](agent/roadmap/03-hud-groups-and-first-plugins.md)。演示插件删除不属于该计划。

## 明确后置

- Windows / macOS / Linux 原生宠物渲染器
- 原生设置页迁移
- 在线商店、签名分发和权限市场

这些事项不再作为当前宠物引擎核心的前置条件。
