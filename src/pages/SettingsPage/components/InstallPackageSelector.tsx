import { useState } from 'react';
import { Button } from '@/components/ui/button';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { RadioGroup, RadioGroupItem } from '@/components/ui/radio-group';
import { Label } from '@/components/ui/label';
import { Package, Download, CheckCircle, AlertCircle } from 'lucide-react';

interface PackageOption {
  id: string;
  name: string;
  description: string;
  url?: string;
  icon: React.ReactNode;
  recommended?: boolean;
}

import type { UpdateInfo } from '@/lib/tauri-commands/types';

interface PlatformInfo {
  is_windows: boolean;
  is_macos: boolean;
  is_linux: boolean;
  arch: string;
}

interface InstallPackageSelectorProps {
  isOpen: boolean;
  onClose: () => void;
  updateInfo: UpdateInfo;
  onDownloadSelected: (url: string) => void;
  platformInfo: PlatformInfo;
}

export function InstallPackageSelector({
  isOpen,
  onClose,
  updateInfo,
  onDownloadSelected,
  platformInfo,
}: InstallPackageSelectorProps) {
  const [selectedPackage, setSelectedPackage] = useState('');
  const [isDownloading, setIsDownloading] = useState(false);

  // 根据平台生成可用选项
  const getAvailablePackages = (): PackageOption[] => {
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
          icon: <Package className="h-5 w-5" />,
          recommended: true,
        },
        {
          id: 'windows_exe',
          name: 'EXE 安装包',
          description: '传统的 Windows 安装程序',
          url: update.windows_exe,
          icon: <Download className="h-5 w-5" />,
        },
        {
          id: 'windows_portable',
          name: '便携版',
          description: '无需安装，解压即用，适合U盘使用',
          url: update.windows,
          icon: <Package className="h-5 w-5" />,
        },
      ].filter((pkg) => pkg.url); // 只显示有URL的选项
    }

    if (platformInfo.is_macos) {
      return [
        {
          id: 'macos_dmg',
          name: 'DMG 镜像',
          description: '推荐的 macOS 安装包，拖拽即可安装',
          url: update.macos_dmg,
          icon: <Package className="h-5 w-5" />,
          recommended: true,
        },
        {
          id: 'macos_pkg',
          name: 'PKG 安装包',
          description: '系统级安装包',
          url: update.macos,
          icon: <Package className="h-5 w-5" />,
        },
      ].filter((pkg) => pkg.url);
    }

    if (platformInfo.is_linux) {
      const packages: PackageOption[] = [
        {
          id: 'linux_appimage',
          name: 'AppImage',
          description: '通用 Linux 应用，无需安装，支持所有发行版',
          url: update.linux_appimage,
          icon: <Package className="h-5 w-5" />,
          recommended: true,
        },
      ];

      // 根据发行版添加包管理器选项
      // 这里简化处理，实际可以检测发行版
      if (update.linux_deb) {
        packages.push({
          id: 'linux_deb',
          name: 'DEB 包',
          description: '适用于 Ubuntu/Debian 及其衍生发行版',
          url: update.linux_deb,
          icon: <Package className="h-5 w-5" />,
        });
      }

      if (update.linux_rpm) {
        packages.push({
          id: 'linux_rpm',
          name: 'RPM 包',
          description: '适用于 Fedora/CentOS/RHEL 及其衍生发行版',
          url: update.linux_rpm,
          icon: <Package className="h-5 w-5" />,
        });
      }

      if (update.linux) {
        packages.push({
          id: 'linux_generic',
          name: '通用版',
          description: 'Linux 通用压缩包',
          url: update.linux,
          icon: <Package className="h-5 w-5" />,
        });
      }

      return packages.filter((pkg) => pkg.url);
    }

    return [];
  };

  const availablePackages = getAvailablePackages();

  const handleDownload = async () => {
    if (!selectedPackage) return;

    const selectedOption = availablePackages.find((pkg) => pkg.id === selectedPackage);
    if (!selectedOption?.url) return;

    setIsDownloading(true);
    try {
      await onDownloadSelected(selectedOption.url);
      onClose();
    } catch (error) {
      console.error('Download failed:', error);
    } finally {
      setIsDownloading(false);
    }
  };

  if (availablePackages.length === 0) {
    return (
      <Dialog open={isOpen} onOpenChange={onClose}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-orange-500" />
              无可用安装包
            </DialogTitle>
          </DialogHeader>
          <div className="text-center py-6">
            <p className="text-muted-foreground">很抱歉，当前平台暂无可用的安装包。</p>
          </div>
        </DialogContent>
      </Dialog>
    );
  }

  return (
    <Dialog open={isOpen} onOpenChange={onClose}>
      <DialogContent className="max-w-2xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Package className="h-5 w-5" />
            选择安装包格式
          </DialogTitle>
          <DialogDescription>
            为您的{' '}
            {platformInfo?.is_windows ? 'Windows' : platformInfo?.is_macos ? 'macOS' : 'Linux'}
            系统选择合适的安装包格式
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4">
          <RadioGroup value={selectedPackage} onValueChange={setSelectedPackage}>
            {availablePackages.map((pkg) => (
              <div key={pkg.id} className="relative">
                <RadioGroupItem value={pkg.id} id={pkg.id} className="peer sr-only" />
                <Label
                  htmlFor={pkg.id}
                  className="flex cursor-pointer rounded-lg border p-4 hover:bg-accent peer-data-[state=checked]:border-primary [&:has([data-state=checked])]:border-primary"
                >
                  <div className="flex items-start gap-3">
                    <div className="mt-1">{pkg.icon}</div>
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
            <Button variant="outline" onClick={onClose} disabled={isDownloading}>
              取消
            </Button>
            <Button onClick={handleDownload} disabled={!selectedPackage || isDownloading}>
              {isDownloading ? (
                <>
                  <Download className="mr-2 h-4 w-4 animate-pulse" />
                  下载中...
                </>
              ) : (
                <>
                  <Download className="mr-2 h-4 w-4" />
                  下载并安装
                </>
              )}
            </Button>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}
