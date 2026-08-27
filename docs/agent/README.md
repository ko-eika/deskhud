# Agent 文档目录

跨工具（Cursor / Codex / Claude Code / 其它）共用。任意智能体接手本仓库时按下面顺序读。

## 必读顺序

1. **[`AGENTS.md`](../../AGENTS.md)**（仓库根）— 协作入口：产品、架构概览、范围、命令。
2. **[`CONSTRAINTS.md`](./CONSTRAINTS.md)** — **现行实现约束**（与入口同级必读，动手前必看）。
3. **[`MEMORY.md`](./MEMORY.md)** — 决策时间线（结论 + 理由）；**不是**现行规则全文。
4. 按任务再读：
   - 架构总览 → [`docs/architecture.md`](../architecture.md)
   - 写包 / 扩展 → [`docs/extension-guide.md`](../extension-guide.md)
   - 改版本 → [`docs/versioning.md`](../versioning.md)
   - 发版 → [`docs/release.md`](../release.md)

## 真相源（防漂移）

| 内容 | 权威位置 | 说明 |
|------|----------|------|
| 产品 / 架构概览 / 命令 | `AGENTS.md` | 入口叙述 |
| **现行硬约束** | `docs/agent/CONSTRAINTS.md` | 改约束只改这里 |
| 决策时间线 | `docs/agent/MEMORY.md` | 只追加行 |
| Cursor 自动注入摘要 | `.cursor/rules/*.mdc` | 薄指针 → AGENTS / CONSTRAINTS |
| Cursor 旧记忆路径 | `.cursor/MEMORY.md` | 仅跳转到 `MEMORY.md` |

## 当前路线图与验收状态

宠物引擎计划、里程碑状态和验收记录统一放在 [`roadmap/`](./roadmap/README.md)。不要在 `tasks/` 下新增计划文件。

## 历史任务索引

| 任务文件 | 状态 | 说明 |
|----------|------|------|
旧任务内容已迁移至 [`roadmap/`](./roadmap/README.md)，`tasks/` 不再作为计划入口。
| [`../window-layers.md`](../window-layers.md) | 定稿 | 四类独立窗口（宠物/气泡/HUD信息/HUD布局）的分层交互说明 |

## 变更约定

- 改架构叙述、产品范围、命令 → 更新 **`AGENTS.md`**。
- 改窗口 / HUD / 分层 / 工程硬约束 → 更新 **`CONSTRAINTS.md`**，并在 **`MEMORY.md`** 追加一行。
- 不要把「已失败的实验」写成现行规则；失败结论可记入 MEMORY，现行做法写在 CONSTRAINTS。
- 内置包 `manifest.toml` 的 `version` 跟程序 `workspace.package.version`；`engine` 跟兼容族（见 versioning）。
