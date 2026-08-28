# DeskHud 全局字体资源

该目录存放 DeskHud 可随软件发布的外置字体文件。应用不会把字体嵌入二进制，而是从可执行文件同目录的 `fonts/` 目录递归加载。Cargo 构建和运行时会把这里的内容复制到 `target/<profile>/fonts/`。

默认字体家族：`Source Han Sans`（按家族名选择，不要求固定文件名或目录层级）

推荐目录结构：

```text
fonts/
  Source Han Sans/
    zh-CN/
      Regular.ttc
      Bold.ttc
    ja/
      Regular.ttc
  Inter/
    Regular.ttc
```

文件也可以直接放在 `fonts/` 根目录；支持 `.ttf`、`.otf` 和 `.ttc`，目录会递归扫描。

`C:\Users\eika\Downloads\Inter-4.1\Inter.ttc`
