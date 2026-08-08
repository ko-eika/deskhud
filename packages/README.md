# 本地包目录

宿主通过 [`deskhud-runtime`](../crates/deskhud-runtime) 扫描本目录（以及 `%APPDATA%/DeskHud/packages`）。

每个子目录为一个包根，至少包含：

```text
my-pack/
  manifest.toml
  guest.wasm      # 社区 WASM 包（Phase 3）
  i18n/           # 可选
    zh-CN.toml
    en.toml
  assets/         # 可选皮肤资源
```

开发时可把示例打包输出直接放到这里做本地加载验证。
