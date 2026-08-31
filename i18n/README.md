# DeskHud i18n

程序外壳的 PO 源文件按职责拆分：

- `i18n/<locale>/interface.po`：界面、菜单和导航文案
- `i18n/<locale>/info.po`：关于页、元信息和介绍文案
- `i18n/<locale>/settings.po`：设置项和选项文案
- `i18n/<locale>/keys.po`：键盘按键名称和按键提示文案

构建阶段将 PO 编译为 `target/<profile>/i18n/<locale>/*.mo`。运行时只读取 MO。
宠物包和 HUD 插件使用相同的 `i18n/<locale>/` 目录约定，其中 `info.po` 记录
名称/介绍，`config.po` 记录配置项和条目文案。
