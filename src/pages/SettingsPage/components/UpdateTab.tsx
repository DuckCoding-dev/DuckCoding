import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card';
import { Progress } from '@/components/ui/progress';
import { Badge } from '@/components/ui/badge';
import {
  Download,
  Check,
  AlertCircle,
  RefreshCw,
  Zap,
  Clock,
  HardDrive,
  Info,
  Loader2,
  RotateCcw,
} from 'lucide-react';
import { useUpdate } from '@/hooks/useUpdate';
import { InstallPackageSelector } from './InstallPackageSelector';
import type { UpdateInfo } from '@/lib/tauri-commands';

interface UpdateTabProps {
  updateInfo?: UpdateInfo | null;
  onUpdateCheck?: () => void;
}

export function UpdateTab({ updateInfo: externalUpdateInfo, onUpdateCheck }: UpdateTabProps) {
  const [showReleaseNotes, setShowReleaseNotes] = useState(false);
  const [showPackageSelector, setShowPackageSelector] = useState(false);

  const {
    updateInfo,
    updateStatus,
    downloadProgress,
    currentVersion,
    platformInfo,
    packageFormatInfo,
    isChecking,
    isDownloading,
    error,
    checkForUpdates,
    downloadAndInstallSpecificPackage,
    restartToUpdate,
    formatFileSize,
    formatSpeed,
    formatEta,
    isUpdateAvailable,
    isUpdateDownloaded,
    isUpdateInstalled,
    isUpdateFailed,
    downloadPercentage,
  } = useUpdate({ externalUpdateInfo, onExternalUpdateCheck: onUpdateCheck });

  const getStatusColor = () => {
    switch (updateStatus) {
      case 'Available':
        return 'bg-blue-500';
      case 'Downloading':
        return 'bg-yellow-500';
      case 'Downloaded':
        return 'bg-green-500';
      case 'Installing':
        return 'bg-purple-500';
      case 'Installed':
        return 'bg-green-600';
      case 'Failed':
        return 'bg-red-500';
      default:
        return 'bg-gray-500';
    }
  };

  const getStatusText = () => {
    switch (updateStatus) {
      case 'Idle':
        return '系统正常';
      case 'Checking':
        return '检查更新中...';
      case 'Available':
        return '有可用更新';
      case 'Downloading':
        return '下载中...';
      case 'Downloaded':
        return '下载完成';
      case 'Installing':
        return '安装中...';
      case 'Installed':
        return '安装完成，等待重启';
      case 'Failed':
        return '更新失败';
      default:
        return '未知状态';
    }
  };

  return (
    <div className="space-y-6">
      {/* 当前版本信息 */}
      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Info className="h-5 w-5" />
            当前版本信息
          </CardTitle>
          <CardDescription>查看当前应用版本和更新状态</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          <div className="flex items-center justify-between">
            <div>
              <div className="text-sm font-medium">当前版本</div>
              <div className="text-2xl font-bold">{currentVersion}</div>
            </div>
            <Badge variant="outline" className={`${getStatusColor()} text-white border-none`}>
              {getStatusText()}
            </Badge>
          </div>

          {platformInfo && (
            <div className="grid grid-cols-2 gap-4 pt-2 border-t">
              <div>
                <div className="text-sm text-gray-500">操作系统</div>
                <div className="font-medium">
                  {platformInfo.is_windows && 'Windows'}
                  {platformInfo.is_macos && 'macOS'}
                  {platformInfo.is_linux && 'Linux'}
                </div>
              </div>
              <div>
                <div className="text-sm text-gray-500">架构</div>
                <div className="font-medium">{platformInfo.arch}</div>
              </div>
            </div>
          )}

          {packageFormatInfo && (
            <div className="pt-2">
              <div className="text-sm text-gray-500 mb-2">支持的包格式</div>
              <div className="flex flex-wrap gap-2">
                {packageFormatInfo.preferred_formats.map((format: string, index: number) => (
                  <Badge key={index} variant="secondary" className="text-xs">
                    {format.replace(/_/g, '.').toUpperCase()}
                  </Badge>
                ))}
              </div>
            </div>
          )}
        </CardContent>
      </Card>

      {/* 更新信息 */}
      {updateInfo && updateInfo.has_update && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Zap className="h-5 w-5" />
              发现新版本
            </CardTitle>
            <CardDescription>最新版本 {updateInfo.latest_version} 可供下载</CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div>
                <div className="text-sm font-medium">新版本</div>
                <div className="text-lg font-semibold text-green-600">
                  {updateInfo.latest_version}
                </div>
              </div>
              <div>
                <div className="text-sm font-medium">文件大小</div>
                <div className="text-lg font-semibold">
                  {updateInfo.file_size ? formatFileSize(updateInfo.file_size) : '未知'}
                </div>
              </div>
            </div>

            {updateInfo.release_notes && (
              <div>
                <Button
                  variant="outline"
                  size="sm"
                  onClick={() => setShowReleaseNotes(!showReleaseNotes)}
                  className="mb-2"
                >
                  {showReleaseNotes ? '隐藏' : '显示'}更新说明
                </Button>
                {showReleaseNotes && (
                  <div className="p-3 bg-gray-50 rounded-md text-sm">
                    <pre className="whitespace-pre-wrap">{updateInfo.release_notes}</pre>
                  </div>
                )}
              </div>
            )}
          </CardContent>
        </Card>
      )}

      {/* 下载进度 */}
      {isDownloading && downloadProgress && !isUpdateDownloaded && (
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Download className="h-5 w-5" />
              下载更新
            </CardTitle>
          </CardHeader>
          <CardContent className="space-y-4">
            <div>
              <div className="flex justify-between text-sm mb-2">
                <span>下载进度</span>
                <span>{downloadPercentage.toFixed(1)}%</span>
              </div>
              <Progress value={downloadPercentage} className="w-full" />
            </div>

            <div className="grid grid-cols-3 gap-4 text-sm">
              <div className="flex items-center gap-1">
                <HardDrive className="h-4 w-4" />
                <span>
                  {formatFileSize(downloadProgress.downloaded_bytes)} /
                  {formatFileSize(downloadProgress.total_bytes)}
                </span>
              </div>
              {downloadProgress.speed && (
                <div className="flex items-center gap-1">
                  <RefreshCw className="h-4 w-4" />
                  <span>{formatSpeed(downloadProgress.speed)}</span>
                </div>
              )}
              {downloadProgress.eta && (
                <div className="flex items-center gap-1">
                  <Clock className="h-4 w-4" />
                  <span>剩余 {formatEta(downloadProgress.eta)}</span>
                </div>
              )}
            </div>
          </CardContent>
        </Card>
      )}

      {/* 操作按钮 */}
      <Card>
        <CardHeader>
          <CardTitle>更新操作</CardTitle>
          <CardDescription>检查、下载和安装应用更新</CardDescription>
        </CardHeader>
        <CardContent className="space-y-3">
          {/* 统一的按钮状态管理 - 使用互斥条件避免重复显示 */}
          {(() => {
            // 优先级：安装完成 > 下载完成 > 下载中 > 安装中 > 有更新 > 检查失败 > 无更新 > 初始状态

            // 1. 安装完成状态
            if (isUpdateInstalled) {
              return (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-green-600">
                    <Check className="h-4 w-4" />
                    <span>更新安装完成</span>
                  </div>
                  <div className="text-sm text-gray-600">请重启应用以使用新版本</div>
                  <Button onClick={restartToUpdate} className="w-full" variant="default">
                    <RotateCcw className="mr-2 h-4 w-4" />
                    重启应用
                  </Button>
                </div>
              );
            }

            // 2. 下载完成状态
            if (isUpdateDownloaded) {
              return (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-green-600">
                    <Check className="h-4 w-4" />
                    <span>更新已下载完成</span>
                  </div>
                  <div className="text-sm text-gray-600">点击下方按钮重启应用以完成安装</div>
                  <Button onClick={restartToUpdate} className="w-full" variant="default">
                    <RotateCcw className="mr-2 h-4 w-4" />
                    重启应用完成更新
                  </Button>
                </div>
              );
            }

            // 3. 下载中状态
            if (isDownloading) {
              return (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-blue-600">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    <span>正在下载更新...</span>
                  </div>
                  <div className="space-y-2">
                    <div className="flex justify-between text-sm">
                      <span>下载进度</span>
                      <span>{downloadPercentage.toFixed(1)}%</span>
                    </div>
                    <Progress value={downloadPercentage} className="w-full" />
                    {downloadProgress && (
                      <div className="text-xs text-gray-500">
                        {formatFileSize(downloadProgress.downloaded_bytes)} /
                        {formatFileSize(downloadProgress.total_bytes)}
                      </div>
                    )}
                  </div>
                  <Button className="w-full" disabled variant="outline">
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    下载中，请稍候...
                  </Button>
                </div>
              );
            }

            // 4. 安装中状态
            if (updateStatus === 'Installing') {
              return (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-purple-600">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    <span>正在安装更新...</span>
                  </div>
                  <Button className="w-full" disabled>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    安装中，请稍候...
                  </Button>
                </div>
              );
            }

            // 5. 发现更新状态
            if (isUpdateAvailable && updateInfo) {
              return (
                <Button onClick={() => setShowPackageSelector(true)} className="w-full">
                  <Download className="mr-2 h-4 w-4" />
                  选择安装包并更新
                </Button>
              );
            }

            // 6. 更新失败状态
            if (isUpdateFailed) {
              return (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-red-600">
                    <AlertCircle className="h-4 w-4" />
                    <span>更新失败</span>
                  </div>
                  {error && (
                    <div className="text-sm text-red-600 bg-red-50 p-2 rounded">{error}</div>
                  )}
                  <Button
                    onClick={checkForUpdates}
                    className="w-full"
                    variant="outline"
                    disabled={isChecking}
                  >
                    {isChecking ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        重新检查中...
                      </>
                    ) : (
                      <>
                        <RefreshCw className="mr-2 h-4 w-4" />
                        重新检查更新
                      </>
                    )}
                  </Button>
                </div>
              );
            }

            // 7. 无更新状态
            if (updateInfo && updateInfo.has_update === false) {
              return (
                <div className="space-y-3">
                  <div className="flex items-center gap-2 text-green-600">
                    <Check className="h-4 w-4" />
                    <span>已是最新版本</span>
                  </div>
                  <Button
                    onClick={checkForUpdates}
                    className="w-full"
                    variant="outline"
                    disabled={isChecking}
                  >
                    {isChecking ? (
                      <>
                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        重新检查中...
                      </>
                    ) : (
                      <>
                        <RefreshCw className="mr-2 h-4 w-4" />
                        重新检查
                      </>
                    )}
                  </Button>
                </div>
              );
            }

            // 8. 检查失败状态 (有错误但没有明确的状态)
            if (error && updateInfo) {
              return (
                <Button
                  onClick={checkForUpdates}
                  className="w-full"
                  variant="outline"
                  disabled={isChecking}
                >
                  {isChecking ? (
                    <>
                      <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                      重新检查中...
                    </>
                  ) : (
                    <>
                      <RefreshCw className="mr-2 h-4 w-4" />
                      重新检查
                    </>
                  )}
                </Button>
              );
            }

            // 9. 初始状态 - 未检查过
            return (
              <Button onClick={checkForUpdates} className="w-full" disabled={isChecking}>
                {isChecking ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    检查更新中...
                  </>
                ) : (
                  <>
                    <RefreshCw className="mr-2 h-4 w-4" />
                    检查更新
                  </>
                )}
              </Button>
            );
          })()}
        </CardContent>
      </Card>

      {/* 安装包选择器 */}
      <InstallPackageSelector
        isOpen={showPackageSelector}
        onClose={() => setShowPackageSelector(false)}
        updateInfo={updateInfo}
        onDownloadSelected={downloadAndInstallSpecificPackage}
        platformInfo={platformInfo}
      />
    </div>
  );
}
