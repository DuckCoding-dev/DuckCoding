/**
 * 令牌详情卡片组件
 *
 * 展示选中令牌的详细信息（分组、额度、过期时间等）
 */

import type { RemoteToken, RemoteTokenGroup } from '@/types/remote-token';

interface TokenDetailCardProps {
  /** 令牌对象 */
  token: RemoteToken;
  /** 令牌所属分组（可选） */
  group?: RemoteTokenGroup;
}

/**
 * 令牌详情卡片
 */
export function TokenDetailCard({ token, group }: TokenDetailCardProps) {
  /**
   * 格式化过期时间
   */
  const formatExpireTime = (timestamp: number): string => {
    if (timestamp === 0) return '永不过期';
    return new Date(timestamp * 1000).toLocaleDateString('zh-CN', {
      year: 'numeric',
      month: '2-digit',
      day: '2-digit',
    });
  };

  /**
   * 格式化额度（500000 = $1）
   */
  const formatQuota = (quota: number): string => {
    return `$${(quota / 500000).toFixed(2)}`;
  };

  return (
    <div className="rounded-md border bg-muted/50 p-3 space-y-2">
      {/* 令牌名称 */}
      <div className="flex items-center justify-between text-sm">
        <span className="text-muted-foreground">令牌名称:</span>
        <span className="font-medium">{token.name}</span>
      </div>

      {/* 分组信息 */}
      <div className="flex items-start justify-between text-sm">
        <span className="text-muted-foreground">分组信息:</span>
        <div className="text-right">
          <p className="font-medium">{token.group}</p>
          {group && (
            <p className="text-xs text-muted-foreground">
              {group.desc} (倍率: {group.ratio})
            </p>
          )}
        </div>
      </div>

      {/* 剩余额度 / 总额度 */}
      <div className="flex items-center justify-between text-sm">
        <span className="text-muted-foreground">额度:</span>
        <span className="font-medium">
          {token.unlimited_quota
            ? '无限'
            : `${formatQuota(token.remain_quota)} / ${formatQuota(token.remain_quota + token.used_quota)}`}
        </span>
      </div>

      {/* 过期时间 */}
      <div className="flex items-center justify-between text-sm">
        <span className="text-muted-foreground">过期时间:</span>
        <span className="font-medium">{formatExpireTime(token.expired_time)}</span>
      </div>
    </div>
  );
}
