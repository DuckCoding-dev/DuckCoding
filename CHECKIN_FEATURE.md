# 签到功能实现说明

## 功能概述

为 DuckCoding 的供应商管理添加了完整的签到功能,支持手动签到和自动定时签到。

## 实现内容

### 1. 类型扩展

**前端** (`src/types/provider.ts`):
- 添加 `CheckinConfig` 接口
- 添加 `CheckinResponse` 接口
- `Provider` 接口新增 `checkin_config` 可选字段
- 支持 `checkin_hour` 配置签到时间

**后端** (`src-tauri/src/models/provider.rs`):
- Rust `CheckinConfig` 结构体
- 默认签到时间 9:00
- 完整的签到状态追踪

### 2. 签到服务

**前端** (`src/services/checkin.ts`):
- `isCheckinSupported()` - 检查供应商是否支持签到
- `performCheckin()` - 执行签到
- `getCheckinStatus()` - 获取签到状态
- 支持自定义 API 端点
- 自动处理认证 (Bearer Token + New-Api-User)
- 完善的错误处理

**后端** (`src-tauri/src/services/checkin.rs`):
- `perform_checkin()` - 异步签到请求
- `should_checkin()` - 判断是否需要签到
- 防止重复签到 (同一天只签到一次)
- 时间检查 (到达设定时间才签到)

### 3. 定时调度器 (`src-tauri/src/services/checkin_scheduler.rs`)
- `CheckinScheduler` - 签到调度器
- 每小时检查一次所有供应商
- 自动执行到期的签到任务
- 自动更新签到状态和统计
- 后台运行,不影响主线程

### 4. 签到对话框 (`CheckinDialog.tsx`)
- 显示签到状态和历史
- 自动签到开关
- 签到时间选择器 (0-23 小时)
- 立即签到按钮
- 统计信息展示:
  - 今日签到状态
  - 本月签到次数
  - 累计签到次数
  - 累计获得额度
- 最近签到记录列表
- 不支持签到的友好提示

### 5. UI 集成
- `ProviderCard` 添加签到按钮
- 只在支持签到的供应商上显示按钮
- 已签到状态显示为 "已签"
- 未签到显示为 "签到"
- 主页面集成签到对话框

### 6. 应用启动集成 (`src-tauri/src/main.rs`)
- 应用启动时自动初始化调度器
- 后台运行,持续监控
- 全局状态管理

## 兼容性处理

### 自动检测
系统会自动检测供应商是否支持签到:
- 检查必要字段 (api_address/website_url, user_id, access_token)
- 尝试调用签到 API
- 404 错误识别为不支持

### 不支持签到的供应商
- 不显示签到按钮
- 打开对话框时显示友好提示
- 不影响其他功能使用

### 错误处理
- 网络错误: "网络连接失败，请检查供应商地址是否正确"
- 404 错误: "该供应商不支持签到功能"
- 认证错误: 显示具体错误信息
- 格式错误: "供应商返回的数据格式不正确"

## 使用方式

### 添加供应商时配置
供应商需要包含:
- `api_address` 或 `website_url` - API 基础地址
- `user_id` - 用户 ID
- `access_token` - 访问令牌

### 手动签到
1. 在供应商卡片点击 "签到" 按钮 (仅支持签到的供应商显示)
2. 查看签到状态和历史
3. 点击 "立即签到" 执行签到

### 自动签到
1. 打开签到对话框
2. 开启 "自动签到" 开关
3. 选择签到时间 (默认 9:00)
4. 系统将在每天指定时间自动签到

### 签到时间说明
- 可选择 0-23 小时
- 默认 9:00
- 调度器每小时检查一次
- 到达设定时间后自动执行
- 同一天只签到一次

## API 兼容性

当前实现兼容 New API 的签到接口:
- `GET /api/user/checkin` - 获取状态
- `POST /api/user/checkin` - 执行签到

需要的 Headers:
- `Authorization: Bearer <token>`
- `New-Api-User: <user_id>`

## 测试

已在 duckcoding.com 测试通过:
- ✅ 获取签到状态
- ✅ 执行签到
- ✅ 显示签到历史
- ✅ 统计信息展示
- ✅ 不支持签到的提示
- ✅ 错误处理
- ✅ 时间选择器
- ✅ 自动签到配置

## 技术架构

### 前端
- React + TypeScript
- Tauri Commands
- shadcn/ui 组件库

### 后端
- Rust + Tokio (异步运行时)
- reqwest (HTTP 客户端)
- chrono (时间处理)
- 定时任务调度

### 数据流
1. 用户配置签到 → 保存到 Provider
2. 调度器每小时检查 → 判断是否需要签到
3. 执行签到 → 更新状态和统计
4. 前端实时显示 → 用户查看结果

## 提交记录

- `ba680b4` - feat: 添加供应商签到功能
- `4705b1a` - docs: 添加签到功能说明文档
- `f2473c3` - fix: 添加签到功能兼容性检查和错误处理
- `04a5491` - docs: 更新签到功能文档
- `e6c2923` - feat: 添加自动签到定时任务功能
- `cc89ffc` - feat: 在应用启动时初始化签到调度器

## 未来优化

1. 签到失败重试机制
2. 签到成功通知
3. 更多供应商的签到接口适配
4. 签到历史详细记录
5. 签到统计图表
