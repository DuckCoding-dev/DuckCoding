import {
  Card,
  CardContent,
  CardFooter,
  CardHeader,
  CardTitle,
  CardDescription,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Pencil, Trash2, Download, Tag, Calendar, Coins } from 'lucide-react';
import type { RemoteToken } from '@/types/remote-token';
import { TOKEN_STATUS_TEXT, TOKEN_STATUS_VARIANT, TokenStatus } from '@/types/remote-token';

interface TokenCardProps {
  token: RemoteToken;
  onEdit: (token: RemoteToken) => void;
  onImport: (token: RemoteToken) => void;
  onDelete: (token: RemoteToken) => void;
}

export function TokenCard({ token, onEdit, onImport, onDelete }: TokenCardProps) {
  const formatTimestamp = (timestamp: number) => {
    if (timestamp === -1 || timestamp === 0) return '永不过期';
    return new Date(timestamp * 1000).toLocaleString('zh-CN');
  };

  const formatQuota = (quota: number, unlimited: boolean) => {
    if (unlimited) return '无限';
    return `$${(quota / 1000000).toFixed(2)}`;
  };

  return (
    <Card className="flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex justify-between items-start">
          <div>
            <CardTitle className="text-base font-medium flex items-center gap-2">
              {token.name}
            </CardTitle>
            <CardDescription className="mt-1">
              <Badge
                variant={TOKEN_STATUS_VARIANT[token.status as TokenStatus]}
                className="text-[10px] px-1 h-5"
              >
                {TOKEN_STATUS_TEXT[token.status as TokenStatus]}
              </Badge>
            </CardDescription>
          </div>
        </div>
      </CardHeader>

      <CardContent className="flex-1 pb-3 text-sm space-y-3">
        <div className="space-y-1">
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Tag className="h-3 w-3" /> 分组
          </span>
          <div className="font-medium">{token.group || '-'}</div>
        </div>

        <div className="space-y-1">
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Coins className="h-3 w-3" /> 剩余额度
          </span>
          <div className="font-mono">{formatQuota(token.remain_quota, token.unlimited_quota)}</div>
        </div>

        <div className="space-y-1">
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Calendar className="h-3 w-3" /> 过期时间
          </span>
          <div className="text-xs">{formatTimestamp(token.expired_time)}</div>
        </div>
      </CardContent>

      <CardFooter className="pt-0 gap-2 justify-end">
        <Button
          size="sm"
          variant="outline"
          className="h-8 text-xs flex-1"
          onClick={() => onImport(token)}
        >
          <Download className="h-3 w-3 mr-1.5" />
          导入
        </Button>

        <Button
          size="icon"
          variant="ghost"
          className="h-8 w-8"
          title="编辑"
          onClick={() => onEdit(token)}
        >
          <Pencil className="h-3.5 w-3.5" />
        </Button>

        <Button
          size="icon"
          variant="ghost"
          className="h-8 w-8 text-destructive hover:text-destructive"
          title="删除"
          onClick={() => onDelete(token)}
        >
          <Trash2 className="h-3.5 w-3.5" />
        </Button>
      </CardFooter>
    </Card>
  );
}
