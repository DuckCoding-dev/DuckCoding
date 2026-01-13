/**
 * Tab 相关类型定义
 * 用于管理透明代理页面的 Tab 切换和路由状态
 */

// ==================== 主页面 Tab ====================

/**
 * 主页面 Tab ID
 * - session-list: 会话列表
 * - global-stats: 全局统计
 * - global-logs: 全局日志
 */
export type MainTabId = 'session-list' | 'global-stats' | 'global-logs';

/**
 * 主页面 Tab 元数据
 */
export interface MainTabMetadata {
  id: MainTabId;
  label: string;
  icon: string; // Emoji 图标
}

/**
 * 主页面 Tab 列表
 */
export const MAIN_TABS: MainTabMetadata[] = [
  { id: 'session-list', label: '会话列表', icon: '📋' },
  { id: 'global-stats', label: '全局统计', icon: '📊' },
  { id: 'global-logs', label: '全局日志', icon: '📜' },
];

// ==================== 会话详情页 Tab ====================

/**
 * 会话详情页 Tab ID
 * - session-stats: 会话统计
 * - session-logs: 会话日志
 * - session-settings: 会话设置
 */
export type SessionDetailTabId = 'session-stats' | 'session-logs' | 'session-settings';

/**
 * 会话详情页 Tab 元数据
 */
export interface SessionDetailTabMetadata {
  id: SessionDetailTabId;
  label: string;
  icon: string; // Emoji 图标
}

/**
 * 会话详情页 Tab 列表
 */
export const SESSION_DETAIL_TABS: SessionDetailTabMetadata[] = [
  { id: 'session-stats', label: '会话统计', icon: '📊' },
  { id: 'session-logs', label: '会话日志', icon: '📜' },
  { id: 'session-settings', label: '会话设置', icon: '⚙️' },
];

// ==================== 路由状态 ====================

/**
 * 视图模式
 * - main: 主页面（显示会话列表/全局统计/全局日志）
 * - session-detail: 会话详情页（显示会话统计/会话日志/会话设置）
 */
export type ViewMode = 'main' | 'session-detail';

/**
 * 视图状态
 * 用于管理页面切换和 Tab 状态
 */
export interface ViewState {
  /** 当前视图模式 */
  mode: ViewMode;
  /** 主页面当前激活的 Tab */
  mainTab: MainTabId;
  /** 会话详情页当前激活的 Tab */
  sessionDetailTab: SessionDetailTabId;
  /** 当前查看的会话 ID（仅在 session-detail 模式下有值）*/
  selectedSessionId: string | null;
}

/**
 * 默认视图状态
 */
export const DEFAULT_VIEW_STATE: ViewState = {
  mode: 'main',
  mainTab: 'session-list', // 默认显示会话列表
  sessionDetailTab: 'session-stats',
  selectedSessionId: null,
};

// ==================== 辅助类型 ====================

/**
 * Tab 切换回调函数类型
 */
export type TabChangeHandler<T extends string> = (tabId: T) => void;

/**
 * 导航回调函数类型
 */
export interface NavigationHandlers {
  /** 导航到会话详情页 */
  navigateToSessionDetail: (sessionId: string) => void;
  /** 返回主页面 */
  navigateToMain: () => void;
}
