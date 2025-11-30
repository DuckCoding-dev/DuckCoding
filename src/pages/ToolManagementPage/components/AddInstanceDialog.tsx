import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { useState, useEffect, useCallback } from 'react';
import type { SSHConfig } from '@/types/tool-management';
import { listWslDistributions } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';

interface AddInstanceDialogProps {
  open: boolean;
  onClose: () => void;
  onAdd: (
    baseId: string,
    type: 'wsl' | 'ssh',
    sshConfig?: SSHConfig,
    distroName?: string,
  ) => Promise<void>;
}

export function AddInstanceDialog({ open, onClose, onAdd }: AddInstanceDialogProps) {
  const { toast } = useToast();
  const [baseId, setBaseId] = useState('claude-code');
  const [envType, setEnvType] = useState<'wsl' | 'ssh'>('wsl');
  const [loading, setLoading] = useState(false);
  const [wslDistros, setWslDistros] = useState<string[]>([]);
  const [selectedDistro, setSelectedDistro] = useState<string>('');
  const [loadingDistros, setLoadingDistros] = useState(false);

  const loadWslDistros = useCallback(async () => {
    setLoadingDistros(true);
    try {
      const distros = await listWslDistributions();
      setWslDistros(distros);
      if (distros.length > 0) {
        setSelectedDistro(distros[0]); // 默认选择第一个
      }
    } catch (err) {
      toast({
        title: '加载WSL发行版失败',
        description: String(err),
        variant: 'destructive',
      });
      setWslDistros([]);
    } finally {
      setLoadingDistros(false);
    }
  }, [toast]);

  // 当选择WSL时，加载发行版列表
  useEffect(() => {
    if (open && envType === 'wsl') {
      loadWslDistros();
    }
  }, [open, envType, loadWslDistros]);

  const handleSubmit = async () => {
    if (envType === 'ssh') {
      // SSH功能暂不实现
      return;
    }

    if (envType === 'wsl' && !selectedDistro) {
      toast({
        title: '请选择WSL发行版',
        variant: 'destructive',
      });
      return;
    }

    setLoading(true);
    try {
      await onAdd(baseId, envType, undefined, selectedDistro);
      onClose();
      // 重置状态
      setBaseId('claude-code');
      setEnvType('wsl');
      setSelectedDistro('');
    } finally {
      setLoading(false);
    }
  };

  const handleClose = () => {
    if (!loading) {
      onClose();
      // 重置状态
      setBaseId('claude-code');
      setEnvType('wsl');
      setSelectedDistro('');
    }
  };

  return (
    <Dialog open={open} onOpenChange={handleClose}>
      <DialogContent>
        <DialogHeader>
          <DialogTitle>添加工具实例</DialogTitle>
        </DialogHeader>

        <div className="space-y-4 py-4">
          {/* 选择工具 */}
          <div className="space-y-2">
            <Label>选择工具</Label>
            <Select value={baseId} onValueChange={setBaseId}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="claude-code">Claude Code</SelectItem>
                <SelectItem value="codex">CodeX</SelectItem>
                <SelectItem value="gemini-cli">Gemini CLI</SelectItem>
              </SelectContent>
            </Select>
          </div>

          {/* 选择环境类型 */}
          <div className="space-y-2">
            <Label>环境类型</Label>
            <RadioGroup
              value={envType}
              onValueChange={(value) => setEnvType(value as 'wsl' | 'ssh')}
            >
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="wsl" id="wsl" />
                <Label htmlFor="wsl" className="font-normal cursor-pointer">
                  WSL 环境
                </Label>
              </div>
              <div className="flex items-center space-x-2">
                <RadioGroupItem value="ssh" id="ssh" disabled />
                <Label htmlFor="ssh" className="font-normal text-muted-foreground">
                  SSH 远程环境 (开发中)
                </Label>
              </div>
            </RadioGroup>
          </div>

          {/* WSL发行版选择 */}
          {envType === 'wsl' && (
            <div className="space-y-2">
              <Label>选择WSL发行版</Label>
              {loadingDistros ? (
                <div className="rounded border p-3 bg-muted/50 text-sm text-center">加载中...</div>
              ) : wslDistros.length === 0 ? (
                <div className="rounded border p-3 bg-yellow-50 dark:bg-yellow-950/30">
                  <p className="text-sm text-yellow-800 dark:text-yellow-200">
                    未检测到WSL发行版，请先安装WSL
                  </p>
                </div>
              ) : (
                <>
                  <Select value={selectedDistro} onValueChange={setSelectedDistro}>
                    <SelectTrigger>
                      <SelectValue placeholder="选择发行版" />
                    </SelectTrigger>
                    <SelectContent>
                      {wslDistros.map((distro) => (
                        <SelectItem key={distro} value={distro}>
                          {distro}
                        </SelectItem>
                      ))}
                    </SelectContent>
                  </Select>
                  <div className="rounded border p-3 bg-blue-50 dark:bg-blue-950/30">
                    <p className="text-sm text-blue-800 dark:text-blue-200">
                      将在 {selectedDistro} 中检测工具安装状态
                    </p>
                  </div>
                </>
              )}
            </div>
          )}

          {/* SSH配置表单（预留） */}
          {envType === 'ssh' && (
            <div className="rounded border p-3 bg-muted/50">
              <p className="text-sm text-muted-foreground">SSH功能将在后续版本提供</p>
            </div>
          )}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={loading}>
            取消
          </Button>
          <Button
            onClick={handleSubmit}
            disabled={loading || envType === 'ssh' || (envType === 'wsl' && !selectedDistro)}
          >
            {loading ? '添加中...' : '添加'}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
