# DeskHud 内置字体

`build.rs` 会扫描本目录下全部 `ttf` / `otf` / `ttc` 并嵌入二进制；文件名按 `家族-样式`（如 `JetBrainsMono-BoldItalic.ttf`）解析，同家族多文件会出现在设置「样式」下拉中。

与系统同名字体按家族键合并：**同样式优先内置**，系统独有样式作互补。

| 文件 | 字体 | 来源 | 许可 |
|------|------|------|------|
| `NotoSansSC-*.otf` | Noto Sans SC（Thin / Light / DemiLight / Regular / Medium / Bold / Black） | [notofonts/noto-cjk](https://github.com/notofonts/noto-cjk) SubsetOTF/SC（Sans2.004） | SIL OFL 1.1（`OFL-NotoSansSC.txt`） |
| `JetBrainsMono-*.ttf` | JetBrains Mono | [JetBrains/JetBrainsMono](https://github.com/JetBrains/JetBrainsMono) | SIL OFL 1.1（`OFL-JetBrainsMono.txt`） |
| `JetBrainsMonoNL-*.ttf` | JetBrains Mono NL（无连字） | 同上 | 同上 |

二者均可免费用于开源与商业软件捆绑分发；不可单独出售字体文件。随应用分发时保留对应 OFL 文本。注意：嵌入全部字重会显著增大可执行文件体积。
