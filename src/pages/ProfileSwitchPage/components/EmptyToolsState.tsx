import { Button } from '@/components/ui/button';
import { Card, CardContent } from '@/components/ui/card';
import { Package } from 'lucide-react';

interface EmptyToolsStateProps {
  onNavigateToInstall: () => void;
}

export function EmptyToolsState({ onNavigateToInstall }: EmptyToolsStateProps) {
  return (
    <Card className="shadow-sm border">
      <CardContent className="pt-6">
        <div className="text-center py-12">
          <Package className="h-16 w-16 mx-auto mb-4 text-muted-foreground opacity-30" />
          <h3 className="text-lg font-semibold mb-2">暂无已安装的工具</h3>
          <p className="text-sm text-muted-foreground mb-4">请先安装工具</p>
          <Button
            onClick={onNavigateToInstall}
            className="shadow-md hover:shadow-lg transition-all"
          >
            <Package className="mr-2 h-4 w-4" />
            前往安装
          </Button>
        </div>
      </CardContent>
    </Card>
  );
}
