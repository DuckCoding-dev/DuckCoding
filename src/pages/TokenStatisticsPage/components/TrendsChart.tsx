/**
 * 趋势图表组件
 * 使用 recharts 展示 Token 使用趋势、成本趋势等数据
 */
import React from 'react';
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  type TooltipProps,
} from 'recharts';
import type { TrendDataPoint } from '@/types/analytics';

/**
 * 数据线配置
 */
export interface DataKey {
  /** 数据字段名 */
  key: keyof TrendDataPoint;
  /** 线条颜色 */
  color: string;
  /** 显示名称 */
  name: string;
  /** 值格式化函数 */
  formatter?: (value: number) => string;
}

/**
 * 趋势图表组件属性
 */
export interface TrendsChartProps {
  /** 趋势数据点数组 */
  data: TrendDataPoint[];
  /** 图表标题 */
  title: string;
  /** 数据线配置数组 */
  dataKeys: DataKey[];
  /** 图表高度（像素） */
  height?: number;
  /** Y 轴标签 */
  yAxisLabel?: string;
}

/**
 * 格式化时间戳为可读日期（用于 Tooltip 和 X 轴）
 */
function formatTimestamp(
  timestamp: number,
  granularity: 'hour' | 'day' | 'week' | 'month',
): string {
  const date = new Date(timestamp);
  const month = (date.getMonth() + 1).toString().padStart(2, '0');
  const day = date.getDate().toString().padStart(2, '0');
  const hour = date.getHours().toString().padStart(2, '0');
  const minute = date.getMinutes().toString().padStart(2, '0');

  switch (granularity) {
    case 'hour':
      // 格式：01/10 19:00
      return `${month}/${day} ${hour}:${minute}`;
    case 'day':
      // 格式：01/10
      return `${month}/${day}`;
    case 'week':
    case 'month':
      // 格式：2026/01/10
      return `${date.getFullYear()}/${month}/${day}`;
    default:
      return `${month}/${day}`;
  }
}

/**
 * 自动检测时间粒度
 */
function detectGranularity(data: TrendDataPoint[]): 'hour' | 'day' | 'week' | 'month' {
  if (data.length < 2) return 'day';

  const timeSpan = data[data.length - 1].timestamp - data[0].timestamp;
  const hourMs = 60 * 60 * 1000;
  const dayMs = 24 * hourMs;

  if (timeSpan <= 2 * dayMs) return 'hour';
  if (timeSpan <= 7 * dayMs) return 'day';
  if (timeSpan <= 60 * dayMs) return 'week';
  return 'month';
}

/**
 * 自定义 Tooltip 组件
 */
const CustomTooltip: React.FC<
  TooltipProps<number, string> & {
    active?: boolean;
    payload?: Array<{ payload: TrendDataPoint }>;
    label?: string | number;
    dataKeys: DataKey[];
    granularity: 'hour' | 'day' | 'week' | 'month';
  }
> = ({ active, payload, label, dataKeys, granularity }) => {
  if (!active || !payload || payload.length === 0) {
    return null;
  }

  const timestamp = Number(label);
  const data = payload[0].payload as TrendDataPoint;

  return (
    <div className="rounded-lg border bg-white p-3 shadow-lg dark:bg-gray-800 dark:border-gray-700">
      <p className="mb-2 text-sm font-medium text-gray-900 dark:text-gray-100">
        {formatTimestamp(timestamp, granularity)}
      </p>
      <div className="space-y-1">
        {dataKeys.map((dk) => {
          const value = data[dk.key];
          const formatter = dk.formatter || ((v: number) => v.toLocaleString());
          return (
            <div key={dk.key} className="flex items-center justify-between gap-4 text-sm">
              <div className="flex items-center gap-2">
                <div className="h-2 w-2 rounded-full" style={{ backgroundColor: dk.color }} />
                <span className="text-gray-700 dark:text-gray-300">{dk.name}</span>
              </div>
              <span className="font-medium text-gray-900 dark:text-gray-100">
                {value !== null && value !== undefined ? formatter(Number(value)) : 'N/A'}
              </span>
            </div>
          );
        })}
      </div>
    </div>
  );
};

/**
 * 趋势图表组件
 */
export const TrendsChart: React.FC<TrendsChartProps> = ({
  data,
  title,
  dataKeys,
  height = 300,
  yAxisLabel,
}) => {
  const granularity = detectGranularity(data);

  if (data.length === 0) {
    return (
      <div className="rounded-lg border bg-white p-6 dark:bg-gray-800 dark:border-gray-700">
        <h3 className="mb-4 text-lg font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
        <div className="flex h-64 items-center justify-center text-gray-500 dark:text-gray-400">
          暂无数据
        </div>
      </div>
    );
  }

  return (
    <div className="rounded-lg border bg-white p-6 dark:bg-gray-800 dark:border-gray-700">
      <h3 className="mb-4 text-lg font-semibold text-gray-900 dark:text-gray-100">{title}</h3>
      <ResponsiveContainer width="100%" height={height}>
        <LineChart data={data} margin={{ top: 5, right: 30, left: 20, bottom: 5 }}>
          <CartesianGrid strokeDasharray="3 3" className="stroke-gray-200 dark:stroke-gray-700" />
          <XAxis
            dataKey="timestamp"
            tickFormatter={(timestamp) => formatTimestamp(timestamp, granularity)}
            className="text-xs text-gray-600 dark:text-gray-400"
          />
          <YAxis
            label={
              yAxisLabel ? { value: yAxisLabel, angle: -90, position: 'insideLeft' } : undefined
            }
            className="text-xs text-gray-600 dark:text-gray-400"
          />
          <Tooltip content={<CustomTooltip dataKeys={dataKeys} granularity={granularity} />} />
          <Legend
            wrapperStyle={{ paddingTop: '20px' }}
            iconType="line"
            formatter={(value) => (
              <span className="text-sm text-gray-700 dark:text-gray-300">{value}</span>
            )}
          />
          {dataKeys.map((dk) => (
            <Line
              key={dk.key}
              type="monotone"
              dataKey={dk.key}
              stroke={dk.color}
              strokeWidth={2}
              name={dk.name}
              dot={{ r: 3 }}
              activeDot={{ r: 5 }}
            />
          ))}
        </LineChart>
      </ResponsiveContainer>
    </div>
  );
};
