---
agents: Codex, Claude-Code, Gemini-Cli
last-updated: 2025-12-16
---

# DuckCoding 开发协作规范

> 本文档为指导 AI AGENT 的开发协作规范，同时也作为 AI AGENT 开发指南和持久化项目记忆存在。
> 本文档作为项目记忆文档，需要及时更新。**请务必在开发完成后根据代码的实际情况更新本文档需要修改的地方以反映真实代码情况!!!**

## 核心命令一览

- `npm install`：安装前后端依赖（Node 18+ / npm 9+）。
- `npm run check`：开发工具链主入口，统一调度 AI Agent 配置检查 → ESLint → Clippy → Prettier → cargo fmt，并输出中文摘要。若缺少 `dist/`，会自动尝试 `npm run build` 供 Tauri Clippy 使用。
- `npm run check:fix`：修复版入口，顺序同上，遇可修复项会自动 `--fix`。
- `npm run tauri dev`：本地启动 Tauri 应用进行端到端手动验证。
- `npm run tauri build`: 本地构建 Tauri 应用安装包。
- `npm run test` / `npm run test:rs`：后端 Rust 单测（当前无前端测试，test 等同 test:rs）。
- `cargo test --locked`：Rust 单测执行器；缺乏覆盖时请补测试后再运行。
- `npm run coverage:rs`：后端覆盖率检查（基于 cargo-llvm-cov，默认行覆盖阈值 90%，需先安装 llvm-tools-preview 与 cargo-llvm-cov，可运行 `npm run coverage:rs:setup` 自动安装依赖）。

## 日常开发流程

0. **fork项目**：在 github 找到本项目的上游仓库: <https://github.com/DuckCoding-dev/DuckCoding> 并 fork 最新的 main 分支（后续开发前需确保sync fork以避免冲突），clone 到本地。
1. **创建分支**：`git switch -c feature/<scope>` 或 `refactor/<scope>`，避免多人在同一文件上叠加提交。
2. **编码前**：阅读/更新对应任务的设计文档，确保拆分策略一致，减少冲突。
3. **开发中**：
   - 大改动请模块化提交，保持 main.rs / App.tsx 等中心文件最小改动范围。
   - 随手运行 `npm run check:fix`，保持 0 ESLint/Clippy 告警。
4. **提交前**：
   - 运行 `npm run check`；失败立即`npm run check:fix`尝试自动修复，若无法自动修复则手动修复，禁止忽略告警。
   - 运行 `cargo test --locked` 与必要的端测脚本。
   - 若有必要，更新 `CLAUDE.md`
5. **提交/PR**：
   - commit/pr 需遵循 Conventional Commits 规范，description使用简体中文。
   - pr 描述需包含：动机、主要改动点、测试情况、风险评估。
   - 避免"修复 CI"类模糊描述，明确指出受影响模块。
   - 如有可关闭的 issue，应在 pr 内提及，以便在 merge 后自动关闭。

## 零警告与质量门禁

- ESLint、Clippy、Prettier、`cargo fmt` 必须全部通过，禁止忽略/跳过检查。
- CI 未通过禁止合并；若需临时跳过必须在 PR 中详细说明原因并获 Reviewer 认可。
- 引入第三方依赖需说明用途、体积和维护计划。

## AI 自动阅读文档前提

- `CLAUDE.md` 默认为 Claude-Code 使用
- Codex 使用需要设置 ~/.codex/config.toml 中的
  ```toml
  project_doc_fallback_filenames = ["CLAUDE.md"]
  ```
- Gemini-CLI 使用需要设置 ~/.gemini/settings.json 中的
  ```json
  {
    "context": {
      "fileName": "CLAUDE.md"
    }
  }
  ```
- `npm run check` 会检查这两项配置（仅当检测到对应工具已安装时），未通过显示警告。可用 `npm run check:fix` 自动修复。

## PR 清单

- [ ] 已运行 `npm run check` 且全部通过。
- [ ] Rust/前端测试已运行（或说明尚未覆盖的原因）。
- [ ] 重要变更附测试或验证截图，方便 Reviewer。

## CI / PR 检查

- `.github/workflows/pr-check.yml` 在 pull_request / workflow_dispatch 下运行，矩阵覆盖 ubuntu-22.04、windows-latest、macos-14 (arm64)、macos-13 (x64)，策略 `fail-fast: false`。
- 每个平台执行 `npm ci` → `npm run check`；若首次检查失败，会继续跑 `npm run check:fix` 与复验 `npm run check` 以判断是否可自动修复，但只要初次检查失败，该平台作业仍标红以阻止合并。
- PR 事件下只保留一条自动评论，双语表格固定展示四个平台；未跑完的平台显示"运行中..."，跑完后实时更新结果、check/fix/recheck 状态、run 链接与日志包名（artifact `pr-check-<platform>.zip`，含 `npm run check` / `check:fix` / `recheck` 输出）。文案提示：如首检失败请本地 `npm run check:fix` → `npm run check` 并提交修复；若 fix 仍失败则需本地排查；跨平台差异无法复现可复制日志给 AI 获取排查建议。
- Linux 装 `libwebkit2gtk-4.1-dev`、`libjavascriptcoregtk-4.1-dev`、`patchelf` 等 Tauri v2 依赖；Windows 确保 WebView2 Runtime（先查注册表，winget 安装失败则回退微软官方静默安装包）；Node 20.19.0，Rust stable（含 clippy / rustfmt），启用 npm 与 cargo 缓存。
- CI 未通过不得合并；缺少 dist 时会在 `npm run check` 内自动触发 `npm run build` 以满足 Clippy 输入。

## 架构记忆（2025-12-12）

- **main.rs 模块化重构（2025-12-15）**：
  - **问题**：原 `src-tauri/src/main.rs` 文件过大（652行），包含启动、托盘、窗口、迁移、命令注册等多种职责
  - **解决方案**：按启动流程分层，拆分为 `setup/` 模块
  - **新架构**（位于 `src-tauri/src/setup/`）：
    - `tray.rs` (195行)：托盘菜单创建、窗口管理（显示/隐藏/聚焦/恢复）、事件处理
    - `initialization.rs` (161行)：启动初始化流程（日志/迁移/Profile/代理自启动/工具注册表）
    - `mod.rs` (9行)：模块导出
  - **main.rs 重构**（402行，从 652 行减少 -38%）：
    - 保留：应用启动、状态管理、builder 配置、macOS 事件循环
    - 辅助函数：工作目录设置、配置监听、更新检查调度、单实例判断
    - 命令注册：保留内联（按功能分组注释），避免宏卫生性问题
  - **架构原则**：遵循单一职责原则（SOLID - SRP），按启动流程分层，main() 函数仅保留核心逻辑
  - **代码质量**：所有检查通过（ESLint + Clippy + Prettier + fmt），单元测试 199 通过
- `src-tauri/src/main.rs` 仅保留应用启动与托盘事件注册，所有 Tauri Commands 拆分到 `src-tauri/src/commands/*`，服务实现位于 `services/*`，核心设施放在 `core/*`（HTTP、日志、错误）。
- **配置管理系统（2025-12-12 重构）**：
  - `services/config/` 模块化拆分为 6 个子模块：
    - `types.rs`：共享类型定义（`CodexSettingsPayload`、`ClaudeSettingsPayload`、`GeminiEnvPayload` 等，60行）
    - `utils.rs`：工具函数（`merge_toml_tables` 保留 TOML 注释，85行）
    - `claude.rs`：Claude Code 配置管理（4个公共函数，实现 `ToolConfigManager` trait，177行）
    - `codex.rs`：Codex 配置管理（支持 config.toml + auth.json，保留 TOML 格式，204行）
    - `gemini.rs`：Gemini CLI 配置管理（支持 settings.json + .env 环境变量，199行）
    - `watcher.rs`：外部变更检测 + 文件监听 + 差异分析（合并原 `config_watcher.rs`，~650行）
      - **差异分析（2026-01-07）**：
        - `compute_diff()`：递归比较 JSON 对象，检测字段变更/新增/删除
        - `matches_pattern()`：支持通配符模式匹配（如 `model_providers.*.base_url`）
        - `is_sensitive_change()`：判断变更是否包含敏感字段（API Key、Base URL 等）
        - `filter_blacklisted()`：过滤黑名单字段（如 theme、ui.\*、model 等自动修改字段）
      - **双模式监听**：
        - **默认模式（Default）**：仅通知敏感字段变更（API Key/URL），忽略其他变更
        - **全量模式（Full）**：通知所有非黑名单字段的变更
      - **变更处理**：
        - `mark_external_change()`：增强版，包含完整差异分析和模式判断
        - `block_external_change`：恢复快照到原生配置文件
        - `allow_external_change`：更新快照为当前文件内容
      - 变更检测：`detect_external_changes`、`acknowledge_external_change`
      - Profile 导入：`import_external_change`
      - 文件监听：`ConfigWatcher`（轮询，跨平台）、`NotifyWatcherManager`（notify，高性能）
      - 核心函数：`config_paths`（返回主配置 + 附属文件）、`compute_native_checksum`（SHA256 校验和）
  - 统一接口：`ToolConfigManager` trait 定义标准的 `read_settings`、`save_settings`、`get_schema`
  - 废弃功能：删除 `ConfigService::save_backup` 系列函数（由 `ProfileManager` 替代）
  - 变更检测：与 `ProfileManager` 集成，自动同步激活状态的 dirty 标记和 checksum
  - 命令层更新：`commands/config_commands.rs` 使用新模块路径（`config::claude::*`、`config::codex::*`、`config::gemini::*`）
  - **Tauri 命令（2026-01-07）**：新增配置监听控制命令
    - `block_external_change`：阻止外部变更，恢复快照到原生配置文件
    - `allow_external_change`：允许外部变更，更新快照为当前文件内容
    - `get_watch_config`：获取监听配置（模式、黑名单、敏感字段）
    - `update_watch_config`：更新监听配置
  - **前端实现（2026-01-07）**：
    - **类型定义**：`types/config-watch.ts` 定义 `WatchMode`、`ConfigWatchConfig`、`ExternalConfigChange` 等类型
    - **命令包装**：`lib/tauri-commands/config-watch.ts` 提供 4 个命令的 TypeScript 封装
    - **配置变更对话框**：`components/dialogs/ConfigChangeDialog.tsx` 显示变更详情，提供阻止/允许操作
      - 展示工具名称、文件路径、变更字段列表（标记新增/删除）
      - 敏感变更带警告图标和 Badge 标记
      - 操作说明区域解释阻止和允许的含义
    - **配置监听 Hook**：`hooks/useConfigWatch.ts` 监听 `config-external-change` 事件，管理对话框状态
    - **全局集成**：`App.tsx` 使用 `useConfigWatch` hook，自动弹出配置变更对话框
    - **设置页面**：`pages/SettingsPage/components/ConfigWatchTab.tsx` 提供监听配置管理界面
      - 双模式选择（默认模式/全量模式）
      - 敏感字段列表展示（按工具分组）
      - 黑名单字段列表展示（按工具分组）
  - 测试状态：15 个测试（11 个通过，包含 9 个差异分析/黑名单/模式匹配测试，4 个旧测试标记为 #[ignore] 待 ProfileManager 重写）
- **工具管理系统**：
  - 多环境架构：支持本地（Local）、WSL、SSH 三种环境的工具实例管理
  - 数据模型：`ToolType`（环境类型）、`ToolInstance`（工具实例）存储在 `models/tool.rs`
  - **JSON 存储（2025-12-04）**：`tools.json` 存储所有工具实例，支持版本控制和多端同步（位于 `~/.duckcoding/tools.json`）
  - 数据结构：按工具分组（`ToolGroup`），每个工具包含 `local_tools`、`wsl_tools`、`ssh_tools` 三个实例列表
  - 数据管理：`services/tool/db::ToolInstanceDB` 操作 JSON 文件，使用 `DataManager` 统一读写
  - 自动迁移：首次启动自动从 SQLite 迁移到 JSON，旧数据库备份为 `tool_instances.db.backup`
  - 安装方式记录：`install_method` 字段记录实际安装方式（npm/brew/official），用于自动选择更新方法
  - WSL 支持：`utils/wsl_executor::WSLExecutor` 提供 Windows 下的 WSL 命令执行和工具检测（10秒超时）
  - Tauri 命令：`get_tool_instances`、`refresh_tool_instances`、`add_wsl_tool_instance`、`add_ssh_tool_instance`、`delete_tool_instance`（位于 `commands/tool_management.rs`）
  - 前端页面：`ToolManagementPage` 按工具（Claude Code/CodeX/Gemini CLI）分组展示，每个工具下列出所有环境实例，使用表格列表样式（`components/ToolListSection`）
  - 功能支持：检测更新（仅 DuckCoding 管理 + 非 SSH）、版本管理（占位 UI）、删除实例（仅 SSH 非内置）
  - 导航集成：AppSidebar 新增"工具管理"入口（Wrench 图标），原"安装工具"已注释
  - 类型安全：完整的 TypeScript 类型定义在 `types/tool-management.ts`，Hook `useToolManagement` 负责状态管理和操作
  - SSH 功能：本期仅保留 UI 和数据结构，实际功能禁用（`AddInstanceDialog` 和表格操作按钮灰显）
  - **Trait-based Detector 架构（2025-12-04）**：
    - `ToolDetector` trait 定义统一的检测、安装、配置管理接口（位于 `services/tool/detector_trait.rs`）
    - 每个工具独立实现：`ClaudeCodeDetector`、`CodeXDetector`、`GeminiCLIDetector`（位于 `services/tool/detectors/`）
    - `DetectorRegistry` 注册表管理所有 Detector 实例，提供 `get(tool_id)` 查询接口
    - `ToolRegistry` 和 `InstallerService` 优先使用 Detector，未注册的工具回退到旧逻辑（向后兼容）
    - 新增工具仅需：1) 实现 ToolDetector trait，2) 注册到 DetectorRegistry，3) 添加 Tool 定义
    - 每个 Detector 文件包含完整的检测、安装、更新、配置管理逻辑，模块化且易测试
  - **命令层模块化重构（2025-12-11）**：
    - 原 `commands/tool_commands.rs` (1001行) 按职责拆分为 6 个模块
    - 模块结构：
      - `tool_commands/installation.rs` - 安装和状态查询（3个命令）
      - `tool_commands/detection.rs` - 工具检测（3个命令）
      - `tool_commands/validation.rs` - 路径和环境验证（2个命令）
      - `tool_commands/update.rs` - 版本更新管理（5个命令）
      - `tool_commands/scanner.rs` - 安装器扫描（1个命令）
      - `tool_commands/management.rs` - 实例管理（1个命令）
      - `tool_commands/mod.rs` - 统一导出
    - 架构原则：严格遵守三层架构（Commands → Services → Utils），命令层仅做参数验证，业务逻辑全部在服务层
    - 服务层增强：
      - `ToolRegistry` 新增 7 个方法：`update_instance`、`check_update_for_instance`、`refresh_all_tool_versions`、`scan_tool_candidates`、`validate_tool_path`、`add_tool_instance`、`detect_single_tool_with_cache`
      - `InstallerService` 新增 1 个方法：`update_instance_by_installer`
      - `utils/version.rs` 新增模块：统一版本号解析逻辑（含 6 个单元测试）
    - 代码质量：命令层从 1001 行减少到 548 行（-45%），平均函数从 62 行减少到 8 行（-87%）
    - 重复代码消除：版本解析、命令执行、数据库访问统一化，消除 ~280 行重复代码
    - 测试覆盖：新增 11 个单元测试（version.rs: 6个，registry.rs: 5个，installer.rs: 3个）
    - 废弃代码清理：删除 `update_tool` 命令（72行），移除 main.rs 中的引用
  - **版本解析统一架构（2025-12-12）**：
    - **单一数据源**：所有版本解析逻辑统一到 `utils/version.rs` 模块
    - **两个公共方法**：
      - `parse_version_string(raw: &str) -> String`：提取版本字符串，支持复杂格式（括号、空格分隔、v 前缀）
      - `parse_version(raw: &str) -> Option<semver::Version>`：解析为强类型 semver 对象，用于版本比较
    - **格式支持**：`2.0.61`、`v1.2.3`、`2.0.61 (Claude Code)`、`codex-cli 0.65.0`、`1.2.3-beta.1`、`rust-v0.55.0` 等
    - **调用者统一**：
      - `VersionService::parse_version()` → 调用 `utils::parse_version()`（删除内部正则逻辑）
      - `Detector::extract_version_default()` → 调用 `utils::parse_version_string()`（删除内部正则逻辑）
      - `registry.rs`、`installer.rs`、`detection.rs` 已使用 `utils::parse_version_string()`（保持不变）
    - **测试覆盖**：7 个测试函数（6 个字符串提取测试 + 1 个 semver 解析测试，7 个断言），覆盖所有格式
    - **代码减少**：删除 `VersionService` 和 `Detector` 中的重复正则定义（约 15 行）
  - **ToolRegistry 模块化拆分（2025-12-13）**：
    - **问题**：原 `services/tool/registry.rs` 文件过大（1118行），包含 21 个方法，职责混杂
    - **解决方案**：按职责拆分为 5 个子模块，每个文件 < 400 行
    - **新架构**（位于 `services/tool/registry/`）：
      - `mod.rs` (57行)：ToolRegistry 结构体定义、初始化方法、ToolDetectionProgress
      - `detection.rs` (323行)：工具检测与持久化（5个方法：`detect_and_persist_local_tools`、`detect_single_tool_by_detector`、`detect_and_persist_single_tool`、`refresh_local_tools`、`detect_single_tool_with_cache`）
      - `instance.rs` (229行)：实例 CRUD 操作（4个方法：`add_wsl_instance`、`add_ssh_instance`、`delete_instance`、`add_tool_instance`）
      - `version_ops.rs` (239行)：版本检查与更新（4个方法：`update_instance`、`check_update_for_instance`、`refresh_all_tool_versions`、`detect_install_methods`）
      - `query.rs` (286行)：查询与辅助工具（6个方法：`get_all_grouped`、`refresh_all`、`get_local_tool_status`、`refresh_and_get_local_status`、`scan_tool_candidates`、`validate_tool_path`）
    - **向后兼容**：保持 `use crate::services::tool::ToolRegistry` 路径不变，调用方无需修改
    - **测试迁移**：4 个单元测试随代码迁移到对应子模块（instance.rs: 1个，query.rs: 3个）
    - **代码质量**：遵循单一职责原则（SOLID - SRP），每个模块职责明确，易于维护和测试
    - **文件大小减少**：最大文件从 1118 行减少到 323 行（-71%），平均文件 227 行
- **透明代理已重构为多工具架构**：
  - `ProxyManager` 统一管理三个工具（Claude Code、Codex、Gemini CLI）的代理实例
  - `HeadersProcessor` trait 定义工具特定的 headers 处理逻辑（位于 `services/proxy/headers/`）
  - `ToolProxyConfig` 存储在 `ProxyConfigManager` 管理的 `~/.duckcoding/proxy.json` 中，每个工具独立配置
  - 支持三个代理同时运行，端口由用户配置（默认: claude-code=8787, codex=8788, gemini-cli=8789）
  - 旧的 `transparent_proxy_*` 字段会在读取配置时自动迁移到新结构
  - **命令层（2025-12-14 清理遗留代码）**：
    - 新架构命令：`start_tool_proxy`、`stop_tool_proxy`、`get_all_proxy_status`、`update_proxy_config`、`get_proxy_config`、`get_all_proxy_configs`
    - 旧命令已完全删除：`start_transparent_proxy`、`stop_transparent_proxy`、`get_transparent_proxy_status`、`update_transparent_proxy_config`
    - 已删除遗留服务：`TransparentProxyConfigService`（原 `services/proxy/transparent_proxy_config.rs`，563行）
    - 已删除前端遗留代码：`useTransparentProxy.ts` hook 和旧 API 包装器
  - **代理工具模块化重构（2025-12-16）**：
    - **问题**：旧架构 `TransparentProxyService` (454行) 完全未使用，`proxy_instance.rs` (421行) 包含大量重复代码
    - **解决方案**：删除旧架构 + 提取通用工具到 `services/proxy/utils/`
    - **已删除文件**：
      - `services/proxy/transparent_proxy.rs` (454行)：单代理实例旧实现（已被 ProxyManager 替代）
    - **新建 utils 模块**（位于 `services/proxy/utils/`，消除重复代码 152 行）：
      - `body.rs` (48行)：统一 `BoxBody` 类型定义和 `box_body()` 工厂函数
      - `loop_detector.rs` (45行)：代理回环检测（`is_proxy_loop` 防止配置指向自身）
      - `error_responses.rs` (63行)：统一 JSON 错误响应模板（配置缺失、回环检测、未授权、内部错误）
      - `mod.rs` (10行)：模块导出和常用类型重导出
    - **proxy_instance.rs 简化**：从 421 行减少到 269 行（-36%），删除重复的类型定义和错误响应构建逻辑
    - **GlobalConfig 清理**：删除 6 个废弃字段（`transparent_proxy_enabled`、`transparent_proxy_port`、`transparent_proxy_api_key`、`transparent_proxy_allow_public`、`transparent_proxy_real_api_key`、`transparent_proxy_real_base_url`）
    - **迁移逻辑**：`migrations/proxy_config.rs` 使用 `serde_json::Value` 手动操作 JSON，保持向后兼容
    - **代码质量**：遵循 DRY 原则，所有检查通过（Clippy + fmt + ESLint + Prettier），测试 199 通过
  - **配置管理机制（2025-12-12）**：
    - 代理启动时自动创建内置 Profile（`dc_proxy_*`），通过 `ProfileManager` 切换配置
    - 内置 Profile 在 UI 中不可见（列表查询时过滤 `dc_proxy_` 前缀）
    - `dc_proxy_` 为系统保留前缀，用户无法创建同名 Profile
    - 代理关闭时自动还原到启动前激活的 Profile
    - 运行时禁止修改 `ToolProxyConfig`，确保配置一致性
    - `original_active_profile` 字段记录启动前的 Profile 用于还原
    - Gemini CLI 的 model 字段为可选，允许不填（内置代理 Profile 不设置 model，保留用户原有配置）
- `ToolProxyConfig` 额外存储 `real_profile_name`、`auto_start`、工具级 `session_endpoint_config_enabled`，全局配置新增 `hide_transparent_proxy_tip` 控制设置页横幅显示
- `GlobalConfig.hide_session_config_hint` 持久化会话级端点提示的隐藏状态，`ProxyControlBar`/`ProxySettingsDialog`/`ClaudeContent` 通过 `open-proxy-settings` 与 `proxy-config-updated` 事件联动刷新视图
- 日志系统支持完整配置管理：`GlobalConfig.log_config` 存储级别/格式/输出目标；`log_commands.rs` 提供查询与更新命令，`LogSettingsTab` 可热重载级别、保存文件输出设置；`core/logger.rs` 通过 `update_log_level` reload 机制动态调整
- **配置监听系统（2026-01-07）**：`GlobalConfig.config_watch: ConfigWatchConfig` 管理文件监听行为
  - `mode: WatchMode`：监听模式（Default=仅敏感字段，Full=全量变更）
  - `blacklist: HashMap<String, Vec<String>>`：按工具分组的黑名单字段（如 `theme.*`、`ui.*`、`model`），自动过滤不需通知的变更
  - `sensitive_fields: HashMap<String, Vec<String>>`：按工具分组的敏感字段（API Key、Base URL 等），用于默认模式判断
  - 配合 `ActiveProfile.native_snapshot` 实现 Block/Allow 操作
- 应用启动时 `duckcoding::auto_start_proxies` 会读取配置，满足 `enabled && auto_start` 且存在 `local_api_key` 的代理会自动启动
- `utils::config::migrate_session_config` 会将旧版 `GlobalConfig.session_endpoint_config_enabled` 自动迁移到各工具配置，确保升级过程不会丢开关
- 全局配置读写统一走 `utils::config::{read_global_config, write_global_config, apply_proxy_if_configured}`，避免出现多份路径逻辑；任何命令要修改配置都应调用这些辅助函数。
- UpdateService / 统计命令等都通过 `tauri::State` 注入复用，前端 ToolStatus 的结构保持轻量字段 `{id,name,installed,version}`。
- **工具状态管理已统一到数据库架构（2025-12-04）**：
  - `check_installations` 命令改为从 `ToolRegistry` 获取数据，优先读取数据库（< 10ms），首次启动自动检测并持久化（~1.3s）
  - `refresh_tool_status` 命令重新检测所有本地工具并更新数据库（upsert + 删除已卸载）
  - Dashboard 和 ToolManagement 现使用统一数据源，消除了双数据流问题
  - `ToolStatusCache` 标记为已废弃，保留仅用于向后兼容
  - 所有工具状态查询统一走 `ToolRegistry::get_local_tool_status()` 和 `refresh_and_get_local_status()`
- UI 相关的托盘/窗口操作集中在 `src-tauri/src/ui/*`，其它模块如需最小化到托盘请调用 `ui::hide_window_to_tray` 等封装方法。
- **会话管理系统（2025-12-12 重构）**：
  - **架构迁移**：`SessionDatabase` 已删除，`SessionManager` 直接使用 `DataManager::sqlite()` 管理会话数据
  - **核心模块**（位于 `services/session/`）：
    - `manager.rs`：`SESSION_MANAGER` 单例，内部持有 `Arc<DataManager>` 和 `db_path`
    - `db_utils.rs`：私有工具模块，提供 `QueryRow ↔ ProxySession` 转换、SQL 常量定义
    - `models.rs`：数据模型（`ProxySession`、`SessionEvent`、`SessionListResponse`）
  - **查询缓存**：所有数据库查询自动利用 `SqliteManager` 的查询缓存（容量 100，TTL 5分钟）
  - **转换工具**：
    - `parse_proxy_session(row)` - 将 QueryRow 转换为 ProxySession（处理 13 个字段 + NULL 值）
    - `parse_count(row)` - 提取计数查询结果
    - `parse_session_config(row)` - 提取三元组配置 (config_name, url, api_key)
  - **后台任务**：
    - 批量写入任务：每 100ms 或缓冲区满 10 条时批量 upsert 会话
    - 定期清理任务：每小时清理 30 天未活跃会话和超过 1000 条的旧会话
  - **测试覆盖**：10 个单元测试（5 个 db_utils 转换测试 + 2 个 models 测试 + 3 个 manager 集成测试）
  - **代码减少**：从 366 行（db.rs）减少到 ~320 行（manager.rs 250 行 + db_utils.rs 70 行工具函数）
- **Token 统计与成本分析系统（2026-01-14）**：
  - **后端模块**：`services/token_stats/*`（日志记录、统计分析、趋势聚合）、`services/pricing/*`（价格模板与成本计算）、`services/proxy/log_recorder/*`（请求日志与 Token 提取），数据库表 `token_logs` 支持保留天数与条数上限清理
  - **命令层**：新增 `analytics_commands.rs`、`token_stats_commands.rs`、`pricing_commands.rs`；支持趋势/成本汇总、日志分页查询、会话级统计
  - **配置扩展**：`ToolProxyConfig` 新增 `pricing_template_id`，Profile/会话配置支持覆盖价格模板
  - **前端页面**：`TokenStatisticsPage`（Dashboard + TrendsChart）、`SettingsPage` 新增 `TokenStatsTab` 与 `PricingTab`；统一分析 Hook `useAnalyticsStats` + 时间范围控制 `useTimeRangeControl`
- **透明代理页面重构（2026-01-14）**：
  - `TransparentProxyPage` 采用 Tab 驱动架构（`types/tab-types.ts`），支持主视图/会话详情双模式，URL 查询参数与 ViewState 双向同步
  - 主视图 Tabs：`SessionListTab` / `GlobalStatsTab` / `GlobalLogsTab`；会话详情 Tabs：`SessionStatsTab` / `SessionLogsTab` / `SessionSettingsTab`
  - 共享组件：`LogsTable`、`RealtimeStats`、`CustomTimeRangeDialog`；会话详情页隐藏代理控制栏，简化布局
- **余额监控页面（BalancePage）**：
  - 后端提供通用 `fetch_api` 命令（位于 `commands/api_commands.rs`），支持 GET/POST、自定义 headers、超时控制
  - 前端使用 JavaScript `Function` 构造器执行用户自定义的 extractor 脚本（位于 `utils/extractor.ts`）
  - 配置存储在 localStorage，API Key 仅保存在内存（`useApiKeys` hook）
  - 支持预设模板（NewAPI、OpenAI、自定义），模板定义在 `templates/index.ts`
  - `useBalanceMonitor` hook 负责自动刷新逻辑，支持配置级别的刷新间隔
  - 配置表单（`ConfigFormDialog`）支持模板选择、代码编辑、静态 headers（JSON 格式）
  - 卡片视图（`ConfigCard`）展示余额信息、使用比例、到期时间、错误提示
- **Profile 管理系统 v2.0（2025-12-06）**：
  - **双文件 JSON 架构**：替代旧版分散式目录结构（profiles/、active/、metadata/）
    - `~/.duckcoding/profiles.json`：统一存储所有工具的 Profile 数据仓库
    - `~/.duckcoding/active.json`：工具激活状态管理
  - **数据结构**：
    - `ProfilesStore`：按工具分组（`claude_code`、`codex`、`gemini_cli`），每个工具包含 `HashMap<String, ProfileData>`
    - `ActiveStore`：每个工具一个 `Option<ActiveProfile>`，记录当前激活的 Profile 名称和切换时间
      - `native_snapshot`（2026-01-07）：保存激活 Profile 应用后的完整原生配置快照（JSON），用于阻止外部变更时恢复
      - `last_synced_at`（2026-01-07）：最后同步时间戳，配合 dirty 标记判断配置状态
    - `ProfilePayload`：Enum 类型，支持 Claude/Codex/Gemini 三种变体，存储工具特定配置和原生文件快照
  - **核心服务**（位于 `services/profile_manager/`）：
    - `ProfileManager`：统一的 Profile CRUD 接口，支持列表、创建、更新、删除、激活、导入导出
    - `NativeConfigSync`：原生配置文件参数同步
      - **激活操作**：仅替换工具原生配置文件中的 API Key 和 Base URL 两个参数，保留其他配置（如主题、快捷键等）
      - **支持格式**：Claude（settings.json）、Codex（auth.json + config.toml）、Gemini（.env）
      - **完整快照**：Profile 存储时保存完整原生文件快照（settings.json + config.json、config.toml + auth.json、settings.json + .env），用于导入导出和配置回滚
  - **迁移系统**（ProfileV2Migration）：
    - 支持从**两套旧系统**迁移到新架构：
      1. **原始工具配置**：`~/.claude/settings.{profile}.json`、`~/.codex/config.{profile}.toml + auth.{profile}.json`、`~/.gemini-cli/.env.{profile}`
      2. **旧 profiles/ 目录系统**：`~/.duckcoding/profiles/{tool}/{profile}.{ext}` + `active/{tool}.json` + `metadata/index.json`
    - 迁移逻辑：先从原始配置迁移创建 Profile，再从 profiles/ 目录补充（跳过重复），最后合并激活状态
    - 清理机制：迁移完成后自动备份到 `backup_profile_v1_{timestamp}/` 并删除旧目录（profiles/、active/、metadata/）
    - 手动清理：提供 `clean_legacy_backups` 命令删除备份的原始配置文件（settings.{profile}.json 等）
  - **前端页面**（ProfileManagementPage）：
    - Tab 分组布局：按工具（Claude Code、Codex、Gemini CLI）水平分页
    - `ActiveProfileCard`：显示当前激活配置，支持工具实例选择器（Local/WSL/SSH）、版本信息、更新检测
    - Profile 列表：支持创建、编辑、删除、激活、导入导出操作
  - **Tauri 命令**（位于 `commands/profile_commands.rs`）：
    - Profile CRUD：`list_profiles`、`create_profile`、`update_profile`、`delete_profile`、`activate_profile`
    - 导入导出：`import_profile`、`export_profile`、`import_from_native`
    - 原生同步：`sync_to_native`、`sync_from_native`
  - **类型定义**（`types/profile.ts`）：
    - `ProfileGroup`：工具分组，包含工具信息和 Profile 列表
    - `ProfileDescriptor`：Profile 元数据（名称、格式、创建/更新时间、来源）
    - `ProfilePayload`：联合类型，支持 Claude/Codex/Gemini 配置
- **新用户引导系统**：
  - 首次启动强制引导，配置存储在 `GlobalConfig.onboarding_status: Option<OnboardingStatus>`（包含已完成版本、跳过步骤、完成时间）
  - 版本化管理，支持增量更新（v1 -> v2 只展示新增内容），独立的引导内容版本号（与应用版本解耦）
  - 前端定义引导步骤（`components/Onboarding/config/versions.ts`：`CURRENT_ONBOARDING_VERSION`、`VERSION_STEPS`、`getRequiredSteps`）
  - Rust 命令：`get_onboarding_status`、`save_onboarding_progress`、`complete_onboarding`、`reset_onboarding`（位于 `commands/onboarding.rs`）
  - 设置页「关于」标签可重新打开引导（调用 `reset_onboarding` 后刷新页面）
  - v1 引导包含 4 步：欢迎页、代理配置（可跳过）、工具介绍、完成页；v2/v3 引导聚焦新增特性
  - 引导组件：`OnboardingOverlay`（全屏遮罩）、`OnboardingFlow`（流程控制）、步骤组件（`steps/v*/*`）
  - App.tsx 启动时检查 `onboarding_status`，根据版本对比决定是否显示引导
- **统一数据管理系统（DataManager）**：
  - 模块位置：`src-tauri/src/data/*`，提供 JSON/TOML/ENV 格式的统一管理接口
  - 核心组件：`DataManager` 统一入口，`JsonManager`/`TomlManager`/`EnvManager` 格式管理器，LRU 缓存层（基于文件校验和）
  - 使用模式：
    - `manager.json()` - 带缓存的 JSON 操作，用于全局配置和 Profile
    - `manager.json_uncached()` - 无缓存的 JSON 操作，用于工具原生配置（需实时更新）
    - `manager.toml()` - TOML 操作，支持保留注释和格式（使用 `read_document()` / `write()` 配合 `toml_edit::DocumentMut`）
    - `manager.env()` - .env 文件操作，自动排序和格式化
  - 自动化特性：目录创建、Unix 权限设置（0o600）、原子写入、基于 mtime 的缓存失效
  - 已迁移模块：
    - `utils/config.rs`: 全局配置读写（`read_global_config`、`write_global_config`）
    - `services/config.rs`: 工具配置管理（Claude/Codex/Gemini 的 read/save/apply 系列函数）
    - `services/profile_store.rs`: Profile 存储管理（`save_profile_payload`、`load_profile_payload`、`read_active_state`、`save_active_state`）
  - 测试覆盖：16 个迁移测试（`data::migration_tests`）+ 各模块原有测试全部通过
  - API 原则：所有新代码的文件 I/O 操作必须使用 DataManager，禁止直接使用 `fs::read_to_string`/`fs::write`

### 透明代理扩展指南

添加新工具支持需要：

1. 在 `services/proxy/headers/` 实现 `HeadersProcessor` trait
2. 在 `services/proxy/headers/mod.rs` 的 `create_headers_processor` 工厂函数中注册
3. 在 `models/tool.rs` 添加工具定义（如已存在则跳过）
4. 在 `models/config.rs` 的 `default_proxy_configs` 函数中添加默认端口配置
5. 无需修改 `ProxyManager` 和命令层代码（自动支持）
