# DeskHud 宠物引擎路线图

当前路线只聚焦宠物引擎核心。第一版以 `deskhud-egui` 作为唯一运行态渲染后端，用它验证宠物包、行为、资源和中性场景协议；Windows、macOS、Linux 原生渲染后端暂不作为当前里程碑，待中性协议稳定后再单独评估。

## 当前目标

完成一条可验证的闭环：

```text
.deskhud 宠物包 → 包资源与行为加载 → 宿主输入/时间/配置
             → PetScene 中性场景帧 → deskhud-egui 渲染器
```

引擎不理解眼睛、身体、翅膀等宠物特征；宠物包自行决定外观、部件、动画和行为。

## 阶段

详细任务、验收标准和执行记录见 [`docs/agent/roadmap/README.md`](agent/roadmap/README.md)。

- [ ] A：冻结当前边界并清理旧 `PetPaint` 外观字段
- [ ] B：建立 `PetScene` / 资源引用 / 命中区域契约
- [ ] C：让内置 Rust 宠物输出完整场景帧
- [ ] D：完成 `deskhud-egui` 场景渲染器和真实输入链
- [ ] E：完善包资源校验、切换、配置和错误隔离
- [ ] F：以 WASM Component Model + WIT 接入社区宠物行为
- [ ] G：跨平台渲染适配器评估（后置，不承诺实现）

## 明确后置

- Windows / macOS / Linux 原生宠物渲染器
- 原生设置页迁移
- HUD 插件协议重构
- 在线商店、签名分发和权限市场

这些事项不再作为当前宠物引擎核心的前置条件。
