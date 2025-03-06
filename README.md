# LDUI - Linux Do 论坛终端界面

LDUI（Linux Do UI）是一个基于终端的 Linux Do 论坛交互界面，使用 Rust 语言开发，允许用户在命令行环境中浏览论坛帖子。

## 功能特点

- 终端友好的用户界面，基于 [ratatui](https://github.com/ratatui-org/ratatui) 构建
- 支持 Linux Do 论坛的主要功能：
  - 浏览帖子列表和帖子内容
  - 支持图片显示 (需要终端支持 Sixel 协议)
- API 密钥生成器，简化认证流程

## 安装

### 从源码安装

确保你已安装 Rust 开发环境，然后执行：

```bash
# 克隆仓库
git clone https://github.com/spoonnotfound/ldui.git
cd ldui

# 编译项目
cargo build --release

# 安装
cargo install --path .
```

## 使用方法

### 命令行选项

- `ldui` - 启动 LDUI 终端界面
- `ldui --generate-api-key` 或 `ldui -g` - 启动 API 密钥生成器
- `ldui --help` 或 `ldui -h` - 显示帮助信息

### 快捷键

在应用内部，你可以使用以下快捷键：

- 方向键（h/j/k/l）：导航
- `q`：退出应用
- `?`：查看帮助

## 贡献指南

欢迎提交 Pull Request 或创建 Issue 来改进项目。

## 许可证

本项目采用 [MIT] 许可证 - 详见 LICENSE 文件。
