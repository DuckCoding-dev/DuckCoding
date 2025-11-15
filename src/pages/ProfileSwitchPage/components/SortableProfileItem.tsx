import { Button } from '@/components/ui/button';
import { Loader2, ArrowRightLeft, Trash2, GripVertical } from 'lucide-react';
import { useSortable } from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

interface SortableProfileItemProps {
  profile: string;
  toolId: string;
  switching: boolean;
  deleting: boolean;
  onSwitch: (toolId: string, profile: string) => void;
  onDelete: (toolId: string, profile: string) => void;
}

export function SortableProfileItem({
  profile,
  toolId,
  switching,
  deleting,
  onSwitch,
  onDelete,
}: SortableProfileItemProps) {
  const { attributes, listeners, setNodeRef, transform, transition, isDragging } = useSortable({
    id: profile,
  });

  const style = {
    transform: CSS.Transform.toString(transform),
    transition,
    opacity: isDragging ? 0.5 : 1,
  };

  return (
    <div
      ref={setNodeRef}
      style={style}
      className="flex items-center justify-between p-3 bg-slate-50 dark:bg-slate-800 rounded-lg border hover:border-blue-300 dark:hover:border-blue-700 transition-colors"
    >
      <div className="flex items-center gap-2 flex-1">
        <button
          {...attributes}
          {...listeners}
          className="cursor-grab active:cursor-grabbing p-1 hover:bg-slate-200 dark:hover:bg-slate-700 rounded transition-colors"
          aria-label="拖拽排序"
        >
          <GripVertical className="h-4 w-4 text-slate-400" />
        </button>
        <span className="font-medium text-slate-900 dark:text-slate-100">{profile}</span>
      </div>
      <div className="flex items-center gap-2">
        <Button
          size="sm"
          variant="outline"
          onClick={() => onSwitch(toolId, profile)}
          disabled={switching || deleting}
          className="shadow-sm hover:shadow-md transition-all"
        >
          {switching ? (
            <>
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              切换中...
            </>
          ) : (
            <>
              <ArrowRightLeft className="h-3 w-3 mr-1" />
              切换
            </>
          )}
        </Button>
        <Button
          size="sm"
          variant="destructive"
          onClick={() => onDelete(toolId, profile)}
          disabled={switching || deleting}
          className="shadow-sm hover:shadow-md transition-all"
        >
          {deleting ? (
            <>
              <Loader2 className="h-3 w-3 mr-1 animate-spin" />
              删除中...
            </>
          ) : (
            <>
              <Trash2 className="h-3 w-3 mr-1" />
              删除
            </>
          )}
        </Button>
      </div>
    </div>
  );
}
