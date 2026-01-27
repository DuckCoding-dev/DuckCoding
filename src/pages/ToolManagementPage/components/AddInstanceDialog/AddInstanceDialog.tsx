// 添加工具实例对话框（重构后主组件）
// 职责：步骤流程控制 + 状态协调 + Dialog 外壳

import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from '@/components/ui/dialog';
import { Button } from '@/components/ui/button';
import { useEffect, useCallback } from 'react';
import { Loader2 } from 'lucide-react';
import { open as openDialog } from '@tauri-apps/plugin-dialog';
import type { SSHConfig } from '@/types/tool-management';
import { listWslDistributions, addManualToolInstance } from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { getErrorMessage } from '@/utils/error';
import { useAddInstanceState } from './hooks/useAddInstanceState';
import { useToolScanner } from './hooks/useToolScanner';
import { useInstallerScanner } from './hooks/useInstallerScanner';
import { StepSelector } from './steps/StepSelector';
import { LocalAutoConfig } from './steps/LocalAutoConfig';
import { LocalManualConfig } from './steps/LocalManualConfig';
import { WslConfig } from './steps/WslConfig';
import { SshConfig } from './steps/SshConfig';

interface AddInstanceDialogProps {
  open: boolean;
  onClose: () => void;
  onAdd: (
    baseId: string,
    type: 'local' | 'wsl' | 'ssh',
    sshConfig?: SSHConfig,
    distroName?: string,
  ) => Promise<void>;
}

const TOOL_NAMES: Record<string, string> = {
  'claude-code': 'Claude Code',
  codex: 'CodeX',
  'gemini-cli': 'Gemini CLI',
};

export function AddInstanceDialog({ open, onClose, onAdd }: AddInstanceDialogProps) {
  const { toast } = useToast();
  const { state, actions } = useAddInstanceState();

  // 工具扫描 Hook
  const toolScanner = useToolScanner({
    onCandidatesFound: (candidates) => {
      actions.setToolCandidates(candidates);
      if (candidates.length === 1) {
        actions.setSelectedToolCandidate(candidates[0]);
        actions.setScanResult({ installed: true, version: candidates[0].version });
      }
    },
    onCandidateSelected: (candidate) => {
      actions.setSelectedToolCandidate(candidate);
      actions.setScanResult({ installed: true, version: candidate.version });
    },
    onScanStart: () => actions.setScanning(true),
    onScanEnd: () => actions.setScanning(false),
    onValidationStart: () => actions.setValidating(true),
    onValidationEnd: () => actions.setValidating(false),
    onValidationError: (error) => actions.setValidationError(error),
    onValidationSuccess: (version) => {
      actions.setScanResult({ installed: true, version });
      actions.setValidationError(null);
    },
  });

  // 安装器扫描 Hook
  const installerScanner = useInstallerScanner({
    onInstallersFound: (installers) => {
      actions.setInstallerCandidates(installers);
    },
    onInstallerSelected: (path, type) => {
      actions.setInstallerPath(path);
      // 根据类型自动设置安装方法
      const typeMap: Record<string, 'npm' | 'brew' | 'official' | 'other'> = {
        npm: 'npm',
        brew: 'brew',
        official: 'official',
      };
      if (type.toLowerCase() in typeMap) {
        actions.setInstallMethod(typeMap[type.toLowerCase() as keyof typeof typeMap]);
      }
    },
  });

  // 加载 WSL 发行版
  const loadWslDistros = useCallback(async () => {
    actions.setLoadingDistros(true);
    try {
      const distros = await listWslDistributions();
      actions.setWslDistros(distros);
      if (distros.length > 0) {
        actions.setSelectedDistro(distros[0]);
      }
    } catch (err) {
      toast({
        title: '加载WSL发行版失败',
        description: getErrorMessage(err),
        variant: 'destructive',
      });
      actions.setWslDistros([]);
    } finally {
      actions.setLoadingDistros(false);
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [toast]);

  // 对话框打开时重置状态
  useEffect(() => {
    if (open) {
      actions.resetAllState();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  // WSL 环境切换时加载发行版
  useEffect(() => {
    if (open && state.envType === 'wsl') {
      loadWslDistros();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open, state.envType]);

  // 工具/环境/方式变更时重置扫描状态
  useEffect(() => {
    actions.resetScanState();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [state.baseId, state.envType, state.localMethod]);

  // 浏览选择工具路径
  const handleBrowse = async () => {
    try {
      const isWindows = navigator.platform.toLowerCase().includes('win');
      const selected = await openDialog({
        directory: false,
        multiple: false,
        title: `选择 ${TOOL_NAMES[state.baseId]} 可执行文件`,
        filters: [
          {
            name: '可执行文件',
            extensions: isWindows ? ['exe', 'cmd', 'bat'] : [],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        actions.setManualPath(selected);
        toolScanner.validatePath(state.baseId, selected);
      }
    } catch (error) {
      toast({
        variant: 'destructive',
        title: '打开文件选择器失败',
        description: getErrorMessage(error),
      });
    }
  };

  // 浏览选择安装器路径
  const handleBrowseInstaller = async () => {
    try {
      const isWindows = navigator.platform.toLowerCase().includes('win');
      const selected = await openDialog({
        directory: false,
        multiple: false,
        title: `选择安装器可执行文件（${state.installMethod}）`,
        filters: [
          {
            name: '可执行文件',
            extensions: isWindows ? ['exe', 'cmd', 'bat'] : [],
          },
        ],
      });

      if (selected && typeof selected === 'string') {
        actions.setInstallerPath(selected);
      }
    } catch (error) {
      toast({
        variant: 'destructive',
        title: '打开文件选择器失败',
        description: getErrorMessage(error),
      });
    }
  };

  // 执行扫描/验证
  const handleScan = async () => {
    if (state.envType !== 'local') return;

    if (state.localMethod === 'auto') {
      // 自动扫描
      await toolScanner.scanAllCandidates(state.baseId, TOOL_NAMES[state.baseId]);
    } else {
      // 手动验证
      if (!state.manualPath) {
        toast({
          variant: 'destructive',
          title: '请选择路径',
        });
        return;
      }
      if (state.validationError) {
        toast({
          variant: 'destructive',
          title: '路径验证失败',
          description: state.validationError,
        });
        return;
      }

      // 验证路径并扫描安装器
      await toolScanner.validatePath(state.baseId, state.manualPath);
      await installerScanner.scanInstallersForPath(state.manualPath);

      toast({
        title: '验证成功',
        description: `${TOOL_NAMES[state.baseId]} v${state.scanResult?.version}`,
      });
    }
  };

  // 提交添加实例
  const handleSubmit = async () => {
    if (state.envType === 'local') {
      if (!state.scanResult || !state.scanResult.installed) {
        toast({
          variant: 'destructive',
          title: '无可用结果',
          description: '请先执行扫描',
        });
        return;
      }

      actions.setLoading(true);
      try {
        if (state.localMethod === 'auto') {
          // 自动扫描：使用选中的候选
          if (!state.selectedToolCandidate) {
            toast({
              variant: 'destructive',
              title: '请选择工具实例',
              description: '请从扫描结果中选择一个实例',
            });
            return;
          }

          const methodStr = state.selectedToolCandidate.install_method.toLowerCase();
          await addManualToolInstance(
            state.baseId,
            state.selectedToolCandidate.tool_path,
            methodStr,
            state.selectedToolCandidate.installer_path || undefined,
          );

          toast({
            title: '添加成功',
            description: `${TOOL_NAMES[state.baseId]} v${state.selectedToolCandidate.version}`,
          });
        } else {
          // 手动指定：验证并保存路径
          if (state.installMethod !== 'other' && !state.installerPath) {
            toast({
              variant: 'destructive',
              title: '请选择安装器路径',
              description: `${state.installMethod} 需要提供安装器路径`,
            });
            return;
          }

          await addManualToolInstance(
            state.baseId,
            state.manualPath,
            state.installMethod,
            state.installerPath || undefined,
          );
          toast({
            title: '添加成功',
            description: `${TOOL_NAMES[state.baseId]} 已成功添加`,
          });
        }

        await onAdd(state.baseId, 'local');
        handleClose();
      } catch (error) {
        toast({
          variant: 'destructive',
          title: '添加失败',
          description: getErrorMessage(error),
        });
      } finally {
        actions.setLoading(false);
      }
    } else if (state.envType === 'wsl') {
      if (!state.selectedDistro) {
        toast({
          title: '请选择WSL发行版',
          variant: 'destructive',
        });
        return;
      }

      actions.setLoading(true);
      try {
        await onAdd(state.baseId, state.envType, undefined, state.selectedDistro);
        handleClose();
      } finally {
        actions.setLoading(false);
      }
    }
  };

  const handleClose = () => {
    if (!state.loading && !state.scanning) {
      onClose();
      actions.resetAllState();
    }
  };

  const handleNext = () => {
    if (state.envType === 'wsl' && !state.selectedDistro) {
      toast({
        variant: 'destructive',
        title: '请选择 WSL 发行版',
      });
      return;
    }
    actions.setStep(2);
  };

  const handleBack = () => {
    actions.setStep(1);
    actions.resetScanState();
  };

  return (
    <Dialog open={open} onOpenChange={(isOpen) => !isOpen && !state.loading && onClose()} modal>
      <DialogContent className="sm:max-w-[600px]" onInteractOutside={(e) => e.preventDefault()}>
        <DialogHeader>
          <DialogTitle>添加工具实例</DialogTitle>
        </DialogHeader>

        <div className="space-y-6 py-4">
          {/* 第一步：选择工具、环境类型、添加方式 */}
          {state.step === 1 && (
            <StepSelector
              baseId={state.baseId}
              envType={state.envType}
              localMethod={state.localMethod}
              onBaseIdChange={actions.setBaseId}
              onEnvTypeChange={actions.setEnvType}
              onLocalMethodChange={actions.setLocalMethod}
            />
          )}

          {/* 第二步：配置详情 */}
          {state.step === 2 && state.envType === 'local' && state.localMethod === 'auto' && (
            <LocalAutoConfig
              toolName={TOOL_NAMES[state.baseId]}
              scanning={state.scanning}
              candidates={state.toolCandidates}
              selectedCandidate={state.selectedToolCandidate}
              onScan={handleScan}
              onSelectCandidate={toolScanner.selectCandidate}
            />
          )}

          {state.step === 2 && state.envType === 'local' && state.localMethod === 'manual' && (
            <LocalManualConfig
              toolName={TOOL_NAMES[state.baseId]}
              manualPath={state.manualPath}
              installMethod={state.installMethod}
              installerPath={state.installerPath}
              installerCandidates={state.installerCandidates}
              showCustomInstaller={state.showCustomInstaller}
              validating={state.validating}
              validationError={state.validationError}
              scanResult={state.scanResult}
              scanning={state.scanning}
              onPathChange={(path) => {
                actions.setManualPath(path);
                actions.setValidationError(null);
                actions.setScanResult(null);
                actions.setInstallerCandidates([]);
                actions.setShowCustomInstaller(false);
              }}
              onBrowse={handleBrowse}
              onValidate={() => toolScanner.validatePath(state.baseId, state.manualPath)}
              onScan={handleScan}
              onInstallMethodChange={actions.setInstallMethod}
              onInstallerPathChange={actions.setInstallerPath}
              onShowCustomInstallerChange={() =>
                actions.setShowCustomInstaller(!state.showCustomInstaller)
              }
              onBrowseInstaller={handleBrowseInstaller}
            />
          )}

          {state.step === 2 && state.envType === 'wsl' && (
            <WslConfig
              wslDistros={state.wslDistros}
              selectedDistro={state.selectedDistro}
              loadingDistros={state.loadingDistros}
              onDistroChange={actions.setSelectedDistro}
            />
          )}

          {state.step === 2 && state.envType === 'ssh' && <SshConfig />}
        </div>

        <DialogFooter>
          {state.step === 1 ? (
            <>
              <Button variant="outline" onClick={handleClose}>
                取消
              </Button>
              <Button onClick={handleNext}>下一步</Button>
            </>
          ) : (
            <>
              <Button
                variant="outline"
                onClick={handleBack}
                disabled={state.loading || state.scanning}
              >
                上一步
              </Button>
              <Button
                onClick={handleSubmit}
                disabled={
                  state.loading ||
                  state.scanning ||
                  state.envType === 'ssh' ||
                  (state.envType === 'local' &&
                    state.localMethod === 'auto' &&
                    !state.selectedToolCandidate) ||
                  (state.envType === 'local' &&
                    state.localMethod === 'manual' &&
                    (!state.scanResult || !state.scanResult.installed))
                }
              >
                {state.loading ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    添加中...
                  </>
                ) : (
                  '添加'
                )}
              </Button>
            </>
          )}
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
