# DuckCoding - AI 编程工具统一管理平台

<div align="center">

![DuckCoding Logo](src/assets/duck-logo.png)

**一键安装与配置 Claude Code / CodeX / Gemini CLI 的跨平台桌面应用**

[![GitHub Release](https://img.shields.io/github/v/release/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/releases)
[![License: AGPL v3](https://img.shields.io/badge/License-AGPL_v3-blue.svg)](LICENSE)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-lightgrey)]()
[![GitHub Downloads](https://img.shields.io/github/downloads/DuckCoding-dev/DuckCoding/total)](https://github.com/DuckCoding-dev/DuckCoding/releases)
[![GitHub Stars](https://img.shields.io/github/stars/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/stargazers)
[![GitHub Forks](https://img.shields.io/github/forks/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/network/members)
[![GitHub Issues](https://img.shields.io/github/issues/DuckCoding-dev/DuckCoding)](https://github.com/DuckCoding-dev/DuckCoding/issues)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](https://github.com/DuckCoding-dev/DuckCoding/pulls)

</div>

English version: [README_EN.md](README_EN.md)

## 目录

- 项目概览
- 核心功能
- 功能预览
- DuckCoding vs 手动配置
- 快速开始
- 使用指南（场景）
- 功能详解
- 设计与架构
- 常见问题（FAQ）
- 开发指南
- 配置文件说明
- 隐私与安全 / 许可证 / 相关链接 / Star History

## 项目概览

- 统一管理 Claude Code、CodeX、Gemini CLI，自动识别 npm/brew/官方安装方式
- 多环境实例（Local/WSL/SSH），集中检测、安装、更新与状态查看
- 多配置隔离（Profile），仅替换 API 字段，原生其他设置全部保留
- 三工具独立透明代理，会话级配置、历史记录、自启动、防回环
- 余额监控与供应商管理，模板 + 自定义提取器
- 全量本地存储（`~/.duckcoding`）

## 核心功能

- 工具管理：自动检测/安装/更新；支持 Local/WSL/SSH；识别安装方式并校验版本
- 配置管理（Profile）：多配置隔离；原生同步仅替换 API Key / Base URL
- 透明代理：三工具独立端口（8787/8788/8789）；会话级配置；自启动；回环检测
- 余额监控：多供应商；预设模板（NewAPI、OpenAI）+ 自定义 JS 提取器；可配置自动刷新
- 供应商管理：统一 API 供应商配置，Dashboard 一键切换与校验
- 高级设置：开机自启、单实例、日志级别/格式/文件输出

## 功能预览

<div align="center">

![Dashboard](docs/screenshots/dashboard.png)
![Tool Management](docs/screenshots/tool-management.png)
![Profile Management](docs/screenshots/profile-management.png)
![Transparent Proxy](docs/screenshots/transparent-proxy.png)

</div>

## DuckCoding vs 手动配置

| 能力          | 手动配置                   | DuckCoding                       |
| ------------- | -------------------------- | -------------------------------- |
| 工具安装/更新 | 手动执行 npm/brew/官方命令 | 一键安装与版本检测               |
| 配置切换      | 手改 JSON/TOML/ENV         | UI 一键切换，Profile 隔离        |
| 多环境        | 逐一配置                   | 统一管理 Local/WSL/SSH           |
| 代理          | 修改配置要重启             | UI 启停，三工具独立代理，防回环  |
| 余额监控      | 自写脚本调用 API           | 预设模板 + 自定义提取器 + 可视化 |
| 学习成本      | 需理解各工具配置格式       | 图形界面 + 版本化新手引导        |

## 快速开始

1. 下载  
   前往 Releases 获取最新包：https://github.com/DuckCoding-dev/DuckCoding/releases

- macOS Universal: `DuckCoding-macOS-Universal.dmg`
- Windows x64: `DuckCoding-Windows-x64-setup.exe`（推荐）或 `.msi`
- Linux x64: `.deb` / `.rpm` / `.AppImage`

2. 平台支持

- 支持：Windows 10/11 x64、macOS 10.15+（Intel/Apple Silicon）、Linux x64
- 不支持：WSL GUI（请使用 Windows 原生安装包）

3. 首次启动

- 自动进入新手引导：欢迎 → 代理配置 → 工具介绍 → 完成；可在“设置 → 关于”重新开启

## 使用指南（场景）

- 场景一：Claude Code 快速代理  
  透明代理中填写 API Key / Base URL → 启动代理 → 在“会话历史”确认请求流量。
- 场景二：多账号切换（工作/个人）  
  “Profile 管理”新建/导入配置 → 激活后仅替换 API 字段，主题/快捷键保留。
- 场景三：多环境工具管理  
  “工具管理”添加 Local/WSL/SSH 实例 → 统一查看版本与更新 → 支持一键安装/更新。
- 场景四：多供应商余额监控  
  “供应商管理”配置 API → Dashboard 选择供应商查看余额/趋势 → 支持自定义刷新间隔。
- 场景五：自定义提取器  
  在“余额监控”使用 JS 提取器解析任意响应 → 保存模板 → 自动刷新展示。

## 功能详解

### 工具管理

- 自动检测已安装工具（含安装方式），支持手动刷新
- 一键安装/更新；记录安装方式（npm/brew/官方），支持版本校验
- 数据存储：`~/.duckcoding/tools.json`

### 配置管理（Profile）

- 双文件：`~/.duckcoding/profiles.json` + `~/.duckcoding/active.json`
- 导入/导出；原生同步仅替换 API Key / Base URL，保留主题/快捷键等个性化设置
- 支持 Claude Code（settings.json + config.json）、Codex（config.toml + auth.json）、Gemini CLI（.env + settings.json）

### 透明代理

- 默认端口：8787（Claude Code）/ 8788（CodeX）/ 8789（Gemini CLI）
- 会话级配置临时覆盖全局配置；历史会话可复用；代理可自启动
- 数据存储：`~/.duckcoding/proxy.json`；会话历史：`~/.duckcoding/sessions.db`

### 余额监控

- 预设模板（NewAPI、OpenAI）+ 自定义 JS 提取器
- 可配置刷新间隔；可视化展示余额、用量、到期时间
- 数据存储：`~/.duckcoding/balance.json`

### 供应商管理

- 管理多个 API 供应商，Dashboard 快速切换
- 配置校验与状态展示
- 数据存储：`~/.duckcoding/providers.json`

### 高级设置

- 开机自启、单实例开关

## 设计与架构

- 设计目标：KISS/DRY/YAGNI，最小化用户操作成本，聚焦“安装-配置-代理-监控”闭环
- 前端：React 19 + TypeScript + Vite + Tailwind + shadcn/ui；页面组件化（Dashboard、ToolManagement、Profile、Proxy、Balance、Providers、Settings）
- 后端：Tauri 2 + Rust；三层架构（Commands → Services → Utils）；Trait-based 扩展（ToolDetector、ToolConfigManager、HeadersProcessor）
- 数据管理：统一 DataManager 读写 JSON/TOML/ENV/SQLite，自动原子写入与校验和缓存
- 配置隔离：Profile v2 双文件架构（profiles.json + active.json），只替换 API 字段，保留原生个性化配置
- 质量保障：`npm run check` 统一入口（ESLint + Clippy + Prettier + fmt），CI 四平台矩阵

## 常见问题（FAQ）

### 安装

- Windows 提示“无法验证发布者”？当前未签名，点击“更多信息”→“仍要运行”。
- macOS 显示“来自身份不明的开发者”？右键打开或在“系统设置 → 隐私与安全”中允许。
- Linux 无执行权限？先 `chmod +x DuckCoding-Linux-x64.AppImage`，再运行。
- WSL 可以用吗？不行，WSL 无法运行 GUI，请使用 Windows 原生安装包。

### 配置

- 激活 Profile 会改变主题/快捷键吗？不会，只覆盖 API Key 与 Base URL。
- 如何备份配置？使用“配置管理 → 导出”或直接备份 `~/.duckcoding/`。
- 配置路径？见“配置文件说明”章节。

### 代理

- 端口被占用？在“透明代理 → 代理设置”修改端口。
- 如何验证代理生效？执行 AI 工具后在“会话历史”查看是否有新记录。

### 更新

- 更新 DuckCoding？“设置 → 更新”检查并安装。
- 更新 AI 工具？在“工具管理”点击对应卡片的“更新”按钮。
- 数据会丢失吗？不会，数据存储在 `~/.duckcoding/`。

### 其他

- 是否收集用户数据？不收集，全部本地存储，开源可审计。
- API Key 为什么打码？为安全起见 UI 只展示脱敏内容，完整 Key 在配置文件中可查。
- 遇到问题如何反馈？https://github.com/DuckCoding-dev/DuckCoding/issues

## 开发指南

### 环境要求

- Node.js 20.19+
- Rust 1.70+
- 系统依赖：
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools (MSVC)
  - Linux: `build-essential`、`libwebkit2gtk-4.1-dev`、`libjavascriptcoregtk-4.1-dev`、`libssl-dev`、`patchelf`

### 快速开始

```bash
git clone https://github.com/DuckCoding-dev/DuckCoding.git
cd DuckCoding
npm install
npm run tauri dev
```

### 核心命令

- `npm run check` / `npm run check:fix`（ESLint + Clippy + Prettier + fmt）
- `npm run tauri dev` / `npm run tauri build`
- `npm run test:rs` / `npm run coverage:rs`

### 技术架构

- 三层：Commands → Services → Utils
- Trait-based：ToolDetector / ToolConfigManager / HeadersProcessor
- 统一数据管理：DataManager（JSON/TOML/ENV/SQLite）

### 项目结构

- 前端：`src/`（React + shadcn/ui）
- 后端：`src-tauri/src/`（commands / services / utils / models / data / setup）
- 文档：`docs/`（含 `screenshots/`）

### 贡献指南

- 提交规范：Conventional Commits，描述使用简体中文
- PR 需包含：动机、主要改动、测试情况、风险评估
- 合并前必须通过：`npm run check`、`cargo test --locked`

## 配置文件说明（`~` 为用户主目录）

- DuckCoding：`~/.duckcoding/config.json`、`profiles.json`、`active.json`、`tools.json`、`proxy.json`、`balance.json`、`providers.json`、`sessions.db`、`logs/`
- Claude Code：`~/.claude/settings.json`、`~/.claude/config.json`
- CodeX：`~/.codex/config.toml`、`~/.codex/auth.json`
- Gemini CLI：`~/.gemini/.env`、`~/.gemini/settings.json`

## 隐私与安全

- 不收集/不上报用户数据，配置文件权限 0600
- 完全开源可审计，AGPL-3.0 保障用户权益

## 许可证

[GNU Affero General Public License v3.0](LICENSE)

## 相关链接

- 官网：https://duckcoding.com
- 控制台：https://duckcoding.com/console
- 问题反馈：https://github.com/DuckCoding-dev/DuckCoding/issues
- 贡献指南：CLAUDE.md

## Star History

<div align="center">
<a href="https://star-history.com/#DuckCoding-dev/DuckCoding&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=DuckCoding-dev/DuckCoding&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=DuckCoding-dev/DuckCoding&type=Date" />
   <img alt="Star History Chart" src="https://api.star-history.com/svg?repos=DuckCoding-dev/DuckCoding&type=Date" />
 </picture>
</a>
</div>

<div align="center">
Made with ?? by DuckCoding  
[官网](https://duckcoding.com) · [反馈问题](https://github.com/DuckCoding-dev/DuckCoding/issues)
</div>
