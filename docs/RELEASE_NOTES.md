# MarkLite

轻量、离线的 Markdown 编辑器，支持多标签、实时分栏预览、PDF 与 Word 导出。

## 下载

- **Windows x64**：`MarkLite-0.1.0-windows-x64.zip`
- **Linux x86_64**：`MarkLite-0.1.0-linux-x86_64.tar.gz`
- **macOS（Universal / Intel + Apple Silicon）**：`MarkLite-0.1.0-macos-universal.zip`
- `SHA256SUMS.txt`：所有发布文件的校验和

## 功能

- 编辑 / 分栏 / 预览三种视图，编辑内容实时联动预览
- 多标签页、多选打开、拖入多个文件、重复文件去重
- 每个标签独立的未保存状态、光标/滚动/撤销历史
- 关闭标签和退出时保存确认，支持全部保存
- 中英文界面；外观跟随系统 / 浅色 / 深色；5 套主题色
- 文件 / 查看 / 导出 / 帮助菜单与工具栏图标
- PDF 导出依赖本机 Edge / Chrome / Chromium；Word 导出无需 Office
- UTF-8 / GB18030 读取，原子保存

## 运行说明

### Windows

解压后直接运行 `mdlite.exe`。这是未签名构建，Windows SmartScreen 可能提示“更多信息 → 仍要运行”。

### Linux

```bash
tar -xzf MarkLite-0.1.0-linux-x86_64.tar.gz
cd MarkLite
./mdlite                     # 直接运行
./install.sh                 # 可选：安装到 ~/.local
```

需要 X11 / XWayland、OpenGL 驱动，以及 XDG Desktop Portal（文件对话框）。PDF 导出需要本机安装 Chrome / Chromium / Edge。

### macOS

解压后将 `MarkLite.app` 拖到“应用程序”。该构建未经 Developer ID 签名和公证；首次启动请右键应用选择“打开”，或到“系统设置 → 隐私与安全性”中允许。

macOS 版为 Universal 二进制，同时支持 Intel 与 Apple Silicon。PDF 导出需要本机安装 Chrome / Chromium / Edge。

## 已知限制

- 分栏内容实时联动，但编辑区与预览的滚动位置尚未同步
- 不恢复上次打开的标签页，不支持标签拖拽排序
- 数学公式、Mermaid 和原始 HTML 暂不渲染
- 极长文档的预览尚未做视口虚拟化

完整规划见仓库中的 `docs/PLAN.md`。
