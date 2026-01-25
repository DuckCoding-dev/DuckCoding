import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';

/**
 * 帮助弹窗组件
 */
export function HelpDialog({
  open,
  onOpenChange,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
}) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>配置管理帮助</DialogTitle>
          <DialogDescription>了解如何使用 Profile 配置管理功能</DialogDescription>
        </DialogHeader>
        <div className="space-y-4 text-sm">
          <div className="space-y-2">
            <h4 className="font-medium">1.正常配置模式[未开启透明代理]</h4>
            <p className="text-muted-foreground">
              切换配置后，如果工具正在运行，需要重启对应的工具才能使新配置生效。
            </p>
            <h4 className="font-medium">2.透明代理模式</h4>
            <p className="text-muted-foreground">
              切换配置请前往透明代理页面进行，切换配置后无需重启工具即可生效。
            </p>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}