# Codex 项目配置

此目录预留给 DeskHud 的项目级 Codex 配置。仅在信任该仓库时，Codex 才会加载 `.codex/config.toml`。

- 项目协作规则：[`../AGENTS.md`](../AGENTS.md)
- 现行实现约束：[`../docs/agent/CONSTRAINTS.md`](../docs/agent/CONSTRAINTS.md)
- 决策追溯：[`../docs/agent/MEMORY.md`](../docs/agent/MEMORY.md)

不要把规则复制到这里，也不要通过 `project_doc_fallback_filenames` 试图附加加载 `CONSTRAINTS.md`：它只在同目录不存在 `AGENTS.md` 时才生效。
