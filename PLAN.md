# macOS 标题栏 Profile 快捷切换功能实现计划

## 需求概述

在 macOS 下点击顶部标题栏（应用菜单栏），显示快捷操作菜单：

- 显示 Claude Code、Codex、Gemini CLI 三个工具的配置方案（Profile）
- 显示每个工具当前激活的配置
- 支持从菜单栏直接切换激活的配置

## 技术方案

### 方案选择：Tauri v2 原生应用菜单栏

使用 Tauri v2 的 `Menu` API 创建 macOS 原生应用菜单栏（非托盘菜单），通过 `SubmenuBuilder` 构建层级菜单结构。

**优势**：

- 原生 macOS 体验，符合用户习惯
- 支持动态更新菜单项
- 支持复选标记（CheckMenuItem）显示当前激活状态
- 与现有托盘菜单系统架构一致

### 菜单结构设计

```
DuckCoding (应用菜单)
├── 关于 DuckCoding
├── ─────────────
├── 配置方案                    ← 新增子菜单
│   ├── Claude Code
│   │   ├── ✓ profile-1        ← 当前激活（带复选标记）
│   │   ├──   profile-2
│   │   └──   profile-3
│   ├── ─────────────
│   ├── Codex
│   │   ├──   codex-profile-1
│   │   └── ✓ codex-profile-2
│   ├── ─────────────
│   └── Gemini CLI
│       └──   gemini-profile-1
├── ─────────────
├── 设置...                     ← 可选：快捷跳转
├── ─────────────
└── 退出
```

## 实现步骤

### Phase 1: 后端菜单模块

#### 1.1 创建菜单模块 `src-tauri/src/setup/menu.rs`

**职责**：

- 创建应用菜单栏
- 构建 Profile 子菜单
- 处理菜单事件

**核心函数**：

```rust
// 创建应用菜单栏
pub fn create_app_menu<R: Runtime>(app: &AppHandle<R>) -> tauri::Result<Menu<R>>

// 构建 Profile 子菜单
fn build_profile_submenu<R: Runtime>(
    app: &AppHandle<R>,
    profile_manager: &ProfileManager,
) -> tauri::Result<Submenu<R>>

// 设置菜单事件处理
pub fn setup_menu_handler<R: Runtime>(app: &tauri::App<R>) -> tauri::Result<()>
```

#### 1.2 菜单项 ID 设计

使用结构化 ID 便于事件处理：

- `profile:claude-code:profile-name` - Profile 切换
- `menu:settings` - 打开设置
- `menu:about` - 关于

#### 1.3 动态菜单更新

**触发时机**：

- 应用启动时
- Profile 创建/删除/激活后
- 收到 `profile-changed` 事件时

**实现方式**：

- 使用 `app.set_menu()` 重建整个菜单
- 或使用 `CheckMenuItem::set_checked()` 仅更新复选状态

### Phase 2: 集成到应用启动流程

#### 2.1 修改 `setup/mod.rs`

导出新的 menu 模块：

```rust
pub mod menu;
```

#### 2.2 修改 `main.rs`

在 `setup_app_hooks` 中添加菜单初始化：

```rust
// 6. 创建应用菜单栏（仅 macOS）
#[cfg(target_os = "macos")]
setup::menu::setup_app_menu(app)?;
```

### Phase 3: 菜单事件处理

#### 3.1 Profile 切换逻辑

```rust
fn handle_profile_activation(
    app: &AppHandle,
    tool_id: &str,
    profile_name: &str,
) -> Result<()> {
    // 1. 调用 ProfileManager.activate_profile()
    // 2. 更新菜单复选状态
    // 3. 发送事件通知前端
}
```

#### 3.2 前端事件监听

监听 `profile-activated-from-menu` 事件，刷新 UI 状态。

### Phase 4: 菜单动态刷新

#### 4.1 新增 Tauri 命令

```rust
#[tauri::command]
pub fn refresh_app_menu(app: AppHandle) -> Result<(), String>
```

#### 4.2 前端触发刷新

在 Profile 操作后调用：

- `pmSaveProfile` 后
- `pmDeleteProfile` 后
- `pmActivateProfile` 后

## 文件变更清单

### 新增文件

| 文件路径                      | 说明           |
| ----------------------------- | -------------- |
| `src-tauri/src/setup/menu.rs` | 应用菜单栏模块 |

### 修改文件

| 文件路径                            | 变更内容                     |
| ----------------------------------- | ---------------------------- |
| `src-tauri/src/setup/mod.rs`        | 导出 menu 模块               |
| `src-tauri/src/main.rs`             | 调用菜单初始化、注册刷新命令 |
| `src/lib/tauri-commands/profile.ts` | 添加 refreshAppMenu 命令包装 |
| `src/hooks/useProfileManagement.ts` | 操作后触发菜单刷新           |

## 技术细节

### macOS 条件编译

所有菜单相关代码使用 `#[cfg(target_os = "macos")]` 条件编译，确保：

- Windows/Linux 不受影响
- 编译产物不包含无用代码

### 菜单项数量限制

考虑 Profile 数量可能较多的情况：

- 每个工具最多显示 10 个 Profile
- 超出部分显示 "更多..." 跳转到 Profile 管理页面

### 错误处理

- Profile 加载失败时显示 "加载失败" 占位项
- 激活失败时显示系统通知

## 测试计划

1. **单元测试**：菜单 ID 解析函数
2. **集成测试**：Profile 激活流程
3. **手动测试**：
   - macOS 菜单栏显示正确
   - 复选标记正确反映激活状态
   - 切换后前端 UI 同步更新
   - Windows/Linux 编译不受影响

## 风险评估

| 风险             | 影响 | 缓解措施                       |
| ---------------- | ---- | ------------------------------ |
| 菜单刷新性能     | 中   | 使用增量更新而非全量重建       |
| Profile 名称过长 | 低   | 截断显示，tooltip 显示完整名称 |
| 跨平台兼容性     | 低   | 条件编译隔离 macOS 代码        |

## 后续扩展

- 添加键盘快捷键（如 ⌘1/⌘2/⌘3 切换工具）
- 支持 Touch Bar 快捷操作
- 添加最近使用的 Profile 快捷列表
