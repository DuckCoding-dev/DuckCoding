/**
 * 仪表盘组件
 * 展示关键指标的卡片视图
 */
import React from 'react';
import { DollarSign, Activity, Clock, AlertCircle } from 'lucide-react';
import type { CostSummary } from '@/types/analytics';

/**
 * 单个指标卡片组件属性
 */
interface MetricCardProps {
  /** 指标标题 */
  title: string;
  /** 指标值 */
  value: string | number;
  /** 副标题或额外信息 */
  subtitle?: string;
  /** 图标组件 */
  icon: React.ReactNode;
  /** 图标背景颜色类名 */
  iconBgColor: string;
  /** 图标颜色类名 */
  iconColor: string;
  /** 趋势指示（可选） */
  trend?: {
    /** 趋势值 */
    value: number;
    /** 是否为正向趋势 */
    isPositive: boolean;
  };
}

/**
 * 单个指标卡片组件
 */
const MetricCard: React.FC<MetricCardProps> = ({
  title,
  value,
  subtitle,
  icon,
  iconBgColor,
  iconColor,
  trend,
}) => {
  return (
    <div className="rounded-lg border bg-white p-6 shadow-sm transition-shadow hover:shadow-md dark:bg-gray-800 dark:border-gray-700">
      <div className="flex items-start justify-between">
        <div className="flex-1">
          <p className="text-sm font-medium text-gray-600 dark:text-gray-400">{title}</p>
          <p className="mt-2 text-3xl font-semibold text-gray-900 dark:text-gray-100">{value}</p>
          {subtitle && <p className="mt-1 text-sm text-gray-500 dark:text-gray-400">{subtitle}</p>}
          {trend && (
            <div className="mt-2 flex items-center gap-1">
              <span
                className={`text-sm font-medium ${
                  trend.isPositive
                    ? 'text-green-600 dark:text-green-400'
                    : 'text-red-600 dark:text-red-400'
                }`}
              >
                {trend.isPositive ? '↑' : '↓'} {Math.abs(trend.value)}%
              </span>
              <span className="text-xs text-gray-500 dark:text-gray-400">vs 上期</span>
            </div>
          )}
        </div>
        <div className={`rounded-lg p-3 ${iconBgColor}`}>
          <div className={iconColor}>{icon}</div>
        </div>
      </div>
    </div>
  );
};

/**
 * 格式化总成本（向上舍入到2位小数）
 */
function formatTotalCost(cost: number): string {
  const rounded = Math.ceil(cost * 100) / 100;
  return `$${rounded.toFixed(2)}`;
}

/**
 * 格式化响应时间（毫秒）
 */
function formatResponseTime(ms: number | null): string {
  if (ms === null) {
    return 'N/A';
  }
  if (ms < 1000) {
    return `${Math.round(ms)}ms`;
  }
  return `${(ms / 1000).toFixed(2)}s`;
}

/**
 * 格式化请求数量
 */
function formatRequestCount(count: number): string {
  if (count >= 1000000) {
    return `${(count / 1000000).toFixed(1)}M`;
  }
  if (count >= 1000) {
    return `${(count / 1000).toFixed(1)}K`;
  }
  return count.toString();
}

/**
 * 仪表盘组件属性
 */
export interface DashboardProps {
  /** 成本汇总数据 */
  summary: CostSummary;
  /** 是否显示加载状态 */
  loading?: boolean;
}

/**
 * 仪表盘组件
 */
export const Dashboard: React.FC<DashboardProps> = ({ summary, loading = false }) => {
  if (loading) {
    return (
      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {[1, 2, 3, 4].map((i) => (
          <div key={i} className="h-32 animate-pulse rounded-lg bg-gray-200 dark:bg-gray-700" />
        ))}
      </div>
    );
  }

  const errorRate =
    summary.total_requests > 0 ? (summary.failed_requests / summary.total_requests) * 100 : 0;

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
      {/* 总成本 */}
      <MetricCard
        title="总成本"
        value={formatTotalCost(summary.total_cost)}
        subtitle={`共 ${formatRequestCount(summary.total_requests)} 个请求`}
        icon={<DollarSign className="h-6 w-6" />}
        iconBgColor="bg-green-100 dark:bg-green-900/20"
        iconColor="text-green-600 dark:text-green-400"
      />

      {/* 总请求数 */}
      <MetricCard
        title="总请求数"
        value={formatRequestCount(summary.total_requests)}
        subtitle={`成功 ${formatRequestCount(summary.successful_requests)} 个`}
        icon={<Activity className="h-6 w-6" />}
        iconBgColor="bg-blue-100 dark:bg-blue-900/20"
        iconColor="text-blue-600 dark:text-blue-400"
      />

      {/* 平均响应时间 */}
      <MetricCard
        title="平均响应时间"
        value={formatResponseTime(summary.avg_response_time)}
        subtitle={summary.avg_response_time !== null ? '所有请求平均值' : '暂无数据'}
        icon={<Clock className="h-6 w-6" />}
        iconBgColor="bg-purple-100 dark:bg-purple-900/20"
        iconColor="text-purple-600 dark:text-purple-400"
      />

      {/* 错误率 */}
      <MetricCard
        title="错误率"
        value={`${errorRate.toFixed(2)}%`}
        subtitle={`失败 ${summary.failed_requests} 个请求`}
        icon={<AlertCircle className="h-6 w-6" />}
        iconBgColor={
          errorRate > 5 ? 'bg-red-100 dark:bg-red-900/20' : 'bg-gray-100 dark:bg-gray-900/20'
        }
        iconColor={
          errorRate > 5 ? 'text-red-600 dark:text-red-400' : 'text-gray-600 dark:text-gray-400'
        }
      />
    </div>
  );
};
