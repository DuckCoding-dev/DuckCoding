import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Plus, Wallet } from 'lucide-react';

interface EmptyStateProps {
  onAdd: () => void;
}

export function EmptyState({ onAdd }: EmptyStateProps) {
  return (
    <Card className="border-dashed">
      <CardContent className="py-12 flex flex-col items-center gap-4 text-center">
        <div className="h-12 w-12 rounded-full bg-muted flex items-center justify-center">
          <Wallet className="h-6 w-6 text-muted-foreground" />
        </div>
        <div>
          <h3 className="text-lg font-semibold">暂无余额配置</h3>
          <p className="text-sm text-muted-foreground">添加一个 API 配置，开始监控余额</p>
        </div>
        <Button onClick={onAdd}>
          <Plus className="mr-2 h-4 w-4" />
          添加配置
        </Button>
      </CardContent>
    </Card>
  );
}
