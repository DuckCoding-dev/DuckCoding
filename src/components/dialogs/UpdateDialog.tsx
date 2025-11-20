import { useEffect } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Progress } from '@/components/ui/progress';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';
import {
  Download,
  Check,
  AlertCircle,
  RefreshCw,
  Loader2,
  RotateCcw,
  Package,
  CheckCircle,
} from 'lucide-react';
import { useUpdate } from '@/hooks/useUpdate';
import { useToast } from '@/hooks/use-toast';
import type { UpdateInfo } from '@/lib/tauri-commands';
import { useState } from 'react';

interface UpdateDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  updateInfo?: UpdateInfo | null;
  onCheckForUpdate?: () => void;
}

export function UpdateDialog({
  open,
  onOpenChange,
  updateInfo: externalUpdateInfo,
  onCheckForUpdate: externalCheckForUpdate,
}: UpdateDialogProps) {
  const [showPackageSelector, setShowPackageSelector] = useState(false);
  const [selectedPackage, setSelectedPackage] = useState('');
  const { toast } = useToast();

  const {
    updateInfo,
    updateStatus,
    downloadProgress,
    currentVersion,
    platformInfo,
    isChecking,
    isDownloading,
    error,
    checkForUpdates,
    downloadAndInstallSpecificPackage,
    restartToUpdate,
    formatFileSize,
    isUpdateAvailable,
    isUpdateDownloaded,
    isUpdateInstalled,
    isUpdateFailed,
    downloadPercentage,
  } = useUpdate({
    externalUpdateInfo,
    onExternalUpdateCheck: externalCheckForUpdate,
  });

  // 判断是否可以关闭弹窗
  const canClose = !isDownloading && updateStatus !== 'Installing';

  // 处理弹窗关闭
  const handleOpenChange = (newOpen: boolean) => {
    if (!newOpen && !canClose) {
      // 下载中或安装中禁止关闭
      toast({
        title: '无法关闭',
        description: '更新正在进行中，请稍候...',
        variant: 'destructive',
      });
      return;
    }
    onOpenChange(newOpen);
  };

  // 当对话框打开时，如果没有更新信息则自动检查更新
  useEffect(() => {
    // 延迟一下，等待 props 更新完成
    const timer = setTimeout(() => {
      if (open && !updateInfo && !isChecking) {
        checkForUpdates();
      }
    }, 100); // 100ms 延迟，确保 updateInfo 已传递

    return () => clearTimeout(timer);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, updateInfo]); // 监听 open 和 updateInfo 的变化

  // 获取可用的安装包选项
  const getAvailablePackages = () => {
    if (!updateInfo?.update || !platformInfo) {
      return [];
    }

    const { update } = updateInfo;

    if (platformInfo.is_windows) {
      return [
        {
          id: 'windows_msi',
          name: 'MSI 安装包',
          description: '推荐的 Windows 安装包，支持静默安装和自动更新',
          url: update.windows_msi,
          recommended: true,
        },
        {
          id: 'windows_exe',
          name: 'EXE 安装包',
          description: '传统的 Windows 安装程序',
          url: update.windows_exe,
          recommended: false,
        },
        {
          id: 'windows_portable',
          name: '便携版',
          description: '无需安装，解压即用，适合U盘使用',
          url: update.windows,
          recommended: false,
        },
      ].filter((pkg) => pkg.url);
    }

    if (platformInfo.is_macos) {
      return [
        {
          id: 'macos_dmg',
          name: 'DMG 镜像',
          description: '推荐的 macOS 安装包，拖拽即可安装',
          url: update.macos_dmg,
          recommended: true,
        },
        {
          id: 'macos_pkg',
          name: 'PKG 安装包',
          description: '系统级安装包',
          url: update.macos,
          recommended: false,
        },
      ].filter((pkg) => pkg.url);
    }

    if (platformInfo.is_linux) {
      const packages = [
        {
          id: 'linux_appimage',
          name: 'AppImage',
          description: '通用 Linux 应用，无需安装，支持所有发行版',
          url: update.linux_appimage,
          recommended: true,
        },
      ];

      if (update.linux_deb) {
        packages.push({
          id: 'linux_deb',
          name: 'DEB 包',
          description: '适用于 Ubuntu/Debian 及其衍生发行版',
          url: update.linux_deb,
          recommended: false,
        });
      }

      if (update.linux_rpm) {
        packages.push({
          id: 'linux_rpm',
          name: 'RPM 包',
          description: '适用于 Fedora/CentOS/RHEL 及其衍生发行版',
          url: update.linux_rpm,
          recommended: false,
        });
      }

      if (update.linux) {
        packages.push({
          id: 'linux_generic',
          name: '通用版',
          description: 'Linux 通用压缩包',
          url: update.linux,
          recommended: false,
        });
      }

      return packages.filter((pkg) => pkg.url);
    }

    return [];
  };

  const availablePackages = getAvailablePackages();

  // 处理下载选中的包
  const handleDownloadPackage = async () => {
    if (!selectedPackage) return;

    const selectedOption = availablePackages.find((pkg) => pkg.id === selectedPackage);
    if (!selectedOption?.url) return;

    try {
      await downloadAndInstallSpecificPackage(selectedOption.url);
      setShowPackageSelector(false);
      setSelectedPackage('');
    } catch (error) {
      console.error('Download failed:', error);
      toast({
        title: '下载失败',
        description: error instanceof Error ? error.message : '未知错误',
        variant: 'destructive',
      });
    }
  };

  const renderContent = () => {
    // 1. 检查中状态 - 最高优先级
    if (isChecking) {
      return (
        <div className="space-y-4 py-8">
          <div className="flex flex-col items-center gap-4">
            <Loader2 className="h-8 w-8 animate-spin text-primary" />
            <p className="text-sm text-muted-foreground">正在检查更新...</p>
          </div>
        </div>
      );
    }

    // 2. 安装完成状态
    if (isUpdateInstalled) {
      return (
        <div className="space-y-4 py-4">
          <div className="flex flex-col items-center gap-4 text-green-600">
            <div className="h-12 w-12 rounded-full bg-green-100 flex items-center justify-center">
              <Check className="h-6 w-6" />
            </div>
            <div className="text-center">
              <h3 className="font-medium text-lg">更新安装完成</h3>
              <p className="text-sm text-muted-foreground mt-1">请重启应用以使用新版本</p>
            </div>
          </div>
          <Button onClick={restartToUpdate} className="w-full">
            <RotateCcw className="mr-2 h-4 w-4" />
            重启应用
          </Button>
        </div>
      );
    }

    // 2. 下载完成状态
    if (isUpdateDownloaded) {
      return (
        <div className="space-y-4 py-4">
          <div className="flex flex-col items-center gap-4 text-green-600">
            <div className="h-12 w-12 rounded-full bg-green-100 flex items-center justify-center">
              <Check className="h-6 w-6" />
            </div>
            <div className="text-center">
              <h3 className="font-medium text-lg">更新已下载</h3>
              <p className="text-sm text-muted-foreground mt-1">准备就绪，请重启应用以完成安装</p>
            </div>
          </div>
          <Button onClick={restartToUpdate} className="w-full">
            <RotateCcw className="mr-2 h-4 w-4" />
            重启应用完成更新
          </Button>
        </div>
      );
    }

    // 3. 下载中状态
    if (isDownloading) {
      return (
        <div className="space-y-4 py-4">
          <div className="flex flex-col items-center gap-4">
            <div className="h-12 w-12 rounded-full bg-blue-100 flex items-center justify-center">
              <Loader2 className="h-6 w-6 text-blue-600 animate-spin" />
            </div>
            <div className="text-center w-full">
              <h3 className="font-medium text-lg mb-2">正在下载更新...</h3>
              <div className="flex justify-between text-sm mb-2 px-1">
                <span>{downloadPercentage.toFixed(1)}%</span>
                {downloadProgress && (
                  <span>
                    {formatFileSize(downloadProgress.downloaded_bytes)} /
                    {formatFileSize(downloadProgress.total_bytes)}
                  </span>
                )}
              </div>
              <Progress value={downloadPercentage} className="w-full h-2" />
            </div>
          </div>
          <Button className="w-full" disabled variant="outline">
            下载中，请稍候...
          </Button>
        </div>
      );
    }

    // 4. 安装中状态
    if (updateStatus === 'Installing') {
      return (
        <div className="space-y-4 py-4">
          <div className="flex flex-col items-center gap-4">
            <div className="h-12 w-12 rounded-full bg-purple-100 flex items-center justify-center">
              <Loader2 className="h-6 w-6 text-purple-600 animate-spin" />
            </div>
            <div className="text-center">
              <h3 className="font-medium text-lg">正在安装更新...</h3>
              <p className="text-sm text-muted-foreground mt-1">请勿关闭应用</p>
            </div>
          </div>
          <Button className="w-full" disabled>
            安装中...
          </Button>
        </div>
      );
    }

    // 5. 发现更新状态
    if (isUpdateAvailable && updateInfo) {
      // 如果显示包选择器
      if (showPackageSelector) {
        return (
          <div className="space-y-4 py-2">
            <div className="text-sm text-muted-foreground mb-4">
              为您的{' '}
              {platformInfo?.is_windows ? 'Windows' : platformInfo?.is_macos ? 'macOS' : 'Linux'}
              系统选择合适的安装包格式
            </div>

            <RadioGroup value={selectedPackage} onValueChange={setSelectedPackage}>
              {availablePackages.map((pkg: any) => (
                <div key={pkg.id} className="relative">
                  <RadioGroupItem value={pkg.id} id={pkg.id} className="peer sr-only" />
                  <Label
                    htmlFor={pkg.id}
                    className="flex cursor-pointer rounded-lg border p-4 hover:bg-accent peer-data-[state=checked]:border-primary [&:has([data-state=checked])]:border-primary"
                  >
                    <div className="flex items-start gap-3 w-full">
                      <div className="mt-1">
                        <Package className="h-5 w-5" />
                      </div>
                      <div className="flex-1">
                        <div className="flex items-center gap-2">
                          <h4 className="font-medium">{pkg.name}</h4>
                          {pkg.recommended && (
                            <span className="inline-flex items-center rounded-full bg-primary/10 px-2 py-1 text-xs font-medium text-primary">
                              推荐
                            </span>
                          )}
                        </div>
                        <p className="text-sm text-muted-foreground mt-1">{pkg.description}</p>
                      </div>
                      <div className="flex items-center">
                        <CheckCircle className="h-4 w-4 text-primary opacity-0 peer-data-[state=checked]:opacity-100 transition-opacity" />
                      </div>
                    </div>
                  </Label>
                </div>
              ))}
            </RadioGroup>

            <div className="flex justify-end gap-3 pt-4 border-t">
              <Button variant="outline" onClick={() => setShowPackageSelector(false)}>
                返回
              </Button>
              <Button onClick={handleDownloadPackage} disabled={!selectedPackage}>
                <Download className="mr-2 h-4 w-4" />
                下载并安装
              </Button>
            </div>
          </div>
        );
      }

      // 默认显示更新详情
      return (
        <div className="space-y-4 py-2">
          <div className="bg-slate-50 dark:bg-slate-900 p-4 rounded-lg border space-y-3">
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium">新版本</span>
              <span className="text-green-600 font-bold">{updateInfo.latest_version}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium">当前版本</span>
              <span className="text-muted-foreground">{currentVersion}</span>
            </div>
            <div className="flex justify-between items-center">
              <span className="text-sm font-medium">文件大小</span>
              <span>{updateInfo.file_size ? formatFileSize(updateInfo.file_size) : '未知'}</span>
            </div>
          </div>

          {updateInfo.release_notes && (
            <div className="max-h-40 overflow-y-auto p-3 bg-slate-50 dark:bg-slate-900 rounded-lg text-sm border">
              <p className="font-medium mb-1">更新内容：</p>
              <pre className="whitespace-pre-wrap text-muted-foreground font-sans">
                {updateInfo.release_notes}
              </pre>
            </div>
          )}

          <Button onClick={() => setShowPackageSelector(true)} className="w-full">
            <Download className="mr-2 h-4 w-4" />
            立即更新
          </Button>
        </div>
      );
    }

    // 6. 更新失败状态
    if (isUpdateFailed) {
      return (
        <div className="space-y-4 py-4">
          <div className="flex flex-col items-center gap-4 text-red-600">
            <div className="h-12 w-12 rounded-full bg-red-100 flex items-center justify-center">
              <AlertCircle className="h-6 w-6" />
            </div>
            <div className="text-center">
              <h3 className="font-medium text-lg">更新失败</h3>
              <p className="text-sm text-red-500 mt-1 px-4">{error || '发生了未知错误'}</p>
            </div>
          </div>
          <Button onClick={checkForUpdates} variant="outline" className="w-full">
            <RefreshCw className="mr-2 h-4 w-4" />
            重试
          </Button>
        </div>
      );
    }

    // 7. 无更新状态
    if (updateInfo && updateInfo.has_update === false) {
      return (
        <div className="space-y-4 py-6">
          <div className="flex flex-col items-center gap-4 text-green-600">
            <div className="h-12 w-12 rounded-full bg-green-100 flex items-center justify-center">
              <Check className="h-6 w-6" />
            </div>
            <div className="text-center">
              <h3 className="font-medium text-lg">已是最新版本</h3>
              <p className="text-sm text-muted-foreground mt-1">
                当前版本 {currentVersion} 是最新的
              </p>
            </div>
          </div>
          <Button onClick={checkForUpdates} variant="outline" className="w-full">
            <RefreshCw className="mr-2 h-4 w-4" />
            再次检查
          </Button>
        </div>
      );
    }

    // 8. 默认状态（正在检查）
    return (
      <div className="space-y-4 py-8">
        <div className="flex flex-col items-center gap-4">
          <Loader2 className="h-8 w-8 animate-spin text-primary" />
          <p className="text-sm text-muted-foreground">正在检查更新...</p>
        </div>
      </div>
    );
  };

  return (
    <Dialog open={open} onOpenChange={handleOpenChange}>
      <DialogContent className="sm:max-w-[400px]">
        <DialogHeader>
          <DialogTitle>软件更新</DialogTitle>
          <DialogDescription>检查并安装 DuckCoding 的最新版本</DialogDescription>
        </DialogHeader>

        {renderContent()}
      </DialogContent>
    </Dialog>
  );
}
