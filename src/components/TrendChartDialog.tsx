import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import { UsageChart } from '@/components/UsageChart';
import type { UsageStatsResult } from '@/lib/tauri-commands';

interface TrendChartDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  stats: UsageStatsResult | null;
  loading: boolean;
}

export function TrendChartDialog({ open, onOpenChange, stats, loading }: TrendChartDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-4xl">
        <DialogHeader>
          <DialogTitle>用量趋势</DialogTitle>
        </DialogHeader>
        <UsageChart stats={stats} loading={loading} />
      </DialogContent>
    </Dialog>
  );
}
