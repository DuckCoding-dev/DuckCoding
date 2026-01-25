import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Building2, Pencil, Trash2, Coins, Globe, User, Clock } from 'lucide-react';
import type { Provider } from '@/lib/tauri-commands';

interface ProviderCardProps {
  provider: Provider;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
  onViewTokens: (providerId: string) => void;
}

export function ProviderCard({ provider, onEdit, onDelete, onViewTokens }: ProviderCardProps) {
  const formatTimestamp = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleString('zh-CN');
  };

  return (
    <Card className="flex flex-col">
      <CardHeader className="pb-3">
        <div className="flex justify-between items-start">
          <div className="flex items-center gap-3">
            <div className="p-2 bg-muted rounded-md">
              <Building2 className="h-5 w-5 text-primary" />
            </div>
            <div>
              <CardTitle className="text-base font-medium">{provider.name}</CardTitle>
              <CardDescription className="text-xs mt-1 flex items-center gap-1">
                <Clock className="h-3 w-3" />
                {formatTimestamp(provider.updated_at)}
              </CardDescription>
            </div>
          </div>
          {provider.is_default && (
            <Badge variant="secondary" className="text-xs">
              默认
            </Badge>
          )}
        </div>
      </CardHeader>

      <CardContent className="flex-1 pb-3 text-sm space-y-3">
        <div className="space-y-1">
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <Globe className="h-3 w-3" /> 官网地址
          </span>
          <a
            href={provider.website_url}
            target="_blank"
            rel="noopener noreferrer"
            className="text-primary hover:underline block truncate"
            title={provider.website_url}
          >
            {provider.website_url}
          </a>
        </div>

        <div className="space-y-1">
          <span className="text-xs text-muted-foreground flex items-center gap-1">
            <User className="h-3 w-3" /> 用户名
          </span>
          <div
            className="bg-muted/50 p-2 rounded text-xs font-mono truncate"
            title={provider.username || '-'}
          >
            {provider.username || '-'}
          </div>
        </div>
      </CardContent>

      <CardFooter className="pt-0 gap-2 justify-end">
        <Button
          size="sm"
          variant="outline"
          className="h-8 text-xs flex-1"
          onClick={() => onViewTokens(provider.id)}
        >
          <Coins className="h-3 w-3 mr-1.5" />
          查看令牌
        </Button>

        <Button
          size="icon"
          variant="ghost"
          className="h-8 w-8"
          title="编辑"
          onClick={() => onEdit(provider)}
        >
          <Pencil className="h-3.5 w-3.5" />
        </Button>

        <Button
          size="icon"
          variant="ghost"
          className="h-8 w-8 text-destructive hover:text-destructive"
          title="删除"
          disabled={provider.is_default}
          onClick={() => onDelete(provider)}
        >
          <Trash2 className="h-3.5 w-3.5" />
        </Button>
      </CardFooter>
    </Card>
  );
}
