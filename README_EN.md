# DuckCoding – Unified Management Platform for AI Coding Tools

<div align="center">

![DuckCoding Logo](src/assets/duck-logo.png)

**Cross-platform desktop app for one-click installation & configuration of Claude Code / CodeX / Gemini CLI**

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

## Table of Contents

- Project Overview
- Core Features
- Feature Preview
- DuckCoding vs Manual Configuration
- Quick Start
- Usage Guide (Scenarios)
- Detailed Features
- Design & Architecture
- FAQ
- Development Guide
- Configuration Files
- Privacy & Security / License / Links / Star History

## Project Overview

- Unified management of Claude Code, CodeX, and Gemini CLI; auto-detects npm/brew/official installs
- Multi-environment instances (Local/WSL/SSH) with centralized detection, installation, updates, and status
- Multi-profile isolation—only API fields are swapped; all other native settings remain intact
- Three independent transparent proxies with per-session config, history, auto-start, and loopback prevention
- Balance monitoring & provider management with templates + custom extractors
- Fully local storage (`~/.duckcoding`)

## Core Features

- **Tool Management**: auto-detect/install/update; supports Local/WSL/SSH; recognizes install method and validates version
- **Profile Management**: isolated profiles; native sync only replaces API Key / Base URL
- **Transparent Proxy**: three independent ports (8787/8788/8789); per-session config; auto-start; loopback detection
- **Balance Monitoring**: multi-provider; preset templates (NewAPI, OpenAI) + custom JS extractors; configurable auto-refresh
- **Provider Management**: unified API provider config; one-click switch & validation in Dashboard
- **Advanced Settings**: launch at login, single-instance, log level/format/file output

## Feature Preview

<div align="center">

![Dashboard](docs/screenshots/dashboard.png)
![Tool Management](docs/screenshots/tool-management.png)
![Profile Management](docs/screenshots/profile-management.png)
![Transparent Proxy](docs/screenshots/transparent-proxy.png)

</div>

## DuckCoding vs Manual Configuration

| Capability          | Manual Configuration                 | DuckCoding                                              |
| ------------------- | ------------------------------------ | ------------------------------------------------------- |
| Tool Install/Update | Manual npm/brew/official commands    | One-click install & version detection                   |
| Profile Switch      | Hand-edit JSON/TOML/ENV              | UI one-click switch with profile isolation              |
| Multi-Environment   | Configure one by one                 | Unified Local/WSL/SSH management                        |
| Proxy               | Restart required after config change | UI start/stop; independent proxies; loopback protection |
| Balance Monitoring  | DIY scripts calling APIs             | Preset templates + custom extractors + visual dashboard |
| Learning Curve      | Must understand each tool’s config   | GUI + versioned onboarding tour                         |

## Quick Start

1. **Download**  
   Grab the latest release at: https://github.com/DuckCoding-dev/DuckCoding/releases
   - macOS Universal: `DuckCoding-macOS-Universal.dmg`
   - Windows x64: `DuckCoding-Windows-x64-setup.exe` (recommended) or `.msi`
   - Linux x64: `.deb` / `.rpm` / `.AppImage`

2. **Platform Support**
   - Supported: Windows 10/11 x64, macOS 10.15+ (Intel/Apple Silicon), Linux x64
   - Not supported: WSL GUI (use native Windows installer instead)

3. **First Launch**
   - Auto onboarding: Welcome → Proxy Setup → Tool Intro → Done; re-open anytime via “Settings → About”

## Usage Guide (Scenarios)

- **Scenario 1**: Quick Claude Code proxy  
  Fill API Key / Base URL in Transparent Proxy → Start → confirm traffic in Session History.
- **Scenario 2**: Multi-account switch (work/personal)  
  Create/import profiles in “Profile Management” → activate; only API fields change, themes/shortcuts kept.
- **Scenario 3**: Multi-env tool management  
  Add Local/WSL/SSH instances in “Tool Management” → view versions & update all in one place.
- **Scenario 4**: Multi-provider balance monitoring  
  Configure APIs in “Provider Management” → select provider in Dashboard to see balance/trends → set refresh interval.
- **Scenario 5**: Custom extractor  
  Use JS extractor in “Balance Monitoring” to parse any response → save template → auto-refresh display.

## Detailed Features

### Tool Management

- Auto-detect installed tools (with install method), manual refresh supported
- One-click install/update; records install method (npm/brew/official), version validation
- Data stored in: `~/.duckcoding/tools.json`

### Profile Management (Profile)

- Dual files: `~/.duckcoding/profiles.json` + `~/.duckcoding/active.json`
- Import/export; native sync only replaces API Key / Base URL, keeps themes/shortcuts
- Supports Claude Code (settings.json + config.json), Codex (config.toml + auth.json), Gemini CLI (.env + settings.json)

### Transparent Proxy

- Default ports: 8787 (Claude Code) / 8788 (CodeX) / 8789 (Gemini CLI)
- Per-session config temporarily overrides global; reusable history; auto-start
- Data stored in: `~/.duckcoding/proxy.json`; session history: `~/.duckcoding/sessions.db`

### Balance Monitoring

- Preset templates (NewAPI, OpenAI) + custom JS extractors
- Configurable refresh interval; visual balance/usage/expiry display
- Data stored in: `~/.duckcoding/balance.json`

### Provider Management

- Manage multiple API providers, quick switch in Dashboard
- Config validation & status display
- Data stored in: `~/.duckcoding/providers.json`

### Advanced Settings

- Launch at login, single-instance toggle

## Design & Architecture

- **Design goals**: KISS/DRY/YAGNI, minimize user ops, focus on “install-config-proxy-monitor” loop
- **Frontend**: React 19 + TypeScript + Vite + Tailwind + shadcn/ui; page components (Dashboard, ToolManagement, Profile, Proxy, Balance, Providers, Settings)
- **Backend**: Tauri 2 + Rust; three-layer (Commands → Services → Utils); trait-based extensibility (ToolDetector, ToolConfigManager, HeadersProcessor)
- **Data**: unified DataManager reads/writes JSON/TOML/ENV/SQLite with atomic writes, checksums, caching
- **Profile isolation**: Profile v2 dual-file (profiles.json + active.json), swaps only API fields, keeps native personalization
- **Quality**: `npm run check` single entry (ESLint + Clippy + Prettier + fmt), CI 4-platform matrix

## FAQ

### Installation

- Windows “Publisher cannot be verified”? Currently unsigned, click “More info” → “Run anyway”.
- macOS “Unidentified developer”? Right-click open or allow in “System Settings → Privacy & Security”.
- Linux no execute permission? `chmod +x DuckCoding-Linux-x64.AppImage` then run.
- WSL support? No GUI in WSL; use native Windows installer.

### Configuration

- Will activating a profile change themes/shortcuts? No, only API Key & Base URL are overwritten.
- How to backup? Use “Profile Management → Export” or simply back up `~/.duckcoding/`.
- Config paths? See “Configuration Files” section.

### Proxy

- Port conflict? Change ports in “Transparent Proxy → Proxy Settings”.
- How to verify proxy works? Run AI tool then check new records in “Session History”.

### Updates

- Update DuckCoding? “Settings → Update” check & install.
- Update AI tools? Click “Update” button on each card in “Tool Management”.
- Data loss? No, everything lives in `~/.duckcoding/`.

### Misc

- Telemetry? None, fully local storage, open-source auditable.
- Why is API Key masked? UI shows redacted for safety; full key still in config file.
- Issues? https://github.com/DuckCoding-dev/DuckCoding/issues

## Development Guide

### Prerequisites

- Node.js 20.19+
- Rust 1.70+
- System deps:
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools (MSVC)
  - Linux: `build-essential`, `libwebkit2gtk-4.1-dev`, `libjavascriptcoregtk-4.1-dev`, `libssl-dev`, `patchelf`

### Quick Start

```bash
git clone https://github.com/DuckCoding-dev/DuckCoding.git
cd DuckCoding
npm install
npm run tauri dev
```

### Key Commands

- `npm run check` / `npm run check:fix` (ESLint + Clippy + Prettier + fmt)
- `npm run tauri dev` / `npm run tauri build`
- `npm run test:rs` / `npm run coverage:rs`

### Tech Stack

- Three layers: Commands → Services → Utils
- Trait-based: ToolDetector / ToolConfigManager / HeadersProcessor
- Unified data: DataManager (JSON/TOML/ENV/SQLite)

### Project Structure

- Frontend: `src/` (React + shadcn/ui)
- Backend: `src-tauri/src/` (commands / services / utils / models / data / setup)
- Docs: `docs/` (includes `screenshots/`)

### Contributing

- Commit style: Conventional Commits, description in Simplified Chinese
- PR must include: motivation, main changes, test status, risk assessment
- Pre-merge: pass `npm run check` & `cargo test --locked`

## Configuration Files (`~` = user home)

- DuckCoding: `~/.duckcoding/config.json`, `profiles.json`, `active.json`, `tools.json`, `proxy.json`, `balance.json`, `providers.json`, `sessions.db`, `logs/`
- Claude Code: `~/.claude/settings.json`, `~/.claude/config.json`
- CodeX: `~/.codex/config.toml`, `~/.codex/auth.json`
- Gemini CLI: `~/.gemini/.env`, `~/.gemini/settings.json`

## Privacy & Security

- No data collection/reporting, config files chmod 0600
- Fully open-source & auditable under AGPL-3.0

## License

[GNU Affero General Public License v3.0](LICENSE)

## Links

- Website: https://duckcoding.com
- Console: https://duckcoding.com/console
- Issues: https://github.com/DuckCoding-dev/DuckCoding/issues
- Contributing: CLAUDE.md

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
[Website](https://duckcoding.com) · [Report Issues](https://github.com/DuckCoding-dev/DuckCoding/issues)
</div>
