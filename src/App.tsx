import { useState, useEffect, useRef, useCallback } from 'react';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import { Button } from '@/components/ui/button';
import {
  Card,
  CardContent,
  CardDescription,
  CardFooter,
  CardHeader,
  CardTitle,
} from '@/components/ui/card';
import { Badge } from '@/components/ui/badge';
import { Separator } from '@/components/ui/separator';
import { Input } from '@/components/ui/input';
import { Label } from '@/components/ui/label';
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select';
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog';
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs';
import {
  CheckCircle2,
  XCircle,
  Package,
  Settings as SettingsIcon,
  RefreshCw,
  LayoutDashboard,
  Loader2,
  Minimize2,
  AlertCircle,
  Save,
  ExternalLink,
  Info,
  ArrowRightLeft,
  Key,
  Sparkles,
  BarChart3,
  GripVertical,
  Trash2,
  Power,
} from 'lucide-react';
import {
  checkInstallations,
  checkNodeEnvironment,
  installTool,
  checkAllUpdates,
  updateTool,
  configureApi,
  listProfiles,
  switchProfile,
  deleteProfile,
  getActiveConfig,
  saveGlobalConfig,
  getGlobalConfig,
  generateApiKeyForTool,
  getUsageStats,
  getUserQuota,
  applyCloseAction,
  type ToolStatus,
  type NodeEnvironment,
  type ActiveConfig,
  type GlobalConfig,
  type UsageStatsResult,
  type UserQuotaResult,
  type CloseAction,
} from '@/lib/tauri-commands';
import { useToast } from '@/hooks/use-toast';
import { Toaster } from '@/components/ui/toaster';
import {
  DndContext,
  closestCenter,
  KeyboardSensor,
  PointerSensor,
  useSensor,
  useSensors,
  DragEndEvent,
} from '@dnd-kit/core';
import {
  arrayMove,
  SortableContext,
  sortableKeyboardCoordinates,
  useSortable,
  verticalListSortingStrategy,
} from '@dnd-kit/sortable';
import { CSS } from '@dnd-kit/utilities';

// Import logos
import ClaudeLogo from '@/assets/claude-logo.png';
import CodexLogo from '@/assets/codex-logo.png';
import GeminiLogo from '@/assets/gemini-logo.png';
import DuckLogo from '@/assets/duck-logo.png';

// Import statistics components
import { QuotaCard } from '@/components/QuotaCard';
import { UsageChart } from '@/components/UsageChart';
import { TodayStatsCard } from '@/components/TodayStatsCard';

interface ToolWithUpdate extends ToolStatus {
  hasUpdate?: boolean;
  latestVersion?: string;
  mirrorVersion?: string; // 镜像实际可安装的版本
  mirrorIsStale?: boolean; // 镜像是否滞后
}

const CLOSE_EVENT = 'duckcoding://request-close-action';
const CLOSE_PREFERENCE_KEY = 'duckcoding.closePreference';
const SINGLE_INSTANCE_EVENT = 'single-instance';

interface SingleInstancePayload {
  args: string[];
  cwd: string;
}

const isTauriEnvironment = () => {
  if (typeof window === 'undefined') {
    return false;
  }

  const globalWindow = window as unknown as Record<string, unknown>;
  return Boolean(
    globalWindow.__TAURI_INTERNALS__ ??
      globalWindow.__TAURI_METADATA__ ??
      globalWindow.__TAURI_IPC__,
  );
};

// 可拖拽的配置项组件
interface ProfileItemProps {
  profile: string;
  toolId: string;
  switching: boolean;
  deleting: boolean;
  onSwitch: (toolId: string, profile: string) => void;
  onDelete: (toolId: string, profile: string) => void;
}

function SortableProfileItem({
  profile,
  toolId,
  switching,
  deleting,
  onSwitch,
  onDelete,
}: ProfileItemProps) {
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

function App() {
  const { toast } = useToast();
  const [activeTab, setActiveTab] = useState('dashboard');
  const [tools, setTools] = useState<ToolWithUpdate[]>([]);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<string | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updateCheckMessage, setUpdateCheckMessage] = useState<{
    type: 'success' | 'error';
    text: string;
  } | null>(null);
  const [configuring, setConfiguring] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [deletingProfiles, setDeletingProfiles] = useState<Record<string, boolean>>({});

  // Ref to store timeout ID for cleanup
  const updateMessageTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastFetchTimeRef = useRef<number>(0);

  // API 配置表单状态
  const [selectedTool, setSelectedTool] = useState<string>('');
  const [provider, setProvider] = useState<string>('duckcoding');
  const [apiKey, setApiKey] = useState<string>('');
  const [baseUrl, setBaseUrl] = useState<string>('');
  const [profileName, setProfileName] = useState<string>('');

  // 配置切换状态
  const [profiles, setProfiles] = useState<Record<string, string[]>>({});
  const [selectedProfile, setSelectedProfile] = useState<Record<string, string>>({});
  const [activeConfigs, setActiveConfigs] = useState<Record<string, ActiveConfig>>({});
  const [selectedSwitchTab, setSelectedSwitchTab] = useState<string>(''); // 切换配置页面的Tab选择

  const [deleteConfirmDialog, setDeleteConfirmDialog] = useState<{
    open: boolean;
    toolId: string;
    profile: string;
  }>({ open: false, toolId: '', profile: '' });

  // 全局配置状态
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [userId, setUserId] = useState('');
  const [systemToken, setSystemToken] = useState('');
  const [savingSettings, setSavingSettings] = useState(false);
  const [generatingKey, setGeneratingKey] = useState(false);

  // 统计数据状态
  const [usageStats, setUsageStats] = useState<UsageStatsResult | null>(null);
  const [userQuota, setUserQuota] = useState<UserQuotaResult | null>(null);
  const [loadingStats, setLoadingStats] = useState(false);

  // Node环境检测状态
  const [nodeEnv, setNodeEnv] = useState<NodeEnvironment | null>(null);
  const [installMethods, setInstallMethods] = useState<Record<string, string>>({
    'claude-code': 'official',
    codex: navigator.userAgent.includes('Mac') ? 'brew' : 'npm', // CodeX 默认：macOS 用 brew，其他用 npm
    'gemini-cli': 'npm',
  });

  // 镜像滞后对话框状态
  const [mirrorStaleDialog, setMirrorStaleDialog] = useState<{
    open: boolean;
    toolId: string;
    mirrorVersion: string;
    officialVersion: string;
    source: 'install' | 'update';
  }>({ open: false, toolId: '', mirrorVersion: '', officialVersion: '', source: 'install' });

  // 配置覆盖确认对话框状态
  const [configOverrideDialog, setConfigOverrideDialog] = useState<{
    open: boolean;
    targetProfile: string;
    willOverride: boolean;
  }>({ open: false, targetProfile: '', willOverride: false });
  const [closeDialogOpen, setCloseDialogOpen] = useState(false);
  const [rememberCloseChoice, setRememberCloseChoice] = useState(false);
  const [closeActionLoading, setCloseActionLoading] = useState<CloseAction | null>(null);

  const logoMap: Record<string, string> = {
    'claude-code': ClaudeLogo,
    codex: CodexLogo,
    'gemini-cli': GeminiLogo,
  };

  const descriptionMap: Record<string, string> = {
    'claude-code': 'Anthropic 官方 CLI - AI 代码助手',
    codex: 'OpenAI 代码助手 - GPT-5 Codex',
    'gemini-cli': 'Google Gemini 命令行工具',
  };

  const groupNameMap: Record<string, string> = {
    'claude-code': 'Claude Code 专用分组',
    codex: 'CodeX 专用分组',
    'gemini-cli': 'Gemini CLI 专用分组',
  };

  const getToolDisplayName = (toolId: string) => {
    switch (toolId) {
      case 'claude-code':
        return 'Claude Code';
      case 'codex':
        return 'CodeX';
      case 'gemini-cli':
        return 'Gemini CLI';
      default:
        return toolId;
    }
  };

  // 拖拽排序相关 - Sensors
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    }),
  );

  // 保存配置文件排序
  const saveProfileOrder = (toolId: string, order: string[]) => {
    try {
      const key = `profile-order-${toolId}`;
      localStorage.setItem(key, JSON.stringify(order));
    } catch (error) {
      console.error('Failed to save profile order:', error);
    }
  };

  // 应用已保存的排序
  const applySavedOrder = useCallback((toolId: string, profiles: string[]): string[] => {
    let savedOrder: string[] = [];
    try {
      const key = `profile-order-${toolId}`;
      const saved = localStorage.getItem(key);
      savedOrder = saved ? JSON.parse(saved) : [];
    } catch (error) {
      console.error('Failed to load profile order:', error);
    }

    if (savedOrder.length === 0) return profiles;

    // 按照保存的顺序排列
    const ordered: string[] = [];
    const remaining = [...profiles];

    savedOrder.forEach((name) => {
      const index = remaining.indexOf(name);
      if (index !== -1) {
        ordered.push(name);
        remaining.splice(index, 1);
      }
    });

    // 将新增的配置文件添加到末尾
    return [...ordered, ...remaining];
  }, []);

  // 处理拖拽结束事件
  const handleDragEnd = (toolId: string) => (event: DragEndEvent) => {
    const { active, over } = event;

    if (over && active.id !== over.id) {
      setProfiles((prevProfiles) => {
        const toolProfiles = prevProfiles[toolId] || [];
        const oldIndex = toolProfiles.indexOf(active.id as string);
        const newIndex = toolProfiles.indexOf(over.id as string);

        if (oldIndex === -1 || newIndex === -1) return prevProfiles;

        const newOrder = arrayMove(toolProfiles, oldIndex, newIndex);
        saveProfileOrder(toolId, newOrder);

        return {
          ...prevProfiles,
          [toolId]: newOrder,
        };
      });
    }
  };

  // 版本号展示：保留 preview/beta 等标记
  const formatVersionLabel = (version: string | null): string => {
    if (!version) return '未知';
    const trimmed = version.trim();

    // 只要包含非数字的版本标记，就直接保留原样（如 preview、beta、rust-v 等）
    if (/[a-zA-Z]/.test(trimmed.replace(/^v/i, '')) || trimmed.includes('-')) {
      return trimmed;
    }

    const match = trimmed.match(/(\d+\.\d+\.\d+)/);
    return match ? match[1] : trimmed;
  };

  // 打开外部链接
  const openExternalLink = async (url: string) => {
    try {
      // 动态导入 shell 插件
      const { open } = await import('@tauri-apps/plugin-shell');
      await open(url);
      console.log('链接已在浏览器中打开:', url);
    } catch (error) {
      console.error('打开链接失败:', error);
      // 降级方案：在浏览器环境中使用 window.open
      if (typeof window !== 'undefined') {
        window.open(url, '_blank');
      }
    }
  };

  // 脱敏显示 API Key
  const maskApiKey = (key: string): string => {
    if (!key) return '';
    if (key.length <= 10) {
      return '*'.repeat(key.length);
    }
    const start = key.substring(0, 4);
    const end = key.substring(key.length - 4);
    const middle = '*'.repeat(Math.min(key.length - 8, 20)); // 最多显示20个星号
    return `${start}${middle}${end}`;
  };

  // 切换到配置页面并选择特定工具
  const switchToConfig = (toolId?: string) => {
    setActiveTab('config');
    if (toolId) {
      setSelectedTool(toolId);
    }
  };

  // 获取工具可用的安装方式
  const getAvailableInstallMethods = (
    toolId: string,
  ): Array<{ value: string; label: string; disabled?: boolean }> => {
    const isMac = navigator.userAgent.includes('Mac');

    if (toolId === 'claude-code') {
      return [
        { value: 'official', label: '官方脚本 (推荐)' },
        { value: 'npm', label: 'npm 安装', disabled: !nodeEnv?.npm_available },
      ];
    } else if (toolId === 'codex') {
      const methods = [{ value: 'npm', label: 'npm 安装', disabled: !nodeEnv?.npm_available }];
      if (isMac) {
        methods.unshift({ value: 'brew', label: 'Homebrew (推荐)', disabled: false });
      }
      // 注意:官方安装方法暂未实现，已移除
      return methods;
    } else if (toolId === 'gemini-cli') {
      return [{ value: 'npm', label: 'npm 安装 (推荐)', disabled: !nodeEnv?.npm_available }];
    }
    return [];
  };

  const checkUpdatesForInstalledTools = useCallback(async () => {
    try {
      // 使用批量 API 一次性检查所有工具
      const results = await checkAllUpdates();

      // 更新工具状态，添加更新信息（仅限已安装的工具）
      setTools((prevTools) =>
        prevTools.map((tool) => {
          const updateInfo = results.find((r) => r.tool_id === tool.id);
          if (tool.installed && updateInfo && updateInfo.success) {
            return {
              ...tool,
              hasUpdate: updateInfo.has_update,
              latestVersion: updateInfo.latest_version || undefined,
              mirrorVersion: updateInfo.mirror_version || undefined,
              mirrorIsStale: updateInfo.mirror_is_stale || false,
            };
          }
          return tool;
        }),
      );
    } catch (error) {
      console.error('Failed to check updates:', error);
    }
  }, []);

  const loadToolStatus = useCallback(async () => {
    try {
      setLoading(true);
      const status = await checkInstallations();
      setTools(status);

      // 自动检查已安装工具的更新（批量 API 会自动处理所有工具）
      checkUpdatesForInstalledTools();
    } catch (error) {
      console.error('Failed to check installations:', error);
      setTools([
        { id: 'claude-code', name: 'Claude Code', installed: false, version: null },
        { id: 'codex', name: 'CodeX', installed: false, version: null },
        { id: 'gemini-cli', name: 'Gemini CLI', installed: false, version: null },
      ]);
    } finally {
      setLoading(false);
    }
  }, [checkUpdatesForInstalledTools]);

  useEffect(() => {
    loadToolStatus();
    checkNodeEnvironment()
      .then((env: NodeEnvironment) => {
        setNodeEnv(env);
      })
      .catch((error: unknown) => {
        console.error('Failed to check node environment:', error);
      });
  }, [loadToolStatus]);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (updateMessageTimeoutRef.current) {
        clearTimeout(updateMessageTimeoutRef.current);
      }
    };
  }, []);

  // 当切换到配置页面时，自动选择第一个已安装的工具并加载配置列表
  const loadAllProfiles = useCallback(async () => {
    const installedTools = tools.filter((t) => t.installed);
    const profileData: Record<string, string[]> = {};
    const configData: Record<string, ActiveConfig> = {};

    for (const tool of installedTools) {
      try {
        const toolProfiles = await listProfiles(tool.id);
        // 应用保存的排序
        profileData[tool.id] = applySavedOrder(tool.id, toolProfiles);
      } catch (error) {
        console.error('Failed to load profiles for ' + tool.id, error);
        profileData[tool.id] = [];
      }

      try {
        const activeConfig = await getActiveConfig(tool.id);
        configData[tool.id] = activeConfig;
      } catch (error) {
        console.error('Failed to load active config for ' + tool.id, error);
        configData[tool.id] = { api_key: '未配置', base_url: '未配置' };
      }
    }

    setProfiles(profileData);
    setActiveConfigs(configData);

    // 设置默认选中的Tab（第一个已安装的工具）
    if (installedTools.length > 0 && !selectedSwitchTab) {
      setSelectedSwitchTab(installedTools[0].id);
    }
  }, [tools, selectedSwitchTab, applySavedOrder]);

  useEffect(() => {
    if (activeTab === 'config' || activeTab === 'switch') {
      const installedTools = tools.filter((t) => t.installed);
      if (!selectedTool && installedTools.length > 0) {
        setSelectedTool(installedTools[0].id);
      }
      // 加载配置列表以支持覆盖检测
      loadAllProfiles();

      // 备份功能已移除，无需额外加载
    }
  }, [activeTab, tools, selectedTool, loadAllProfiles]);

  // 加载时间戳备份列表
  // 加载全局配置
  const loadGlobalConfig = async () => {
    try {
      const config = await getGlobalConfig();
      if (config) {
        setGlobalConfig(config);
        setUserId(config.user_id);
        setSystemToken(config.system_token);
      }
    } catch (error) {
      console.error('Failed to load global config:', error);
    }
  };

  // 加载全局配置
  useEffect(() => {
    loadGlobalConfig();
  }, []);

  // 加载统计数据
  const loadStatistics = useCallback(async () => {
    if (!globalConfig?.user_id || !globalConfig?.system_token) {
      return;
    }

    // 频率限制：5秒内不允许重复请求
    const now = Date.now();
    if (lastFetchTimeRef.current && now - lastFetchTimeRef.current < 5000) {
      return;
    }
    lastFetchTimeRef.current = now;

    try {
      setLoadingStats(true);

      // 并行加载用量统计和额度信息
      const [statsResult, quotaResult] = await Promise.all([
        getUsageStats().catch((err) => {
          console.error('Failed to load usage stats:', err);
          return null;
        }),
        getUserQuota().catch((err) => {
          console.error('Failed to load user quota:', err);
          return null;
        }),
      ]);

      if (statsResult) {
        setUsageStats(statsResult);
      }
      if (quotaResult) {
        setUserQuota(quotaResult);
      }
    } catch (error) {
      console.error('Failed to load statistics:', error);
    } finally {
      setLoadingStats(false);
    }
  }, [globalConfig]);

  const executeCloseAction = useCallback(
    async (action: CloseAction, remember = false, autoTriggered = false) => {
      if (!isTauriEnvironment()) {
        setCloseDialogOpen(false);
        return;
      }

      setCloseActionLoading(action);
      try {
        await applyCloseAction(action);

        if (typeof window !== 'undefined') {
          try {
            if (remember) {
              window.localStorage.setItem(CLOSE_PREFERENCE_KEY, action);
            } else if (!autoTriggered) {
              window.localStorage.removeItem(CLOSE_PREFERENCE_KEY);
            }
          } catch (storageError) {
            console.warn('保存关闭偏好失败:', storageError);
          }
        }
      } catch (error) {
        console.error('执行窗口操作失败:', error);
        toast({
          variant: 'destructive',
          title: '窗口操作失败',
          description:
            error instanceof Error ? error.message : '请稍后重试，或从系统托盘退出/展开窗口',
        });

        if (!autoTriggered) {
          setCloseDialogOpen(true);
        }
      } finally {
        setCloseActionLoading(null);
        if (!autoTriggered) {
          setCloseDialogOpen(false);
          setRememberCloseChoice(false);
        }
      }
    },
    [toast],
  );

  // 当全局配置加载后，自动加载统计数据
  useEffect(() => {
    if (globalConfig?.user_id && globalConfig?.system_token) {
      loadStatistics();
    }
  }, [globalConfig, loadStatistics]);

  // 打开设置对话框时加载最新配置
  useEffect(() => {
    if (settingsOpen) {
      loadGlobalConfig();
    }
  }, [settingsOpen]);

  useEffect(() => {
    if (!isTauriEnvironment()) {
      return;
    }

    let unlisten: UnlistenFn | null = null;
    let disposed = false;

    listen(CLOSE_EVENT, () => {
      if (typeof window !== 'undefined') {
        try {
          const savedPreference = window.localStorage.getItem(
            CLOSE_PREFERENCE_KEY,
          ) as CloseAction | null;

          if (savedPreference === 'minimize' || savedPreference === 'quit') {
            executeCloseAction(savedPreference, true, true);
            return;
          }
        } catch (storageError) {
          console.warn('读取关闭偏好失败:', storageError);
        }
      }

      setCloseDialogOpen(true);
    })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((error) => {
        console.error('注册关闭事件监听失败:', error);
      });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [executeCloseAction]);

  useEffect(() => {
    if (!isTauriEnvironment()) {
      return;
    }

    let unlisten: UnlistenFn | null = null;
    let disposed = false;

    listen<SingleInstancePayload>(SINGLE_INSTANCE_EVENT, (event) => {
      const args = event.payload?.args?.slice(1).join(' ') ?? '';
      toast({
        title: 'DuckCoding 已在运行',
        description: args
          ? `已切换到当前实例（参数：${args}）`
          : '检测到重复启动，已切换到当前实例。',
      });
    })
      .then((fn) => {
        if (disposed) {
          fn();
        } else {
          unlisten = fn;
        }
      })
      .catch((error) => {
        console.error('注册 single-instance 事件监听失败:', error);
      });

    return () => {
      disposed = true;
      if (unlisten) {
        unlisten();
      }
    };
  }, [toast]);

  // 保存全局设置
  const handleSaveSettings = async () => {
    // 验证用户输入
    const trimmedUserId = userId.trim();
    const trimmedToken = systemToken.trim();

    if (!trimmedUserId || !trimmedToken) {
      toast({
        title: '验证失败',
        description: '请填写用户ID和系统访问令牌',
        variant: 'destructive',
      });
      return;
    }

    // 验证用户ID格式（应该是纯数字）
    if (!/^\d+$/.test(trimmedUserId)) {
      toast({
        title: '格式错误',
        description: '用户ID格式错误，应该是纯数字（例如：123456）',
        variant: 'destructive',
      });
      return;
    }

    // 验证系统访问令牌格式（最少5个字符）
    if (trimmedToken.length < 5) {
      toast({
        title: '格式错误',
        description: '系统访问令牌格式错误，长度至少需要5个字符',
        variant: 'destructive',
      });
      return;
    }

    try {
      setSavingSettings(true);
      await saveGlobalConfig(trimmedUserId, trimmedToken);
      setGlobalConfig({ user_id: trimmedUserId, system_token: trimmedToken });
      toast({
        title: '保存成功',
        description: '全局设置保存成功',
      });
      setSettingsOpen(false);
    } catch (error) {
      console.error('Failed to save settings:', error);
      toast({
        title: '保存失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSavingSettings(false);
    }
  };

  // 一键生成API Key
  const handleGenerateApiKey = async () => {
    if (!selectedTool) {
      toast({
        title: '请先选择工具',
        description: '请先选择要配置的工具',
        variant: 'destructive',
      });
      return;
    }

    if (!globalConfig?.user_id || !globalConfig?.system_token) {
      toast({
        title: '缺少配置',
        description: '请先在全局设置中配置用户ID和系统访问令牌',
        variant: 'destructive',
      });
      setSettingsOpen(true);
      return;
    }

    try {
      setGeneratingKey(true);
      const result = await generateApiKeyForTool(selectedTool);

      if (result.success && result.api_key) {
        setApiKey(result.api_key);
        toast({
          title: '生成成功',
          description: 'API Key生成成功！已自动填入配置框',
        });
      } else {
        toast({
          title: '生成失败',
          description: result.message || '未知错误',
          variant: 'destructive',
        });
      }
    } catch (error) {
      console.error('Failed to generate API key:', error);
      toast({
        title: '生成失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setGeneratingKey(false);
    }
  };

  const checkForUpdates = async () => {
    try {
      setCheckingUpdates(true);
      setUpdateCheckMessage(null); // Clear previous messages

      // Clear any existing timeout
      if (updateMessageTimeoutRef.current) {
        clearTimeout(updateMessageTimeoutRef.current);
        updateMessageTimeoutRef.current = null;
      }

      // 使用批量 API 一次性检查所有工具
      const results = await checkAllUpdates();

      // 更新工具状态，根据批量结果更新
      const updatedTools = tools.map((tool) => {
        const updateInfo = results.find((r) => r.tool_id === tool.id);
        if (updateInfo && updateInfo.success && tool.installed) {
          return {
            ...tool,
            hasUpdate: updateInfo.has_update,
            latestVersion: updateInfo.latest_version || undefined,
            mirrorVersion: updateInfo.mirror_version || undefined,
            mirrorIsStale: updateInfo.mirror_is_stale || false,
          };
        }
        return tool;
      });
      setTools(updatedTools);

      // Count updates available
      const updatesAvailable = updatedTools.filter((t) => t.hasUpdate).length;
      if (updatesAvailable > 0) {
        setUpdateCheckMessage({
          type: 'success',
          text: `发现 ${updatesAvailable} 个工具有可用更新！`,
        });
      } else {
        setUpdateCheckMessage({
          type: 'success',
          text: '所有工具均已是最新版本',
        });
      }

      // Auto-hide message after 5 seconds
      updateMessageTimeoutRef.current = setTimeout(() => {
        setUpdateCheckMessage(null);
        updateMessageTimeoutRef.current = null;
      }, 5000);
    } catch (error) {
      console.error('Failed to check for updates:', error);
      setUpdateCheckMessage({
        type: 'error',
        text: '检查更新失败，请重试',
      });
      // Auto-hide error message after 5 seconds
      updateMessageTimeoutRef.current = setTimeout(() => {
        setUpdateCheckMessage(null);
        updateMessageTimeoutRef.current = null;
      }, 5000);
    } finally {
      setCheckingUpdates(false);
    }
  };

  const handleInstall = async (toolId: string) => {
    try {
      setInstalling(toolId);
      const method = installMethods[toolId] || 'official';
      console.log(`Installing ${toolId} using method: ${method}`);
      await installTool(toolId, method);
      await loadToolStatus();
    } catch (error) {
      console.error('Failed to install ' + toolId, error);
      const errorMsg = String(error);

      // 检查是否是镜像滞后错误
      if (errorMsg.includes('MIRROR_STALE')) {
        const parts = errorMsg.split('|');
        if (parts.length === 3) {
          const mirrorVer = parts[1];
          const officialVer = parts[2];

          // 显示镜像滞后对话框
          setMirrorStaleDialog({
            open: true,
            toolId: toolId,
            mirrorVersion: mirrorVer,
            officialVersion: officialVer,
            source: 'install',
          });
          return; // 不显示 toast，由对话框处理
        }
      }

      toast({
        title: '安装失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setInstalling(null);
    }
  };

  const handleUpdate = async (toolId: string) => {
    // 防止重复点击
    if (updating) {
      toast({
        title: '请稍候',
        description: '已有更新任务正在进行，请等待完成后再试',
        variant: 'destructive',
      });
      return;
    }

    try {
      setUpdating(toolId);
      await updateTool(toolId);
      await loadToolStatus();
      toast({
        title: '更新成功',
        description: '工具已更新到最新版本',
      });
    } catch (error) {
      console.error('Failed to update ' + toolId, error);
      const errorMsg = String(error);

      // 检查是否是镜像滞后错误
      if (errorMsg.includes('MIRROR_STALE')) {
        const parts = errorMsg.split('|');
        if (parts.length === 3) {
          const mirrorVer = parts[1];
          const officialVer = parts[2];

          // 显示镜像滞后对话框
          setMirrorStaleDialog({
            open: true,
            toolId: toolId,
            mirrorVersion: mirrorVer,
            officialVersion: officialVer,
            source: 'update',
          });
          return; // 不显示 toast，由对话框处理
        }
      }

      // 检查是否是 Homebrew 锁定错误
      if (
        errorMsg.includes('already locked') ||
        errorMsg.includes('Please wait for it to finish')
      ) {
        toast({
          title: 'Homebrew 繁忙',
          description:
            'Homebrew 正在处理其他任务，请稍后重试。\n\n如需强制解锁，请在终端运行：\npkill -9 brew && rm -rf ~/Library/Caches/Homebrew/Locks/*',
          variant: 'destructive',
        });
      } else {
        toast({
          title: '更新失败',
          description: String(error),
          variant: 'destructive',
        });
      }
    } finally {
      setUpdating(null);
    }
  };

  const handleConfigureApi = async () => {
    if (!selectedTool || !apiKey) {
      toast({
        title: '请填写必填项',
        description: (!selectedTool ? '• 请选择工具\n' : '') + (!apiKey ? '• 请输入 API Key' : ''),
        variant: 'destructive',
      });
      return;
    }

    if (provider === 'custom' && !baseUrl.trim()) {
      toast({
        title: '请填写 Base URL',
        description: '选择自定义端点时必须填写有效的 Base URL',
        variant: 'destructive',
      });
      return;
    }

    // 确保拥有最新的配置数据（防止状态尚未同步）
    let currentConfig = activeConfigs[selectedTool];
    if (!currentConfig) {
      try {
        const latestConfig = await getActiveConfig(selectedTool);
        setActiveConfigs((prev) => ({ ...prev, [selectedTool]: latestConfig }));
        currentConfig = latestConfig;
      } catch (error) {
        console.error('Failed to fetch active config before saving:', error);
      }
    }

    // 检查是否会覆盖现有配置
    const existingProfiles = profiles[selectedTool] || [];

    // 只有真实配置存在才认为会覆盖（排除"未配置"的默认值）
    const hasRealConfig =
      currentConfig && currentConfig.api_key !== '未配置' && currentConfig.base_url !== '未配置';

    const willOverride = profileName ? existingProfiles.includes(profileName) : hasRealConfig;

    // 如果会覆盖且未确认，显示确认对话框
    if (willOverride && !configOverrideDialog.open) {
      setConfigOverrideDialog({
        open: true,
        targetProfile: profileName || '主配置',
        willOverride: true,
      });
      return;
    }

    // 执行保存配置（从确认对话框调用或无需确认时）
    await performConfigSave();
  };

  // 实际执行配置保存的函数
  const performConfigSave = async () => {
    try {
      setConfiguring(true);

      // 调用后端 API 并传递 backup 参数
      await configureApi(
        selectedTool,
        provider,
        apiKey,
        provider === 'custom' ? baseUrl.trim() : undefined,
        profileName || undefined,
      );

      // 清空表单
      setApiKey('');
      setBaseUrl('');
      setProfileName('');

      // 重新加载配置列表
      await loadAllProfiles();

      // 关闭确认对话框
      setConfigOverrideDialog({ open: false, targetProfile: '', willOverride: false });

      // 弹窗提示成功
      const toolName =
        selectedTool === 'claude-code'
          ? 'Claude Code'
          : selectedTool === 'codex'
            ? 'CodeX'
            : 'Gemini CLI';
      toast({
        title: '配置保存成功',
        description: `${toolName} 配置保存成功！${profileName ? `\n配置名称: ${profileName}` : ''}`,
      });
    } catch (error) {
      console.error('Failed to configure API:', error);
      toast({
        title: '配置失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setConfiguring(false);
    }
  };

  const handleSwitchProfile = async (toolId: string, profile: string) => {
    try {
      setSwitching(true);
      await switchProfile(toolId, profile);
      setSelectedProfile({ ...selectedProfile, [toolId]: profile });

      // 重新加载当前生效的配置
      try {
        const activeConfig = await getActiveConfig(toolId);
        setActiveConfigs({ ...activeConfigs, [toolId]: activeConfig });
      } catch (error) {
        console.error('Failed to reload active config', error);
      }

      toast({
        title: '切换成功',
        description: '配置切换成功！\n请重启相关 CLI 工具以使新配置生效。',
      });
    } catch (error) {
      console.error('Failed to switch profile:', error);
      toast({
        title: '切换失败',
        description: String(error),
        variant: 'destructive',
      });
    } finally {
      setSwitching(false);
    }
  };

  const handleDeleteProfile = (toolId: string, profile: string) => {
    setDeleteConfirmDialog({
      open: true,
      toolId,
      profile,
    });
  };

  const performDeleteProfile = async (toolId: string, profile: string) => {
    const profileKey = `${toolId}-${profile}`;

    try {
      setDeletingProfiles((prev) => ({ ...prev, [profileKey]: true }));

      // 后端删除
      await deleteProfile(toolId, profile);

      // 立即本地更新（乐观更新）
      const currentProfiles = profiles[toolId] || [];
      const updatedProfiles = currentProfiles.filter((p) => p !== profile);

      setProfiles((prev) => ({
        ...prev,
        [toolId]: updatedProfiles,
      }));

      // 更新本地排序存档
      saveProfileOrder(toolId, updatedProfiles);

      // 清理相关状态
      setSelectedProfile((prev) => {
        const updated = { ...prev };
        if (updated[toolId] === profile) {
          delete updated[toolId];
        }
        return updated;
      });

      // 尝试重新加载所有配置，确保与后端同步（失败不影响已显示的删除结果）
      try {
        await loadAllProfiles();

        // 如果删除的是当前正在使用的配置，重新获取当前配置
        if (activeConfigs[toolId]?.profile_name === profile) {
          try {
            const newActiveConfig = await getActiveConfig(toolId);
            setActiveConfigs((prev) => ({ ...prev, [toolId]: newActiveConfig }));
          } catch (error) {
            console.error('Failed to reload active config', error);
          }
        }
      } catch (reloadError) {
        console.error('Failed to reload profiles after delete:', reloadError);
        // 不影响用户体验，因为 UI 已经通过乐观更新反映了删除
      }

      toast({
        title: '删除成功',
        description: '配置删除成功！',
      });
    } catch (error) {
      console.error('Failed to delete profile:', error);
      toast({
        title: '删除失败',
        description: String(error),
        variant: 'destructive',
      });

      // 删除失败，重新加载列表恢复 UI 状态
      try {
        await loadAllProfiles();
      } catch (reloadError) {
        console.error('Failed to reload profiles after delete error:', reloadError);
      }
    } finally {
      setDeletingProfiles((prev) => {
        const updated = { ...prev };
        delete updated[profileKey];
        return updated;
      });

      setDeleteConfirmDialog({ open: false, toolId: '', profile: '' });
    }
  };

  const installedTools = tools.filter((t) => t.installed);
  const deleteDialogProfileKey = deleteConfirmDialog.toolId
    ? `${deleteConfirmDialog.toolId}-${deleteConfirmDialog.profile}`
    : '';
  const deleteDialogInProgress = deleteDialogProfileKey
    ? !!deletingProfiles[deleteDialogProfileKey]
    : false;

  return (
    <div className="flex h-screen bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-900 dark:to-slate-800">
      <aside className="w-64 border-r bg-white/80 dark:bg-slate-900/80 backdrop-blur-xl shadow-xl">
        <div className="p-6 flex items-center gap-3">
          <img src={DuckLogo} alt="DuckCoding" className="w-12 h-12 drop-shadow-lg" />
          <div>
            <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100">DuckCoding</h1>
            <p className="text-xs text-muted-foreground">一键配置中心</p>
          </div>
        </div>
        <Separator />
        <nav className="space-y-1 p-3">
          <Button
            variant={activeTab === 'dashboard' ? 'default' : 'ghost'}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab('dashboard')}
          >
            <LayoutDashboard className="mr-2 h-4 w-4" />
            仪表板
          </Button>
          <Button
            variant={activeTab === 'install' ? 'default' : 'ghost'}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab('install')}
          >
            <Package className="mr-2 h-4 w-4" />
            安装工具
          </Button>
          <Button
            variant={activeTab === 'config' ? 'default' : 'ghost'}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab('config')}
            disabled={installedTools.length === 0}
          >
            <Key className="mr-2 h-4 w-4" />
            配置 API
          </Button>
          <Button
            variant={activeTab === 'switch' ? 'default' : 'ghost'}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab('switch')}
            disabled={installedTools.length === 0}
          >
            <ArrowRightLeft className="mr-2 h-4 w-4" />
            切换配置
          </Button>
          <Button
            variant={activeTab === 'statistics' ? 'default' : 'ghost'}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab('statistics')}
          >
            <BarChart3 className="mr-2 h-4 w-4" />
            用量统计
          </Button>
          <Separator className="my-3" />
          <Button
            variant="ghost"
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setSettingsOpen(true)}
          >
            <SettingsIcon className="mr-2 h-4 w-4" />
            全局设置
          </Button>
        </nav>
        {installedTools.length === 0 && (
          <div className="px-3 pt-8">
            <div className="rounded-lg bg-blue-50 dark:bg-blue-950/50 p-3 text-xs text-blue-800 dark:text-blue-200 border border-blue-200 dark:border-blue-800">
              <Info className="h-4 w-4 mb-2" />
              <p>安装工具后即可配置 API</p>
            </div>
          </div>
        )}
      </aside>
      <main className="flex-1 overflow-auto">
        <div className="p-8">
          <div className="max-w-6xl mx-auto">
            {activeTab === 'dashboard' && (
              <div>
                <div className="mb-8 flex items-center justify-between">
                  <div>
                    <h2 className="text-4xl font-semibold tracking-tight mb-2 bg-gradient-to-r from-slate-900 to-slate-600 dark:from-slate-100 dark:to-slate-400 bg-clip-text text-transparent">
                      仪表板
                    </h2>
                    <p className="text-muted-foreground">管理您的 AI 开发工具</p>
                  </div>
                  <div className="flex flex-col items-end gap-2">
                    <Button
                      onClick={checkForUpdates}
                      disabled={checkingUpdates || tools.every((t) => !t.installed)}
                      variant="outline"
                      className="shadow-md hover:shadow-lg transition-all"
                    >
                      {checkingUpdates ? (
                        <>
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                          检查中...
                        </>
                      ) : (
                        <>
                          <RefreshCw className="mr-2 h-4 w-4" />
                          检查更新
                        </>
                      )}
                    </Button>
                    {updateCheckMessage && (
                      <div
                        className={`flex items-center gap-2 text-sm px-3 py-1.5 rounded-md animate-in fade-in slide-in-from-top-2 ${
                          updateCheckMessage.type === 'success'
                            ? 'bg-green-50 text-green-700 border border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-800'
                            : 'bg-red-50 text-red-700 border border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-800'
                        }`}
                      >
                        {updateCheckMessage.type === 'success' ? (
                          <CheckCircle2 className="h-4 w-4" />
                        ) : (
                          <AlertCircle className="h-4 w-4" />
                        )}
                        <span className="font-medium">{updateCheckMessage.text}</span>
                      </div>
                    )}
                  </div>
                </div>
                {loading ? (
                  <div className="flex items-center justify-center py-20">
                    <Loader2 className="h-10 w-10 animate-spin text-primary" />
                    <span className="ml-3 text-muted-foreground text-lg">加载中...</span>
                  </div>
                ) : (
                  <div className="space-y-8">
                    {/* 工具状态卡片 */}
                    <div className="grid grid-cols-1 md:grid-cols-3 gap-8">
                      {tools.map((tool) => (
                        <Card
                          key={tool.id}
                          className="hover:shadow-2xl transition-all duration-300 hover:scale-[1.02] border-2 bg-white/95 dark:bg-slate-900/95 backdrop-blur-sm overflow-hidden"
                        >
                          <CardHeader className="pb-4 space-y-4">
                            <div className="flex items-start justify-between">
                              <div className="bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-800 dark:to-slate-900 p-4 rounded-2xl shadow-lg">
                                <img
                                  src={logoMap[tool.id]}
                                  alt={tool.name}
                                  className="w-16 h-16 drop-shadow-xl"
                                />
                              </div>
                              <div className="flex flex-col gap-2 items-end">
                                {tool.installed ? (
                                  <>
                                    <Badge
                                      variant="default"
                                      className="gap-1.5 shadow-md px-3 py-1"
                                    >
                                      <CheckCircle2 className="h-3.5 w-3.5" />
                                      已安装
                                    </Badge>
                                    {tool.hasUpdate && (
                                      <Badge
                                        variant="destructive"
                                        className="gap-1.5 shadow-md animate-pulse px-3 py-1"
                                      >
                                        <AlertCircle className="h-3.5 w-3.5" />
                                        需要更新
                                      </Badge>
                                    )}
                                    {tool.installed && tool.hasUpdate === false && (
                                      <Badge
                                        variant="outline"
                                        className="gap-1.5 text-green-600 border-green-600 shadow-sm px-3 py-1"
                                      >
                                        <CheckCircle2 className="h-3.5 w-3.5" />
                                        最新版本
                                      </Badge>
                                    )}
                                  </>
                                ) : (
                                  <Badge
                                    variant="secondary"
                                    className="gap-1.5 shadow-md px-3 py-1"
                                  >
                                    <XCircle className="h-3.5 w-3.5" />
                                    未安装
                                  </Badge>
                                )}
                              </div>
                            </div>
                            <div className="space-y-2">
                              <CardTitle className="text-2xl font-semibold">{tool.name}</CardTitle>
                              <CardDescription className="text-sm leading-relaxed">
                                {descriptionMap[tool.id]}
                              </CardDescription>
                            </div>
                          </CardHeader>
                          <CardContent className="pb-4">
                            {tool.installed && tool.version ? (
                              <div className="space-y-2 bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-800/80 dark:to-slate-900/80 p-4 rounded-xl border border-slate-200 dark:border-slate-700">
                                <div className="flex items-center justify-between">
                                  <span className="text-sm font-semibold text-slate-600 dark:text-slate-400">
                                    当前版本
                                  </span>
                                  <span className="font-mono text-sm font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-3 py-1 rounded-lg">
                                    {formatVersionLabel(tool.version)}
                                  </span>
                                </div>
                                {tool.hasUpdate && tool.latestVersion && (
                                  <>
                                    <div className="flex items-center justify-between pt-1 border-t border-slate-200 dark:border-slate-700">
                                      <span className="text-sm font-semibold text-slate-600 dark:text-slate-400">
                                        官方最新
                                      </span>
                                      <span className="font-mono text-sm font-semibold text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950 px-3 py-1 rounded-lg">
                                        {formatVersionLabel(tool.latestVersion)}
                                      </span>
                                    </div>
                                    {tool.mirrorVersion &&
                                      tool.mirrorVersion !== tool.latestVersion && (
                                        <div className="flex items-center justify-between pt-1 border-t border-slate-200 dark:border-slate-700">
                                          <span className="text-sm font-semibold text-slate-600 dark:text-slate-400">
                                            镜像版本
                                          </span>
                                          <span className="font-mono text-sm font-semibold text-amber-600 dark:text-amber-400 bg-amber-50 dark:bg-amber-950 px-3 py-1 rounded-lg">
                                            {formatVersionLabel(tool.mirrorVersion)}
                                          </span>
                                        </div>
                                      )}
                                    {tool.mirrorIsStale && (
                                      <div className="mt-2 p-2 bg-amber-50 dark:bg-amber-950 border border-amber-200 dark:border-amber-800 rounded-lg">
                                        <div className="flex items-start gap-2">
                                          <AlertCircle className="h-4 w-4 text-amber-600 dark:text-amber-400 mt-0.5 flex-shrink-0" />
                                          <div className="text-xs text-amber-700 dark:text-amber-300">
                                            <p className="font-semibold mb-1">镜像版本滞后</p>
                                            <p className="text-amber-600 dark:text-amber-400">
                                              镜像站版本较旧，建议使用 npm 或官方脚本安装最新版本
                                            </p>
                                          </div>
                                        </div>
                                      </div>
                                    )}
                                  </>
                                )}
                              </div>
                            ) : (
                              <div className="text-sm text-center text-muted-foreground bg-slate-50 dark:bg-slate-800/50 p-4 rounded-xl border-2 border-dashed border-slate-200 dark:border-slate-700">
                                点击安装按钮开始使用
                              </div>
                            )}
                          </CardContent>
                          <CardFooter className="gap-3 pt-0 pb-5">
                            {tool.installed ? (
                              <>
                                <Button
                                  size="sm"
                                  variant="outline"
                                  className="flex-1 shadow-md hover:shadow-lg transition-all h-10"
                                  onClick={() => switchToConfig(tool.id)}
                                >
                                  <Key className="mr-2 h-4 w-4" />
                                  配置
                                </Button>
                                {tool.hasUpdate ? (
                                  <Button
                                    size="sm"
                                    className="flex-1 shadow-md hover:shadow-lg transition-all bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-600 hover:to-red-600 h-10"
                                    onClick={() => handleUpdate(tool.id)}
                                    disabled={updating === tool.id}
                                  >
                                    {updating === tool.id ? (
                                      <>
                                        <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                        更新中
                                      </>
                                    ) : (
                                      <>
                                        <RefreshCw className="mr-2 h-4 w-4" />
                                        更新
                                      </>
                                    )}
                                  </Button>
                                ) : (
                                  <Button
                                    size="sm"
                                    variant="outline"
                                    className="flex-1 shadow-md hover:shadow-lg transition-all h-10"
                                    onClick={checkForUpdates}
                                    disabled={checkingUpdates}
                                  >
                                    检查更新
                                  </Button>
                                )}
                              </>
                            ) : (
                              <Button
                                size="sm"
                                className="w-full shadow-md hover:shadow-xl transition-all bg-gradient-to-r from-blue-500 to-cyan-500 hover:from-blue-600 hover:to-cyan-600 h-10 font-medium"
                                onClick={() => handleInstall(tool.id)}
                                disabled={installing === tool.id}
                              >
                                {installing === tool.id ? (
                                  <>
                                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                    安装中...
                                  </>
                                ) : (
                                  <>
                                    <Package className="mr-2 h-4 w-4" />
                                    安装
                                  </>
                                )}
                              </Button>
                            )}
                          </CardFooter>
                        </Card>
                      ))}
                    </div>
                  </div>
                )}
              </div>
            )}

            {activeTab === 'statistics' && (
              <div>
                <div className="mb-6">
                  <h2 className="text-2xl font-semibold mb-1">用量统计</h2>
                  <p className="text-sm text-muted-foreground">
                    查看您的 DuckCoding API 使用情况和消费记录
                  </p>
                </div>

                {!globalConfig?.user_id || !globalConfig?.system_token ? (
                  <Card className="shadow-sm border">
                    <CardContent className="pt-6">
                      <div className="text-center py-12">
                        <BarChart3 className="h-16 w-16 mx-auto mb-4 text-muted-foreground opacity-30" />
                        <h3 className="text-lg font-semibold mb-2">需要配置凭证</h3>
                        <p className="text-sm text-muted-foreground mb-4">
                          请先在全局设置中配置您的用户ID和系统访问令牌
                        </p>
                        <Button
                          onClick={() => setSettingsOpen(true)}
                          className="shadow-md hover:shadow-lg transition-all"
                        >
                          <SettingsIcon className="mr-2 h-4 w-4" />
                          前往设置
                        </Button>
                      </div>
                    </CardContent>
                  </Card>
                ) : (
                  <div className="space-y-6">
                    <div className="flex items-center justify-end">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={loadStatistics}
                        disabled={loadingStats}
                        className="shadow-sm hover:shadow-md transition-all"
                      >
                        {loadingStats ? (
                          <>
                            <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                            加载中...
                          </>
                        ) : (
                          <>
                            <RefreshCw className="mr-2 h-4 w-4" />
                            刷新数据
                          </>
                        )}
                      </Button>
                    </div>

                    {/* 顶部卡片网格 - 2列 */}
                    <div className="grid grid-cols-1 lg:grid-cols-2 gap-6">
                      <QuotaCard quota={userQuota} loading={loadingStats} />
                      <TodayStatsCard stats={usageStats} loading={loadingStats} />
                    </div>

                    {/* 用量趋势图 - 全宽 */}
                    <UsageChart stats={usageStats} loading={loadingStats} />
                  </div>
                )}
              </div>
            )}

            {activeTab === 'install' && (
              <div>
                <div className="mb-6">
                  <h2 className="text-2xl font-semibold mb-1">安装工具</h2>
                  <p className="text-sm text-muted-foreground">选择并安装您需要的 AI 开发工具</p>
                </div>
                {loading ? (
                  <div className="flex items-center justify-center py-20">
                    <Loader2 className="h-8 w-8 animate-spin text-primary" />
                    <span className="ml-3 text-muted-foreground">加载中...</span>
                  </div>
                ) : (
                  <div className="grid gap-4">
                    {tools.map((tool) => (
                      <Card key={tool.id} className="shadow-sm border">
                        <CardContent className="p-5">
                          <div className="flex items-start justify-between gap-6">
                            <div className="flex items-center gap-4 flex-1">
                              <div className="bg-secondary p-3 rounded-lg flex-shrink-0">
                                <img src={logoMap[tool.id]} alt={tool.name} className="w-12 h-12" />
                              </div>
                              <div className="flex-1 space-y-1.5">
                                <div className="flex items-center gap-3">
                                  <h4 className="font-semibold text-lg">{tool.name}</h4>
                                  {tool.installed && (
                                    <Badge variant="default" className="gap-1">
                                      <CheckCircle2 className="h-3 w-3" />
                                      已安装
                                    </Badge>
                                  )}
                                </div>
                                <p className="text-sm text-muted-foreground leading-relaxed">
                                  {descriptionMap[tool.id]}
                                </p>
                                {tool.installed && tool.version && (
                                  <div className="flex items-center gap-2 mt-3">
                                    <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">
                                      当前版本:
                                    </span>
                                    <span className="font-mono text-xs font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-2.5 py-1 rounded-lg shadow-sm">
                                      {formatVersionLabel(tool.version)}
                                    </span>
                                  </div>
                                )}
                              </div>
                            </div>
                            <div className="flex flex-col gap-3 items-end">
                              {!tool.installed && (
                                <div className="w-48">
                                  <Label
                                    htmlFor={`method-${tool.id}`}
                                    className="text-xs mb-1.5 block"
                                  >
                                    安装方式
                                  </Label>
                                  <Select
                                    value={installMethods[tool.id]}
                                    onValueChange={(value) =>
                                      setInstallMethods({ ...installMethods, [tool.id]: value })
                                    }
                                  >
                                    <SelectTrigger
                                      id={`method-${tool.id}`}
                                      className="shadow-sm h-9 text-sm"
                                    >
                                      <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                      {getAvailableInstallMethods(tool.id).map((method) => (
                                        <SelectItem
                                          key={method.value}
                                          value={method.value}
                                          disabled={method.disabled}
                                        >
                                          {method.label}
                                        </SelectItem>
                                      ))}
                                    </SelectContent>
                                  </Select>
                                </div>
                              )}
                              <Button
                                disabled={tool.installed || installing === tool.id}
                                onClick={() => handleInstall(tool.id)}
                                className="shadow-md hover:shadow-lg transition-all bg-gradient-to-r from-blue-500 to-cyan-500 hover:from-blue-600 hover:to-cyan-600 disabled:from-slate-400 disabled:to-slate-400 h-11 px-6 font-medium w-48"
                                size="lg"
                              >
                                {installing === tool.id ? (
                                  <>
                                    <Loader2 className="mr-2 h-5 w-5 animate-spin" />
                                    安装中...
                                  </>
                                ) : tool.installed ? (
                                  <>
                                    <CheckCircle2 className="mr-2 h-5 w-5" />
                                    已安装
                                  </>
                                ) : (
                                  <>
                                    <Package className="mr-2 h-5 w-5" />
                                    安装工具
                                  </>
                                )}
                              </Button>
                            </div>
                          </div>
                        </CardContent>
                      </Card>
                    ))}
                  </div>
                )}
              </div>
            )}

            {activeTab === 'config' && (
              <div>
                <div className="mb-6">
                  <h2 className="text-2xl font-semibold mb-1">配置 API</h2>
                  <p className="text-sm text-muted-foreground">
                    配置 DuckCoding API 或自定义 API 端点
                  </p>
                </div>

                {installedTools.length > 0 ? (
                  <div className="grid gap-4">
                    {/* 重要提示 - 移到顶部 */}
                    <div className="p-4 bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800">
                      <div className="flex items-start gap-3">
                        <Info className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
                        <div className="space-y-2">
                          <h4 className="font-semibold text-amber-900 dark:text-amber-100">
                            重要提示
                          </h4>
                          <div className="text-sm text-amber-800 dark:text-amber-200 space-y-2">
                            <div>
                              <p className="font-semibold mb-1">DuckCoding API Key 分组:</p>
                              <ul className="list-disc list-inside space-y-1 ml-2">
                                {selectedTool && groupNameMap[selectedTool] && (
                                  <li>
                                    当前工具需要使用{' '}
                                    <span className="font-mono bg-amber-100 dark:bg-amber-900 px-1.5 py-0.5 rounded">
                                      {groupNameMap[selectedTool]}
                                    </span>{' '}
                                    的 API Key
                                  </li>
                                )}
                                <li>每个工具必须使用其专用分组的 API Key</li>
                                <li>API Key 不能混用</li>
                              </ul>
                            </div>
                            <div>
                              <p className="font-semibold mb-1">获取 API Key:</p>
                              <button
                                onClick={() =>
                                  openExternalLink('https://duckcoding.com/console/token')
                                }
                                className="inline-flex items-center gap-1 text-amber-700 dark:text-amber-300 hover:underline font-medium cursor-pointer bg-transparent border-0 p-0"
                              >
                                访问 DuckCoding 控制台 <ExternalLink className="h-3 w-3" />
                              </button>
                            </div>
                          </div>
                        </div>
                      </div>
                    </div>

                    <Card className="shadow-sm border">
                      <CardHeader>
                        <CardTitle>API 配置</CardTitle>
                        <CardDescription>为已安装的工具配置 API 密钥</CardDescription>
                      </CardHeader>
                      <CardContent className="space-y-6">
                        <div className="space-y-4">
                          <div className="space-y-2">
                            <Label htmlFor="tool-select">选择工具 *</Label>
                            <Select value={selectedTool} onValueChange={setSelectedTool}>
                              <SelectTrigger id="tool-select" className="shadow-sm">
                                <SelectValue placeholder="选择要配置的工具" />
                              </SelectTrigger>
                              <SelectContent>
                                {installedTools.map((tool) => (
                                  <SelectItem key={tool.id} value={tool.id}>
                                    <div className="flex items-center gap-2">
                                      <img src={logoMap[tool.id]} className="w-4 h-4" />
                                      {tool.name}
                                    </div>
                                  </SelectItem>
                                ))}
                              </SelectContent>
                            </Select>
                          </div>

                          <div className="space-y-2">
                            <Label htmlFor="provider-select">API 提供商 *</Label>
                            <Select value={provider} onValueChange={setProvider}>
                              <SelectTrigger id="provider-select" className="shadow-sm">
                                <SelectValue />
                              </SelectTrigger>
                              <SelectContent>
                                <SelectItem value="duckcoding">DuckCoding (推荐)</SelectItem>
                                <SelectItem value="custom">自定义端点</SelectItem>
                              </SelectContent>
                            </Select>
                          </div>

                          <div className="space-y-2">
                            <Label htmlFor="api-key">API Key *</Label>
                            <div className="flex gap-2">
                              <Input
                                id="api-key"
                                type="password"
                                placeholder="输入 API Key"
                                value={apiKey}
                                onChange={(e) => setApiKey(e.target.value)}
                                className="shadow-sm flex-1"
                              />
                              <Button
                                onClick={handleGenerateApiKey}
                                disabled={generatingKey || !selectedTool}
                                variant="outline"
                                className="shadow-sm hover:shadow-md transition-all"
                                title="一键生成 DuckCoding API Key"
                              >
                                {generatingKey ? (
                                  <>
                                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                                    生成中...
                                  </>
                                ) : (
                                  <>
                                    <Sparkles className="mr-2 h-4 w-4" />
                                    一键生成
                                  </>
                                )}
                              </Button>
                            </div>
                            <p className="text-xs text-muted-foreground">
                              点击"一键生成"可自动创建 DuckCoding API Key（需先配置全局设置）
                            </p>
                          </div>

                          {provider === 'duckcoding' && (
                            <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
                              <div className="flex items-start gap-3">
                                <Info className="h-5 w-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
                                <div className="space-y-2">
                                  <p className="text-sm font-medium text-blue-900 dark:text-blue-100">
                                    DuckCoding 默认配置
                                  </p>
                                  <div className="text-xs text-blue-700 dark:text-blue-300 space-y-1">
                                    <p>
                                      • Base URL:{' '}
                                      <code className="bg-white/50 dark:bg-slate-900/50 px-1.5 py-0.5 rounded">
                                        {selectedTool === 'codex'
                                          ? 'https://jp.duckcoding.com/v1'
                                          : 'https://jp.duckcoding.com'}
                                      </code>
                                    </p>
                                    <p>• 无需手动填写 Base URL，将自动使用默认端点</p>
                                    <p>
                                      • 切换配置后，请<strong>重启相关 CLI</strong> 以使新配置生效
                                    </p>
                                  </div>
                                </div>
                              </div>
                            </div>
                          )}

                          {provider === 'custom' && (
                            <div className="space-y-2">
                              <Label htmlFor="base-url">Base URL *</Label>
                              <Input
                                id="base-url"
                                type="url"
                                placeholder="https://api.example.com"
                                value={baseUrl}
                                onChange={(e) => setBaseUrl(e.target.value)}
                                className="shadow-sm"
                              />
                            </div>
                          )}

                          <div className="space-y-2">
                            <Label htmlFor="profile-name">配置文件名称 (可选)</Label>
                            <Input
                              id="profile-name"
                              type="text"
                              placeholder="例如: work, personal"
                              value={profileName}
                              onChange={(e) => setProfileName(e.target.value)}
                              className="shadow-sm"
                            />
                            <p className="text-xs text-muted-foreground">
                              留空将直接保存到主配置。填写名称可保存多个配置方便切换
                            </p>
                          </div>
                        </div>
                      </CardContent>
                      <CardFooter className="flex justify-between">
                        <Button
                          variant="outline"
                          onClick={() => {
                            setApiKey('');
                            setBaseUrl('');
                            setProfileName('');
                          }}
                          className="shadow-sm"
                        >
                          清空
                        </Button>
                        <Button
                          onClick={handleConfigureApi}
                          disabled={configuring || !selectedTool || !apiKey}
                          className="shadow-sm hover:shadow-md transition-all"
                        >
                          {configuring ? (
                            <>
                              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                              保存中...
                            </>
                          ) : (
                            <>
                              <Save className="mr-2 h-4 w-4" />
                              保存配置
                            </>
                          )}
                        </Button>
                      </CardFooter>
                    </Card>
                  </div>
                ) : (
                  <Card className="shadow-sm border">
                    <CardContent className="py-16 text-center">
                      <Package className="h-16 w-16 mx-auto mb-4 text-muted-foreground" />
                      <p className="text-muted-foreground mb-4 text-lg">请先安装工具后再进行配置</p>
                      <Button
                        onClick={() => setActiveTab('install')}
                        className="shadow-sm hover:shadow-md transition-all"
                      >
                        <Package className="mr-2 h-4 w-4" />
                        前往安装
                      </Button>
                    </CardContent>
                  </Card>
                )}
              </div>
            )}

            {activeTab === 'switch' && (
              <div>
                <div className="mb-6">
                  <h2 className="text-2xl font-semibold mb-1">切换配置</h2>
                  <p className="text-sm text-muted-foreground">在不同的配置文件之间快速切换</p>
                </div>

                {/* 重启提示 */}
                <div className="mb-6 p-4 bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800">
                  <div className="flex items-start gap-3">
                    <AlertCircle className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
                    <div className="space-y-1">
                      <h4 className="font-semibold text-amber-900 dark:text-amber-100">重要提示</h4>
                      <p className="text-sm text-amber-800 dark:text-amber-200">
                        切换配置后，如果工具正在运行，<strong>需要重启对应的工具</strong>
                        才能使新配置生效。
                      </p>
                    </div>
                  </div>
                </div>

                {installedTools.length > 0 ? (
                  <Tabs value={selectedSwitchTab} onValueChange={setSelectedSwitchTab}>
                    <TabsList className="grid w-full grid-cols-3 mb-6">
                      {installedTools.map((tool) => (
                        <TabsTrigger key={tool.id} value={tool.id} className="gap-2">
                          <img src={logoMap[tool.id]} alt={tool.name} className="w-4 h-4" />
                          {tool.name}
                        </TabsTrigger>
                      ))}
                    </TabsList>

                    {installedTools.map((tool) => {
                      const toolProfiles = profiles[tool.id] || [];
                      const activeConfig = activeConfigs[tool.id];
                      return (
                        <TabsContent key={tool.id} value={tool.id}>
                          <Card className="shadow-sm border">
                            <CardContent className="pt-6">
                              {/* 显示当前生效的配置 */}
                              {activeConfig && (
                                <div className="mb-6 p-4 bg-gradient-to-r from-blue-50 to-cyan-50 dark:from-blue-950 dark:to-cyan-950 rounded-lg border border-blue-200 dark:border-blue-800">
                                  <div className="flex items-center gap-2 mb-3">
                                    <Key className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                                    <h4 className="font-semibold text-blue-900 dark:text-blue-100">
                                      当前生效配置
                                    </h4>
                                  </div>
                                  <div className="space-y-2 text-sm">
                                    {activeConfig.profile_name && (
                                      <div className="flex items-start gap-2">
                                        <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">
                                          配置名称:
                                        </span>
                                        <span className="font-semibold text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded">
                                          {activeConfig.profile_name}
                                        </span>
                                      </div>
                                    )}
                                    <div className="flex items-start gap-2">
                                      <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">
                                        API Key:
                                      </span>
                                      <span className="font-mono text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded">
                                        {maskApiKey(activeConfig.api_key)}
                                      </span>
                                    </div>
                                    <div className="flex items-start gap-2">
                                      <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">
                                        Base URL:
                                      </span>
                                      <span className="font-mono text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded break-all">
                                        {activeConfig.base_url}
                                      </span>
                                    </div>
                                  </div>
                                </div>
                              )}

                              {toolProfiles.length > 0 ? (
                                <div className="space-y-3">
                                  <div className="flex items-center gap-2 mb-2">
                                    <Label>可用的配置文件（拖拽可调整顺序）</Label>
                                  </div>
                                  <DndContext
                                    sensors={sensors}
                                    collisionDetection={closestCenter}
                                    onDragEnd={handleDragEnd(tool.id)}
                                  >
                                    <SortableContext
                                      items={toolProfiles}
                                      strategy={verticalListSortingStrategy}
                                    >
                                      <div className="space-y-2">
                                        {toolProfiles.map((profile) => (
                                          <SortableProfileItem
                                            key={profile}
                                            profile={profile}
                                            toolId={tool.id}
                                            switching={switching}
                                            deleting={
                                              deletingProfiles[`${tool.id}-${profile}`] || false
                                            }
                                            onSwitch={handleSwitchProfile}
                                            onDelete={handleDeleteProfile}
                                          />
                                        ))}
                                      </div>
                                    </SortableContext>
                                  </DndContext>
                                </div>
                              ) : (
                                <div className="text-center py-8 bg-slate-50 dark:bg-slate-800/50 rounded-lg">
                                  <p className="text-muted-foreground mb-3">暂无保存的配置文件</p>
                                  <p className="text-sm text-muted-foreground">
                                    在"配置 API"页面保存配置时填写名称即可创建多个配置
                                  </p>
                                </div>
                              )}
                            </CardContent>
                          </Card>
                        </TabsContent>
                      );
                    })}
                  </Tabs>
                ) : (
                  <Card className="shadow-sm border">
                    <CardContent className="py-16 text-center">
                      <Package className="h-16 w-16 mx-auto mb-4 text-muted-foreground" />
                      <p className="text-muted-foreground mb-4 text-lg">请先安装工具</p>
                      <Button
                        onClick={() => setActiveTab('install')}
                        className="shadow-sm hover:shadow-md transition-all"
                      >
                        <Package className="mr-2 h-4 w-4" />
                        前往安装
                      </Button>
                    </CardContent>
                  </Card>
                )}
              </div>
            )}
          </div>
        </div>
      </main>

      {/* 全局设置对话框 */}
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="sm:max-w-[500px]" onPointerDown={(e) => e.stopPropagation()}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <SettingsIcon className="h-5 w-5" />
              全局设置
            </DialogTitle>
            <DialogDescription>
              配置 DuckCoding 用户ID 和系统访问令牌，用于一键生成 API Key
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="user-id">用户ID *</Label>
              <Input
                id="user-id"
                type="text"
                placeholder="在 DuckCoding 控制台个人中心查看"
                value={userId}
                onChange={(e) => setUserId(e.target.value)}
                className="shadow-sm"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="system-token">系统访问令牌 *</Label>
              <Input
                id="system-token"
                type="password"
                placeholder="在 DuckCoding 控制台系统配置中生成"
                value={systemToken}
                onChange={(e) => setSystemToken(e.target.value)}
                className="shadow-sm"
              />
            </div>
            <div className="p-3 bg-blue-50 dark:bg-blue-950/50 rounded-lg border border-blue-200 dark:border-blue-800 text-sm text-blue-800 dark:text-blue-200">
              <div className="flex items-start gap-2">
                <Info className="h-4 w-4 flex-shrink-0 mt-0.5" />
                <div className="space-y-1">
                  <p className="font-semibold">如何获取？</p>
                  <p>
                    1. 访问{' '}
                    <button
                      onClick={() => openExternalLink('https://duckcoding.com/console/personal')}
                      className="underline hover:text-blue-600 cursor-pointer bg-transparent border-0 p-0 inline"
                    >
                      个人中心
                    </button>{' '}
                    查看用户ID
                  </p>
                  <p>2. 在系统配置中生成系统访问令牌</p>
                </div>
              </div>
            </div>
          </div>
          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => setSettingsOpen(false)}
              disabled={savingSettings}
            >
              取消
            </Button>
            <Button
              type="button"
              onClick={(e) => {
                e.preventDefault();
                e.stopPropagation();
                handleSaveSettings();
              }}
              disabled={savingSettings}
              className="shadow-sm hover:shadow-md transition-all"
            >
              {savingSettings ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  保存中...
                </>
              ) : (
                <>
                  <Save className="mr-2 h-4 w-4" />
                  保存设置
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 镜像滞后对话框 */}
      <Dialog
        open={mirrorStaleDialog.open}
        onOpenChange={(open) => setMirrorStaleDialog({ ...mirrorStaleDialog, open })}
      >
        <DialogContent className="sm:max-w-[550px]" onPointerDown={(e) => e.stopPropagation()}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-amber-600" />
              镜像版本滞后
            </DialogTitle>
            <DialogDescription>检测到镜像站的版本落后于官方最新版本</DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {/* 版本对比 */}
            <div className="bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800 p-4">
              <div className="space-y-3">
                <div className="flex items-center justify-between">
                  <span className="text-sm font-semibold text-amber-900 dark:text-amber-100">
                    镜像版本
                  </span>
                  <span className="font-mono text-sm font-semibold text-amber-700 dark:text-amber-300 bg-white/50 dark:bg-slate-900/50 px-3 py-1.5 rounded-lg">
                    {mirrorStaleDialog.mirrorVersion}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-sm font-semibold text-amber-900 dark:text-amber-100">
                    官方最新
                  </span>
                  <span className="font-mono text-sm font-semibold text-green-700 dark:text-green-300 bg-white/50 dark:bg-slate-900/50 px-3 py-1.5 rounded-lg">
                    {mirrorStaleDialog.officialVersion}
                  </span>
                </div>
              </div>
            </div>

            {/* 说明文字 */}
            <div className="space-y-3 text-sm text-slate-700 dark:text-slate-300">
              <p>
                DuckCoding 镜像站的脚本版本（{mirrorStaleDialog.mirrorVersion}
                ）尚未同步到最新的官方版本（{mirrorStaleDialog.officialVersion}）。
              </p>
              <p className="font-semibold">建议：</p>
              <ul className="list-disc list-inside space-y-1 ml-2">
                <li>
                  改用 <strong>npm 安装</strong>可获取最新版本（{mirrorStaleDialog.officialVersion}
                  ）
                </li>
                <li>
                  或继续使用镜像安装较旧版本（{mirrorStaleDialog.mirrorVersion}），功能基本可用
                </li>
              </ul>
            </div>

            {/* npm 安装提示 */}
            {nodeEnv?.npm_available && (
              <div className="bg-green-50 dark:bg-green-950/50 border border-green-200 dark:border-green-800 rounded-lg p-4">
                <div className="flex items-start gap-2">
                  <Info className="h-4 w-4 text-green-600 dark:text-green-400 flex-shrink-0 mt-0.5" />
                  <div className="space-y-2 flex-1">
                    <p className="text-xs font-semibold text-green-800 dark:text-green-200">
                      点击"改用 npm 安装"将自动切换为 npm 方式并重新安装
                    </p>
                    <p className="text-xs text-green-700 dark:text-green-300">
                      npm 安装会直接从 npm 仓库获取最新版本
                    </p>
                  </div>
                </div>
              </div>
            )}

            {!nodeEnv?.npm_available && (
              <div className="bg-red-50 dark:bg-red-950/50 border border-red-200 dark:border-red-800 rounded-lg p-4">
                <div className="flex items-start gap-2">
                  <AlertCircle className="h-4 w-4 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
                  <div className="text-xs text-red-700 dark:text-red-300">
                    <p className="font-semibold mb-1">npm 不可用</p>
                    <p>您的系统未安装 npm 或无法检测到，只能继续使用镜像安装</p>
                  </div>
                </div>
              </div>
            )}
          </div>
          <DialogFooter className="gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setMirrorStaleDialog({
                  open: false,
                  toolId: '',
                  mirrorVersion: '',
                  officialVersion: '',
                  source: 'install',
                });
              }}
            >
              取消
            </Button>
            <Button
              type="button"
              variant="secondary"
              onClick={async () => {
                // 缓存所有需要的值，避免清空状态后无法访问
                const { toolId, source, mirrorVersion } = mirrorStaleDialog;
                setMirrorStaleDialog({
                  open: false,
                  toolId: '',
                  mirrorVersion: '',
                  officialVersion: '',
                  source: 'install',
                });

                // 根据来源调用强制安装或更新
                try {
                  if (source === 'install') {
                    setInstalling(toolId);
                    const method = installMethods[toolId] || 'official';
                    await installTool(toolId, method, true); // force=true
                    await loadToolStatus();
                    toast({
                      title: '安装成功',
                      description: `已安装镜像版本 ${mirrorVersion}`,
                    });
                  } else {
                    setUpdating(toolId);
                    await updateTool(toolId, true); // force=true
                    await loadToolStatus();
                    toast({
                      title: '更新成功',
                      description: `已更新到镜像版本 ${mirrorVersion}`,
                    });
                  }
                } catch (error) {
                  console.error('Failed to force install/update', error);
                  toast({
                    title: source === 'install' ? '安装失败' : '更新失败',
                    description: String(error),
                    variant: 'destructive',
                  });
                } finally {
                  if (source === 'install') {
                    setInstalling(null);
                  } else {
                    setUpdating(null);
                  }
                }
              }}
            >
              继续使用镜像 ({mirrorStaleDialog.mirrorVersion})
            </Button>
            {nodeEnv?.npm_available && (
              <Button
                type="button"
                onClick={async () => {
                  // 缓存所有需要的值
                  const { toolId, officialVersion } = mirrorStaleDialog;
                  setMirrorStaleDialog({
                    open: false,
                    toolId: '',
                    mirrorVersion: '',
                    officialVersion: '',
                    source: 'install',
                  });

                  // 改用 npm 安装
                  setInstallMethods({ ...installMethods, [toolId]: 'npm' });

                  // 重新触发安装
                  try {
                    setInstalling(toolId);
                    await installTool(toolId, 'npm');
                    await loadToolStatus();
                    toast({
                      title: '安装成功',
                      description: `已获取最新版本 ${officialVersion}`,
                    });
                  } catch (error) {
                    console.error('Failed to install with npm', error);
                    toast({
                      title: 'npm 安装失败',
                      description: String(error),
                      variant: 'destructive',
                    });
                  } finally {
                    setInstalling(null);
                  }
                }}
                className="shadow-sm hover:shadow-md transition-all bg-gradient-to-r from-green-600 to-emerald-600 hover:from-green-700 hover:to-emerald-700"
              >
                改用 npm 安装 ({mirrorStaleDialog.officialVersion})
              </Button>
            )}
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 删除配置确认对话框 */}
      <Dialog
        open={deleteConfirmDialog.open}
        onOpenChange={(open) => {
          if (!open && !deleteDialogInProgress) {
            setDeleteConfirmDialog({ open: false, toolId: '', profile: '' });
          }
        }}
      >
        <DialogContent className="sm:max-w-[480px]" onPointerDown={(e) => e.stopPropagation()}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-red-600" />
              确认删除配置
            </DialogTitle>
            <DialogDescription>
              此操作会永久删除 {getToolDisplayName(deleteConfirmDialog.toolId)} 的配置 「
              {deleteConfirmDialog.profile}」，该操作不可恢复，请谨慎确认。
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-3 text-sm text-slate-700 dark:text-slate-300">
            <p>
              删除后，{getToolDisplayName(deleteConfirmDialog.toolId)}{' '}
              将无法再使用该配置。如需要保留，请先备份或导出配置，再进行删除。
            </p>
            <div className="bg-red-50 dark:bg-red-950/40 border border-red-200 dark:border-red-800 rounded-lg p-4">
              <p className="text-xs text-red-700 dark:text-red-300">
                ⚠️ 注意：删除操作不可撤销，请确认已不再需要该配置。
              </p>
            </div>
          </div>
          <DialogFooter className="gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => setDeleteConfirmDialog({ open: false, toolId: '', profile: '' })}
              disabled={deleteDialogInProgress}
            >
              取消
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() =>
                performDeleteProfile(deleteConfirmDialog.toolId, deleteConfirmDialog.profile)
              }
              disabled={deleteDialogInProgress}
              className="shadow-sm hover:shadow-md transition-all"
            >
              {deleteDialogInProgress ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  删除中...
                </>
              ) : (
                <>
                  <Trash2 className="mr-2 h-4 w-4" />
                  确认删除
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      {/* 配置覆盖确认对话框 */}
      <Dialog
        open={configOverrideDialog.open}
        onOpenChange={(open) => setConfigOverrideDialog({ ...configOverrideDialog, open })}
      >
        <DialogContent className="sm:max-w-[500px]" onPointerDown={(e) => e.stopPropagation()}>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-amber-600" />
              配置已存在
            </DialogTitle>
            <DialogDescription>
              检测到配置 "{configOverrideDialog.targetProfile}" 已存在，是否覆盖？
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4 py-4">
            {/* 警告信息 */}
            <div className="bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800 p-4">
              <div className="flex items-start gap-3">
                <AlertCircle className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
                <div className="space-y-2">
                  <h4 className="font-semibold text-amber-900 dark:text-amber-100">覆盖确认</h4>
                  <p className="text-sm text-amber-800 dark:text-amber-200">
                    当前配置{' '}
                    <span className="font-mono bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded">
                      {configOverrideDialog.targetProfile}
                    </span>{' '}
                    已存在。继续保存将覆盖原有内容。
                  </p>
                </div>
              </div>
            </div>

            <p className="text-sm text-slate-700 dark:text-slate-300">
              如果需要保留旧配置，可在覆盖前手动复制信息或另存为其他名称。
            </p>
          </div>
          <DialogFooter className="gap-2">
            <Button
              type="button"
              variant="outline"
              onClick={() => {
                setConfigOverrideDialog({ open: false, targetProfile: '', willOverride: false });
              }}
              disabled={configuring}
            >
              取消
            </Button>
            <Button
              type="button"
              variant="destructive"
              onClick={() => {
                performConfigSave();
              }}
              disabled={configuring}
              className="shadow-sm hover:shadow-md transition-all"
            >
              {configuring ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  保存中...
                </>
              ) : (
                <>
                  <Save className="mr-2 h-4 w-4" />
                  确认覆盖
                </>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <Dialog
        open={closeDialogOpen}
        onOpenChange={(open) => {
          if (!open) {
            setCloseDialogOpen(false);
            setRememberCloseChoice(false);
          }
        }}
      >
        <DialogContent
          className="sm:max-w-[420px]"
          onPointerDown={(event) => event.stopPropagation()}
        >
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Power className="h-5 w-5 text-slate-600 dark:text-slate-100" />
              关闭 DuckCoding？
            </DialogTitle>
            <DialogDescription>选择关闭窗口或最小化到系统托盘。</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <Button
              variant="secondary"
              className="w-full justify-between"
              disabled={Boolean(closeActionLoading)}
              onClick={() => executeCloseAction('minimize', rememberCloseChoice)}
            >
              <span className="flex items-center gap-2">
                {closeActionLoading === 'minimize' ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Minimize2 className="h-4 w-4" />
                )}
                最小化到系统托盘
              </span>
              <span className="text-xs text-slate-500">推荐</span>
            </Button>
            <Button
              variant="destructive"
              className="w-full justify-between"
              disabled={Boolean(closeActionLoading)}
              onClick={() => executeCloseAction('quit', rememberCloseChoice)}
            >
              <span className="flex items-center gap-2">
                {closeActionLoading === 'quit' ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Power className="h-4 w-4" />
                )}
                直接退出程序
              </span>
              <span className="text-xs opacity-80">退出</span>
            </Button>
            <label className="flex items-center gap-2 text-sm text-slate-600 dark:text-slate-300">
              <input
                type="checkbox"
                checked={rememberCloseChoice}
                onChange={(event) => setRememberCloseChoice(event.target.checked)}
                className="h-4 w-4 rounded border-slate-300 text-blue-600 focus:ring-blue-500"
              />
              记住我的选择
            </label>
            <p className="text-xs text-slate-500 dark:text-slate-400">
              若取消勾选「记住我的选择」，下次点击关闭时会再次询问。
            </p>
          </div>
        </DialogContent>
      </Dialog>

      <Toaster />
    </div>
  );
}

export default App;
