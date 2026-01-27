import { LayoutGrid, List } from 'lucide-react';
import { Button } from '@/components/ui/button';
import { cn } from '@/lib/utils';

export type ViewMode = 'grid' | 'list';

interface ViewToggleProps {
  mode: ViewMode;
  onChange: (mode: ViewMode) => void;
}

export function ViewToggle({ mode, onChange }: ViewToggleProps) {
  return (
    <div className="flex items-center border rounded-md p-1 h-9 bg-muted/20">
      <Button
        variant="ghost"
        size="icon"
        className={cn(
          'h-7 w-7 rounded-sm',
          mode === 'grid' && 'bg-background shadow-sm hover:bg-background',
        )}
        onClick={() => onChange('grid')}
        title="卡片视图"
      >
        <LayoutGrid className="h-4 w-4" />
      </Button>
      <Button
        variant="ghost"
        size="icon"
        className={cn(
          'h-7 w-7 rounded-sm',
          mode === 'list' && 'bg-background shadow-sm hover:bg-background',
        )}
        onClick={() => onChange('list')}
        title="列表视图"
      >
        <List className="h-4 w-4" />
      </Button>
    </div>
  );
}
