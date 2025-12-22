# CI Workflows 说明

本项目使用两步 workflow 实现跨仓库 PR 的评论功能。

> 最终测试：验证从 artifact 提取 PR 号的评论功能。

## Workflows

### 1. PR Check (`pr-check.yml`)

**触发条件**：

- `pull_request` 事件
- `workflow_dispatch` 手动触发

**功能**：

- 在 4 个平台上运行代码检查（ubuntu-22.04, windows-latest, macos-14, macos-15）
- 执行 `npm run check` → `npm run check:fix` → `npm run check`（复验）
- 将每个平台的状态保存到 artifact (`pr-check-state-*`)
- 上传日志文件到 artifact (`pr-check-*`)

**输出 artifacts**：

- `pr-check-<platform>`: 包含 check.log, check-fix.log, check-recheck.log
- `pr-check-state-<platform>`: 包含平台状态的 JSON 文件

### 2. PR Check Comment (`pr-check-comment.yml`)

**触发条件**：

- `workflow_run` 事件（当 PR Check 完成时）
- 仅在默认分支（main）上的 workflow 文件会被触发

**功能**：

- 下载所有平台的状态 artifact
- 聚合所有平台的检查结果
- 创建/更新 PR 评论，显示所有平台的状态

**权限**：

- 使用主仓库的 GITHUB_TOKEN（有完整的 `pull-requests: write` 权限）
- 支持跨仓库 PR（fork）的评论

## 为什么需要两步 workflow？

### 问题

在 fork 仓库发起的 PR 中，`GITHUB_TOKEN` 只有 `read` 权限，无法创建/更新评论。这是 GitHub 的安全限制。

### 解决方案

使用 `workflow_run` 事件：

1. PR 触发的 workflow 在 fork 的上下文中运行（权限受限）
2. `workflow_run` 触发的 workflow 在主仓库的上下文中运行（权限完整）
3. 通过 artifact 传递数据，实现权限隔离

### 架构图

```
PR 提交
  ↓
PR Check (fork 上下文，read-only)
  ├─ 运行检查
  ├─ 保存状态到 artifact
  └─ 上传日志
  ↓
PR Check 完成
  ↓
PR Check Comment (main 上下文，write 权限) ← workflow_run 触发
  ├─ 下载 artifacts
  ├─ 聚合状态
  └─ 发布/更新评论 ✅
```

## 重要限制

⚠️ **workflow_run 要求**：

- 被触发的 workflow 文件必须存在于**默认分支**（main）
- 修改 `pr-check-comment.yml` 后，必须先合并到 main 才能生效
- Fork PR 无法测试评论功能，只能在合并到 main 后验证

## 开发建议

### 修改 PR Check workflow

1. 修改 `.github/workflows/pr-check.yml`
2. 提交到功能分支并创建 PR
3. PR 中可以直接测试检查逻辑
4. 合并到 main

### 修改 PR Check Comment workflow

1. 修改 `.github/workflows/pr-check-comment.yml`
2. 提交并合并到 main（评论功能无法在 PR 中测试）
3. 合并后，下一个 PR 会触发新版本的评论 workflow

### 调试评论功能

```bash
# 查看评论 workflow 运行记录
gh run list --workflow="PR Check Comment" --limit 5

# 查看特定运行的日志
gh run view <run-id> --log

# 查看 PR Check 的 artifacts
gh api repos/OWNER/REPO/actions/runs/<run-id>/artifacts
```

## 参考资料

- [GitHub Actions: workflow_run event](https://docs.github.com/en/actions/using-workflows/events-that-trigger-workflows#workflow_run)
- [GitHub Actions: Permissions for GITHUB_TOKEN](https://docs.github.com/en/actions/security-guides/automatic-token-authentication#permissions-for-the-github_token)
- [Using artifacts to share data between jobs](https://docs.github.com/en/actions/using-workflows/storing-workflow-data-as-artifacts)
