# DuckCoding 一键配置工具

<div align="center">

![DuckCoding Logo](src/assets/duck-logo.png)

**一键安装和配置 AI 编程工具的桌面应用**

支持 Claude Code、CodeX、Gemini CLI

[![GitHub Release](https://img.shields.io/github/v/release/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)]()
[![GitHub Downloads](https://img.shields.io/github/downloads/DuckCoding-dev/DuckCoding/total)](https://github.com/DuckCoding-dev/DuckCoding/releases)
[![GitHub Stars](https://img.shields.io/github/stars/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/network/members)
[![GitHub Issues](https://img.shields.io/github/issues/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/issues)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/DuckCoding-dev/DuckCoding/pulls)

</div>

## ✨ 功能特性

- **🚀 一键安装** - 自动安装 Claude Code、CodeX、Gemini CLI
- **⚙️ 一键配置** - 快速配置 DuckCoding API 或自定义 API 端点
- **📊 用量统计** - 实时查看账户余额和30天用量趋势
- **🔑 一键生成令牌** - 直接在应用内创建 DuckCoding API 令牌
- **🔄 多配置管理** - 支持保存和切换多个配置文件
- **🎨 现代界面** - 基于 React + Tailwind CSS 的精美 UI
- **💻 跨平台** - 支持 macOS (Intel/Apple Silicon)、Windows、Linux

## 📥 下载安装

前往 [Releases 页面](https://github.com/DuckCoding-dev/DuckCoding/releases) 下载适合你系统的安装包：

### 桌面应用（推荐）

- **macOS Universal**: `DuckCoding-macOS-Universal.dmg` (支持 Intel 和 Apple Silicon M1/M2/M3/M4/M5)
- **Windows x64**: `DuckCoding-Windows-x64-setup.exe` 或 `DuckCoding-Windows-x64.msi`
- **Linux x64**:
  - Debian/Ubuntu: `DuckCoding-Linux-x64.deb`
  - Fedora/RHEL: `DuckCoding-Linux-x64.rpm`
  - 通用: `DuckCoding-Linux-x64.AppImage`

### 平台支持说明

✅ **完全支持**:

- Windows 10/11 (x64) - 原生桌面应用
- macOS 10.15+ (Intel x64 和 Apple Silicon ARM64) - 通用二进制
- Linux (x64) - Debian/Ubuntu/Fedora/RHEL 及其他发行版

❌ **不支持**:

- **Windows WSL / Linux WSL**: WSL环境无法运行GUI桌面应用
  - **解决方案**: WSL用户请使用Windows原生版本（.msi/.exe）

📝 **CLI模式支持**:
如果你只需要命令行功能（不需要GUI），可以使用 Rust CLI（需要从源码编译 `cargo build --features cli --bin duckcoding`），支持所有平台包括WSL。

## 🎯 使用方法

### 1. 安装工具

在「安装工具」标签页选择需要安装的 AI 编程工具：

- **Claude Code** - Anthropic 官方 AI 编程助手
- **CodeX** - OpenAI 官方代码生成工具
- **Gemini CLI** - Google Gemini 命令行工具

点击「安装」按钮即可自动安装。

### 2. 配置全局设置（可选）

如果你想使用用量统计和一键生成令牌功能：

1. 访问 [DuckCoding 控制台](https://duckcoding.com/console/token)
2. 点击右上角头像 → 个人中心
3. 获取「用户ID」和「系统访问令牌」
4. 在应用的「控制台」标签页点击「配置全局设置」填入

### 3. 配置 API

在「配置 API」标签页：

#### 方式一：一键生成（推荐）

1. **选择工具** - 选择要配置的工具
2. **点击「一键生成」** - 自动创建对应的专用分组令牌并配置

#### 方式二：手动配置

1. **选择工具** - 选择要配置的工具
2. **选择提供商**
   - **DuckCoding** - 使用 DuckCoding API（需要专用分组令牌）
   - **自定义** - 使用自己的 API 端点
3. **输入 API 密钥** - 填写你的 API 密钥
4. **保存配置** - 可选：为配置命名以便后续切换

### 4. 查看用量

在「控制台」标签页：

- **余额显示** - 查看账户总额度、已用额度、剩余额度
- **用量图表** - 查看最近30天的用量趋势
- **请求统计** - 查看总请求次数

### 5. 切换配置

在「切换配置」标签页：

- 查看所有已保存的配置
- 一键切换到不同的配置文件

## 🔑 关于 DuckCoding API 令牌

### 专用分组说明

DuckCoding 要求每个工具使用对应的专用分组令牌：

| 工具        | 必须选择的分组           |
| ----------- | ------------------------ |
| Claude Code | **Claude Code 专用分组** |
| CodeX       | **CodeX 专用分组**       |
| Gemini CLI  | **Gemini CLI 专用分组**  |

❌ **不能混用**：不同工具的专用分组令牌不能互相使用

✅ **一键生成**：应用会自动为你创建正确的专用分组令牌

### 手动获取令牌

如果需要手动创建令牌：

1. 访问 [DuckCoding 令牌管理](https://duckcoding.com/console/token)
2. 点击「创建令牌」
3. 选择对应工具的专用分组
4. 复制生成的令牌到应用中配置

## 🛠️ 技术栈

### 桌面应用

- **前端**: React 19 + TypeScript + Tailwind CSS
- **桌面框架**: Tauri 2.0
- **后端**: Rust（完整服务层架构）
  - InstallerService - 工具安装和版本管理
  - VersionService - 版本检查（npm registry API）
  - ConfigService - 配置文件管理（增量更新）
  - CommandExecutor - 跨平台命令执行
- **图表**: Recharts
- **UI 组件**: Shadcn/ui + Radix UI
- **构建工具**: Vite

### Rust CLI (可选)

- **CLI 框架**: clap + inquire
- **共享服务层**: 与桌面应用相同的 Rust 服务层
- **编译**: `cargo build --features cli --bin duckcoding`

## 📖 配置文件说明

应用会在以下位置创建配置文件：

### Claude Code

- **位置**: `~/.claude/settings.json`
- **格式**: JSON
- 只更新 API 相关字段，保留其他自定义配置

### CodeX

- **位置**: `~/.codex/config.toml` + `~/.codex/auth.json`
- **格式**: TOML + JSON
- 保存模型提供商配置和认证信息

### Gemini CLI

- **位置**: `~/.gemini/.env`（主要配置）+ `~/.gemini/settings.json`（认证设置）
- **格式**: ENV + JSON
- 主要配置在 `.env` 文件，`settings.json` 仅用于指定认证类型

## 🔒 隐私和安全

- ✅ **不收集用户数据** - 所有配置保存在本地
- ✅ **不上传配置文件** - 应用包不包含任何用户配置
- ✅ **安全存储** - 配置文件权限设置为仅所有者可读写 (0600)
- ✅ **开源透明** - 所有代码公开可审查

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

## 📄 许可证

[MIT License](LICENSE)

## 🔗 相关链接

- [DuckCoding 官网](https://duckcoding.com)
- [DuckCoding 控制台](https://duckcoding.com/console)
- [Claude Code 文档](https://docs.claude.com/claude-code)
- [OpenAI CodeX](https://openai.com/codex)
- [Google Gemini](https://ai.google.dev)

## ⚠️ 免责声明

本工具仅用于简化 AI 编程工具的安装和配置流程，不提供 API 服务本身。使用第三方 API 服务时请遵守其服务条款。

---

## ⭐ Star History

<div align="center">

<a href="https://star-history.com/#DuckCoding-dev/DuckCoding&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=DuckCoding-dev/DuckCoding&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=DuckCoding-dev/DuckCoding&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=DuckCoding-dev/DuckCoding&type=Date" />
 </picture>
</a>

</div>

---

<div align="center">

Made with ❤️ by DuckCoding

[官网](https://duckcoding.com) · [反馈问题](https://github.com/DuckCoding-dev/DuckCoding/issues)

</div>
