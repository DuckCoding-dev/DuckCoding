import { useState, useEffect, useRef, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Separator } from "@/components/ui/separator";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { CheckCircle2, XCircle, Package, Settings as SettingsIcon, RefreshCw, LayoutDashboard, Loader2, AlertCircle, Save, ExternalLink, Info, ArrowRightLeft, Key, Sparkles, BarChart3, GripVertical, Trash2 } from "lucide-react";
import { checkInstallations, checkNodeEnvironment, installTool, checkAllUpdates, updateTool, configureApi, listProfiles, switchProfile, deleteProfile, getActiveConfig, saveGlobalConfig, getGlobalConfig, generateApiKeyForTool, getUsageStats, getUserQuota, type ToolStatus, type NodeEnvironment, type ActiveConfig, type GlobalConfig, type UsageStatsResult, type UserQuotaResult } from "@/lib/tauri-commands";
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
import ClaudeLogo from "@/assets/claude-logo.png";
import CodexLogo from "@/assets/codex-logo.png";
import GeminiLogo from "@/assets/gemini-logo.png";
import DuckLogo from "@/assets/duck-logo.png";

// Import statistics components
import { QuotaCard } from "@/components/QuotaCard";
import { UsageChart } from "@/components/UsageChart";
import { TodayStatsCard } from "@/components/TodayStatsCard";

interface ToolWithUpdate extends ToolStatus {
  hasUpdate?: boolean;
  latestVersion?: string;
}

// 可拖拽的配置项组件
interface ProfileItemProps {
  profile: string;
  toolId: string;
  switching: boolean;
  deleting: boolean;
  onSwitch: (toolId: string, profile: string) => void;
  onDelete: (toolId: string, profile: string) => void;
}

function SortableProfileItem({ profile, toolId, switching, deleting, onSwitch, onDelete }: ProfileItemProps) {
  const {
    attributes,
    listeners,
    setNodeRef,
    transform,
    transition,
    isDragging,
  } = useSortable({ id: profile });

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
  const [activeTab, setActiveTab] = useState("dashboard");
  const [tools, setTools] = useState<ToolWithUpdate[]>([]);
  const [loading, setLoading] = useState(true);
  const [installing, setInstalling] = useState<string | null>(null);
  const [updating, setUpdating] = useState<string | null>(null);
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updateCheckMessage, setUpdateCheckMessage] = useState<{ type: 'success' | 'error', text: string } | null>(null);
  const [configuring, setConfiguring] = useState(false);
  const [switching, setSwitching] = useState(false);
  const [deleting, setDeleting] = useState(false);

  // Ref to store timeout ID for cleanup
  const updateMessageTimeoutRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const lastFetchTimeRef = useRef<number>(0);

  // API 配置表单状态
  const [selectedTool, setSelectedTool] = useState<string>("");
  const [provider, setProvider] = useState<string>("duckcoding");
  const [apiKey, setApiKey] = useState<string>("");
  const [baseUrl, setBaseUrl] = useState<string>("");
  const [profileName, setProfileName] = useState<string>("");

  // 配置切换状态
  const [profiles, setProfiles] = useState<Record<string, string[]>>({});
  const [selectedProfile, setSelectedProfile] = useState<Record<string, string>>({});
  const [activeConfigs, setActiveConfigs] = useState<Record<string, ActiveConfig>>({});
  const [selectedSwitchTab, setSelectedSwitchTab] = useState<string>("");  // 切换配置页面的Tab选择

  // 全局配置状态
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [globalConfig, setGlobalConfig] = useState<GlobalConfig | null>(null);
  const [userId, setUserId] = useState("");
  const [systemToken, setSystemToken] = useState("");
  const [savingSettings, setSavingSettings] = useState(false);
  const [generatingKey, setGeneratingKey] = useState(false);

  // 统计数据状态
  const [usageStats, setUsageStats] = useState<UsageStatsResult | null>(null);
  const [userQuota, setUserQuota] = useState<UserQuotaResult | null>(null);
  const [loadingStats, setLoadingStats] = useState(false);

  // Node环境检测状态
  const [nodeEnv, setNodeEnv] = useState<NodeEnvironment | null>(null);
  const [installMethods, setInstallMethods] = useState<Record<string, string>>({
    "claude-code": "official",
    "codex": "official",
    "gemini-cli": "npm",
  });

  const logoMap: Record<string, string> = {
    "claude-code": ClaudeLogo,
    "codex": CodexLogo,
    "gemini-cli": GeminiLogo,
  };

  const descriptionMap: Record<string, string> = {
    "claude-code": "Anthropic 官方 CLI - AI 代码助手",
    "codex": "OpenAI 代码助手 - GPT-5 Codex",
    "gemini-cli": "Google Gemini 命令行工具",
  };

  const groupNameMap: Record<string, string> = {
    "claude-code": "Claude Code 专用分组",
    "codex": "CodeX 专用分组",
    "gemini-cli": "Gemini CLI 专用分组",
  };

  // 拖拽排序相关 - Sensors
  const sensors = useSensors(
    useSensor(PointerSensor),
    useSensor(KeyboardSensor, {
      coordinateGetter: sortableKeyboardCoordinates,
    })
  );

  // 加载配置文件排序
  const loadProfileOrder = (toolId: string): string[] => {
    try {
      const key = `profile-order-${toolId}`;
      const saved = localStorage.getItem(key);
      return saved ? JSON.parse(saved) : [];
    } catch (error) {
      console.error("Failed to load profile order:", error);
      return [];
    }
  };

  // 保存配置文件排序
  const saveProfileOrder = (toolId: string, order: string[]) => {
    try {
      const key = `profile-order-${toolId}`;
      localStorage.setItem(key, JSON.stringify(order));
    } catch (error) {
      console.error("Failed to save profile order:", error);
    }
  };

  // 应用已保存的排序
  const applySavedOrder = (toolId: string, profiles: string[]): string[] => {
    const savedOrder = loadProfileOrder(toolId);
    if (savedOrder.length === 0) return profiles;

    // 按照保存的顺序排列
    const ordered: string[] = [];
    const remaining = [...profiles];

    savedOrder.forEach(name => {
      const index = remaining.indexOf(name);
      if (index !== -1) {
        ordered.push(name);
        remaining.splice(index, 1);
      }
    });

    // 将新增的配置文件添加到末尾
    return [...ordered, ...remaining];
  };

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

  // 清理版本号显示
  const cleanVersion = (version: string | null): string => {
    if (!version) return "未知";
    const match = version.match(/(\d+\.\d+\.\d+)/);
    return match ? match[1] : version;
  };

  // 打开外部链接
  const openExternalLink = async (url: string) => {
    try {
      // 动态导入 shell 插件
      const { open } = await import("@tauri-apps/plugin-shell");
      await open(url);
      console.log("链接已在浏览器中打开:", url);
    } catch (error) {
      console.error("打开链接失败:", error);
      // 降级方案：在浏览器环境中使用 window.open
      if (typeof window !== 'undefined') {
        window.open(url, '_blank');
      }
    }
  };

  // 脱敏显示 API Key
  const maskApiKey = (key: string): string => {
    if (!key) return "";
    if (key.length <= 10) {
      return "*".repeat(key.length);
    }
    const start = key.substring(0, 4);
    const end = key.substring(key.length - 4);
    const middle = "*".repeat(Math.min(key.length - 8, 20)); // 最多显示20个星号
    return `${start}${middle}${end}`;
  };

  // 切换到配置页面并选择特定工具
  const switchToConfig = (toolId?: string) => {
    setActiveTab("config");
    if (toolId) {
      setSelectedTool(toolId);
    }
  };

  // 获取工具可用的安装方式
  const getAvailableInstallMethods = (toolId: string): Array<{value: string, label: string, disabled?: boolean}> => {
    const isMac = navigator.userAgent.includes('Mac');

    if (toolId === "claude-code") {
      return [
        { value: "official", label: "官方脚本 (推荐)" },
        { value: "npm", label: "npm 安装", disabled: !nodeEnv?.npm_available }
      ];
    } else if (toolId === "codex") {
      const methods = [
        { value: "official", label: "官方安装 (推荐)" },
        { value: "npm", label: "npm 安装", disabled: !nodeEnv?.npm_available }
      ];
      if (isMac) {
        methods.splice(1, 0, { value: "brew", label: "Homebrew" });
      }
      return methods;
    } else if (toolId === "gemini-cli") {
      return [
        { value: "npm", label: "npm 安装 (推荐)", disabled: !nodeEnv?.npm_available }
      ];
    }
    return [];
  };

  useEffect(() => {
    loadToolStatus();
    checkNodeEnvironment().then((env: NodeEnvironment) => {
      setNodeEnv(env);
    }).catch((error: unknown) => {
      console.error("Failed to check node environment:", error);
    });
  }, []);

  // Cleanup timeout on unmount
  useEffect(() => {
    return () => {
      if (updateMessageTimeoutRef.current) {
        clearTimeout(updateMessageTimeoutRef.current);
      }
    };
  }, []);

  // 当切换到配置页面时，自动选择第一个已安装的工具并加载配置列表
  useEffect(() => {
    if (activeTab === "config" || activeTab === "switch") {
      const installedTools = tools.filter(t => t.installed);
      if (!selectedTool && installedTools.length > 0) {
        setSelectedTool(installedTools[0].id);
      }
      if (activeTab === "switch") {
        loadAllProfiles();
      }
    }
  }, [activeTab, tools, selectedTool]);

  const loadToolStatus = async () => {
    try {
      setLoading(true);
      const status = await checkInstallations();
      setTools(status);

      // 自动检查已安装工具的更新（批量 API 会自动处理所有工具）
      checkUpdatesForInstalledTools();
    } catch (error) {
      console.error("Failed to check installations:", error);
      setTools([
        { id: "claude-code", name: "Claude Code", installed: false, version: null },
        { id: "codex", name: "CodeX", installed: false, version: null },
        { id: "gemini-cli", name: "Gemini CLI", installed: false, version: null },
      ]);
    } finally {
      setLoading(false);
    }
  };

  // 自动检查已安装工具的更新（后台静默检查）
  const checkUpdatesForInstalledTools = async () => {
    try {
      // 使用批量 API 一次性检查所有工具
      const results = await checkAllUpdates();

      // 更新工具状态，添加更新信息（仅限已安装的工具）
      setTools(prevTools => prevTools.map(tool => {
        const updateInfo = results.find(r => r.tool_id === tool.id);
        if (tool.installed && updateInfo && updateInfo.success) {
          return {
            ...tool,
            hasUpdate: updateInfo.has_update,
            latestVersion: updateInfo.latest_version || undefined
          };
        }
        return tool;
      }));
    } catch (error) {
      console.error("Failed to check updates:", error);
    }
  };

  const loadAllProfiles = async () => {
    const installedTools = tools.filter(t => t.installed);
    const profileData: Record<string, string[]> = {};
    const configData: Record<string, ActiveConfig> = {};

    for (const tool of installedTools) {
      try {
        const toolProfiles = await listProfiles(tool.id);
        // 应用保存的排序
        profileData[tool.id] = applySavedOrder(tool.id, toolProfiles);
      } catch (error) {
        console.error("Failed to load profiles for " + tool.id, error);
        profileData[tool.id] = [];
      }

      try {
        const activeConfig = await getActiveConfig(tool.id);
        configData[tool.id] = activeConfig;
      } catch (error) {
        console.error("Failed to load active config for " + tool.id, error);
        configData[tool.id] = { api_key: "未配置", base_url: "未配置" };
      }
    }

    setProfiles(profileData);
    setActiveConfigs(configData);

    // 设置默认选中的Tab（第一个已安装的工具）
    if (installedTools.length > 0 && !selectedSwitchTab) {
      setSelectedSwitchTab(installedTools[0].id);
    }
  };

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
      console.error("Failed to load global config:", error);
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
        getUsageStats().catch(err => {
          console.error("Failed to load usage stats:", err);
          return null;
        }),
        getUserQuota().catch(err => {
          console.error("Failed to load user quota:", err);
          return null;
        })
      ]);

      if (statsResult) {
        setUsageStats(statsResult);
      }
      if (quotaResult) {
        setUserQuota(quotaResult);
      }
    } catch (error) {
      console.error("Failed to load statistics:", error);
    } finally {
      setLoadingStats(false);
    }
  }, [globalConfig]);

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

  // 保存全局设置
  const handleSaveSettings = async () => {
    // 验证用户输入
    const trimmedUserId = userId.trim();
    const trimmedToken = systemToken.trim();

    if (!trimmedUserId || !trimmedToken) {
      alert("请填写用户ID和系统访问令牌");
      return;
    }

    // 验证用户ID格式（应该是纯数字）
    if (!/^\d+$/.test(trimmedUserId)) {
      alert("用户ID格式错误，应该是纯数字（例如：123456）");
      return;
    }

    // 验证系统访问令牌格式（最少5个字符）
    if (trimmedToken.length < 5) {
      alert("系统访问令牌格式错误，长度至少需要5个字符");
      return;
    }

    try {
      setSavingSettings(true);
      await saveGlobalConfig(trimmedUserId, trimmedToken);
      setGlobalConfig({ user_id: trimmedUserId, system_token: trimmedToken });
      alert("全局设置保存成功");
      setSettingsOpen(false);
    } catch (error) {
      console.error("Failed to save settings:", error);
      alert("保存设置失败: " + error);
    } finally {
      setSavingSettings(false);
    }
  };

  // 一键生成API Key
  const handleGenerateApiKey = async () => {
    if (!selectedTool) {
      alert("请先选择要配置的工具");
      return;
    }

    if (!globalConfig?.user_id || !globalConfig?.system_token) {
      alert("请先在全局设置中配置用户ID和系统访问令牌");
      setSettingsOpen(true);
      return;
    }

    try {
      setGeneratingKey(true);
      const result = await generateApiKeyForTool(selectedTool);

      if (result.success && result.api_key) {
        setApiKey(result.api_key);
        alert("API Key生成成功！已自动填入配置框");
      } else {
        alert("生成失败: " + result.message);
      }
    } catch (error) {
      console.error("Failed to generate API key:", error);
      alert("生成API Key失败: " + error);
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
        const updateInfo = results.find(r => r.tool_id === tool.id);
        if (updateInfo && updateInfo.success && tool.installed) {
          return {
            ...tool,
            hasUpdate: updateInfo.has_update,
            latestVersion: updateInfo.latest_version || undefined,
          };
        }
        return tool;
      });
      setTools(updatedTools);

      // Count updates available
      const updatesAvailable = updatedTools.filter(t => t.hasUpdate).length;
      if (updatesAvailable > 0) {
        setUpdateCheckMessage({
          type: 'success',
          text: `发现 ${updatesAvailable} 个工具有可用更新！`
        });
      } else {
        setUpdateCheckMessage({
          type: 'success',
          text: '所有工具均已是最新版本'
        });
      }

      // Auto-hide message after 5 seconds
      updateMessageTimeoutRef.current = setTimeout(() => {
        setUpdateCheckMessage(null);
        updateMessageTimeoutRef.current = null;
      }, 5000);
    } catch (error) {
      console.error("Failed to check for updates:", error);
      setUpdateCheckMessage({
        type: 'error',
        text: '检查更新失败，请重试'
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
      const method = installMethods[toolId] || "official";
      console.log(`Installing ${toolId} using method: ${method}`);
      await installTool(toolId, method);
      await loadToolStatus();
    } catch (error) {
      console.error("Failed to install " + toolId, error);
      alert("安装失败: " + error);
    } finally {
      setInstalling(null);
    }
  };

  const handleUpdate = async (toolId: string) => {
    // 防止重复点击
    if (updating) {
      alert("已有更新任务正在进行，请等待完成后再试");
      return;
    }

    try {
      setUpdating(toolId);
      await updateTool(toolId);
      await loadToolStatus();
      alert("更新成功！");
    } catch (error) {
      console.error("Failed to update " + toolId, error);
      const errorMsg = String(error);

      // 检查是否是 Homebrew 锁定错误
      if (errorMsg.includes("already locked") || errorMsg.includes("Please wait for it to finish")) {
        alert("Homebrew 正在处理其他任务，请稍后重试。\n\n如需强制解锁，请在终端运行：\npkill -9 brew && rm -rf ~/Library/Caches/Homebrew/Locks/*");
      } else {
        alert("更新失败: " + error);
      }
    } finally {
      setUpdating(null);
    }
  };

  const handleConfigureApi = async () => {
    if (!selectedTool || !apiKey) {
      alert("❌ 请填写必填项\n\n" + (!selectedTool ? "• 请选择工具\n" : "") + (!apiKey ? "• 请输入 API Key" : ""));
      return;
    }

    try {
      setConfiguring(true);
      await configureApi(
        selectedTool,
        provider,
        apiKey,
        provider === "custom" ? baseUrl : undefined,
        profileName || undefined
      );

      // 清空表单
      setApiKey("");
      setBaseUrl("");
      setProfileName("");

      // 重新加载配置列表
      await loadAllProfiles();

      // 弹窗提示成功
      const toolName = selectedTool === 'claude-code' ? 'Claude Code' : selectedTool === 'codex' ? 'CodeX' : 'Gemini CLI';
      alert(`✅ ${toolName} 配置保存成功！${profileName ? `\n\n配置名称: ${profileName}` : ''}`);
    } catch (error) {
      console.error("Failed to configure API:", error);
      alert("❌ 配置失败\n\n" + error);
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
        console.error("Failed to reload active config", error);
      }

      alert("✅ 配置切换成功！\n\n请重启相关 CLI 工具以使新配置生效。");
    } catch (error) {
      console.error("Failed to switch profile:", error);
      alert("❌ 切换失败\n\n" + error);
    } finally {
      setSwitching(false);
    }
  };

  const handleDeleteProfile = async (toolId: string, profile: string) => {
    // 确认删除
    if (!confirm(`确定要删除配置 "${profile}" 吗？\n\n此操作不可恢复。`)) {
      return;
    }

    try {
      setDeleting(true);
      await deleteProfile(toolId, profile);

      // 更新profiles列表，移除已删除的配置
      setProfiles(prevProfiles => {
        const updatedProfiles = { ...prevProfiles };
        if (updatedProfiles[toolId]) {
          updatedProfiles[toolId] = updatedProfiles[toolId].filter(p => p !== profile);
        }
        return updatedProfiles;
      });

      alert("✅ 配置删除成功！");
    } catch (error) {
      console.error("Failed to delete profile:", error);
      alert("❌ 删除失败\n\n" + error);
    } finally {
      setDeleting(false);
    }
  };

  const installedTools = tools.filter(t => t.installed);

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
            variant={activeTab === "dashboard" ? "default" : "ghost"}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab("dashboard")}
          >
            <LayoutDashboard className="mr-2 h-4 w-4" />仪表板
          </Button>
          <Button
            variant={activeTab === "install" ? "default" : "ghost"}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab("install")}
          >
            <Package className="mr-2 h-4 w-4" />安装工具
          </Button>
          <Button
            variant={activeTab === "config" ? "default" : "ghost"}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab("config")}
            disabled={installedTools.length === 0}
          >
            <Key className="mr-2 h-4 w-4" />配置 API
          </Button>
          <Button
            variant={activeTab === "switch" ? "default" : "ghost"}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab("switch")}
            disabled={installedTools.length === 0}
          >
            <ArrowRightLeft className="mr-2 h-4 w-4" />切换配置
          </Button>
          <Button
            variant={activeTab === "statistics" ? "default" : "ghost"}
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setActiveTab("statistics")}
          >
            <BarChart3 className="mr-2 h-4 w-4" />用量统计
          </Button>
          <Separator className="my-3" />
          <Button
            variant="ghost"
            className="w-full justify-start transition-all hover:scale-105"
            onClick={() => setSettingsOpen(true)}
          >
            <SettingsIcon className="mr-2 h-4 w-4" />全局设置
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
            {activeTab === "dashboard" && (
              <div>
                <div className="mb-8 flex items-center justify-between">
                  <div>
                    <h2 className="text-4xl font-semibold tracking-tight mb-2 bg-gradient-to-r from-slate-900 to-slate-600 dark:from-slate-100 dark:to-slate-400 bg-clip-text text-transparent">仪表板</h2>
                    <p className="text-muted-foreground">管理您的 AI 开发工具</p>
                  </div>
                  <div className="flex flex-col items-end gap-2">
                    <Button
                      onClick={checkForUpdates}
                      disabled={checkingUpdates || tools.every(t => !t.installed)}
                      variant="outline"
                      className="shadow-md hover:shadow-lg transition-all"
                    >
                      {checkingUpdates ? <><Loader2 className="mr-2 h-4 w-4 animate-spin" />检查中...</> : <><RefreshCw className="mr-2 h-4 w-4" />检查更新</>}
                    </Button>
                    {updateCheckMessage && (
                      <div className={`flex items-center gap-2 text-sm px-3 py-1.5 rounded-md animate-in fade-in slide-in-from-top-2 ${
                        updateCheckMessage.type === 'success'
                          ? 'bg-green-50 text-green-700 border border-green-200 dark:bg-green-900/20 dark:text-green-400 dark:border-green-800'
                          : 'bg-red-50 text-red-700 border border-red-200 dark:bg-red-900/20 dark:text-red-400 dark:border-red-800'
                      }`}>
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
                              <img src={logoMap[tool.id]} alt={tool.name} className="w-16 h-16 drop-shadow-xl" />
                            </div>
                            <div className="flex flex-col gap-2 items-end">
                              {tool.installed ? (
                                <>
                                  <Badge variant="default" className="gap-1.5 shadow-md px-3 py-1">
                                    <CheckCircle2 className="h-3.5 w-3.5" />已安装
                                  </Badge>
                                  {tool.hasUpdate && (
                                    <Badge variant="destructive" className="gap-1.5 shadow-md animate-pulse px-3 py-1">
                                      <AlertCircle className="h-3.5 w-3.5" />需要更新
                                    </Badge>
                                  )}
                                  {tool.installed && tool.hasUpdate === false && (
                                    <Badge variant="outline" className="gap-1.5 text-green-600 border-green-600 shadow-sm px-3 py-1">
                                      <CheckCircle2 className="h-3.5 w-3.5" />最新版本
                                    </Badge>
                                  )}
                                </>
                              ) : (
                                <Badge variant="secondary" className="gap-1.5 shadow-md px-3 py-1">
                                  <XCircle className="h-3.5 w-3.5" />未安装
                                </Badge>
                              )}
                            </div>
                          </div>
                          <div className="space-y-2">
                            <CardTitle className="text-2xl font-semibold">{tool.name}</CardTitle>
                            <CardDescription className="text-sm leading-relaxed">{descriptionMap[tool.id]}</CardDescription>
                          </div>
                        </CardHeader>
                        <CardContent className="pb-4">
                          {tool.installed && tool.version ? (
                            <div className="space-y-2 bg-gradient-to-br from-slate-50 to-slate-100 dark:from-slate-800/80 dark:to-slate-900/80 p-4 rounded-xl border border-slate-200 dark:border-slate-700">
                              <div className="flex items-center justify-between">
                                <span className="text-sm font-semibold text-slate-600 dark:text-slate-400">当前版本</span>
                                <span className="font-mono text-sm font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-3 py-1 rounded-lg">
                                  {cleanVersion(tool.version)}
                                </span>
                              </div>
                              {tool.hasUpdate && tool.latestVersion && (
                                <div className="flex items-center justify-between pt-1 border-t border-slate-200 dark:border-slate-700">
                                  <span className="text-sm font-semibold text-slate-600 dark:text-slate-400">最新版本</span>
                                  <span className="font-mono text-sm font-semibold text-green-600 dark:text-green-400 bg-green-50 dark:bg-green-950 px-3 py-1 rounded-lg">
                                    {cleanVersion(tool.latestVersion)}
                                  </span>
                                </div>
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
                                <Key className="mr-2 h-4 w-4" />配置
                              </Button>
                              {tool.hasUpdate ? (
                                <Button
                                  size="sm"
                                  className="flex-1 shadow-md hover:shadow-lg transition-all bg-gradient-to-r from-orange-500 to-red-500 hover:from-orange-600 hover:to-red-600 h-10"
                                  onClick={() => handleUpdate(tool.id)}
                                  disabled={updating === tool.id}
                                >
                                  {updating === tool.id ? <><Loader2 className="mr-2 h-4 w-4 animate-spin" />更新中</> : <><RefreshCw className="mr-2 h-4 w-4" />更新</>}
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
                              {installing === tool.id ? <><Loader2 className="mr-2 h-4 w-4 animate-spin" />安装中...</> : <><Package className="mr-2 h-4 w-4" />安装</>}
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

            {activeTab === "statistics" && (
              <div>
                <div className="mb-6">
                  <h2 className="text-2xl font-semibold mb-1">用量统计</h2>
                  <p className="text-sm text-muted-foreground">查看您的 DuckCoding API 使用情况和消费记录</p>
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
                          <><Loader2 className="mr-2 h-4 w-4 animate-spin" />加载中...</>
                        ) : (
                          <><RefreshCw className="mr-2 h-4 w-4" />刷新数据</>
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

            {activeTab === "install" && (
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
                      <Card
                        key={tool.id}
                        className="shadow-sm border"
                      >
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
                                      <CheckCircle2 className="h-3 w-3" />已安装
                                    </Badge>
                                  )}
                                </div>
                                <p className="text-sm text-muted-foreground leading-relaxed">{descriptionMap[tool.id]}</p>
                                {tool.installed && tool.version && (
                                  <div className="flex items-center gap-2 mt-3">
                                    <span className="text-xs font-semibold text-slate-600 dark:text-slate-400">当前版本:</span>
                                    <span className="font-mono text-xs font-semibold text-blue-600 dark:text-blue-400 bg-blue-50 dark:bg-blue-950 px-2.5 py-1 rounded-lg shadow-sm">
                                      {cleanVersion(tool.version)}
                                    </span>
                                  </div>
                                )}
                              </div>
                            </div>
                            <div className="flex flex-col gap-3 items-end">
                              {!tool.installed && (
                                <div className="w-48">
                                  <Label htmlFor={`method-${tool.id}`} className="text-xs mb-1.5 block">安装方式</Label>
                                  <Select
                                    value={installMethods[tool.id]}
                                    onValueChange={(value) => setInstallMethods({ ...installMethods, [tool.id]: value })}
                                  >
                                    <SelectTrigger id={`method-${tool.id}`} className="shadow-sm h-9 text-sm">
                                      <SelectValue />
                                    </SelectTrigger>
                                    <SelectContent>
                                      {getAvailableInstallMethods(tool.id).map(method => (
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
                                  <><Loader2 className="mr-2 h-5 w-5 animate-spin" />安装中...</>
                                ) : tool.installed ? (
                                  <><CheckCircle2 className="mr-2 h-5 w-5" />已安装</>
                                ) : (
                                  <><Package className="mr-2 h-5 w-5" />安装工具</>
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

            {activeTab === "config" && (
              <div>
                <div className="mb-6">
                  <h2 className="text-2xl font-semibold mb-1">配置 API</h2>
                  <p className="text-sm text-muted-foreground">配置 DuckCoding API 或自定义 API 端点</p>
                </div>

                {installedTools.length > 0 ? (
                  <div className="grid gap-4">
                    {/* 重要提示 - 移到顶部 */}
                    <div className="p-4 bg-gradient-to-r from-amber-50 to-orange-50 dark:from-amber-950 dark:to-orange-950 rounded-lg border border-amber-200 dark:border-amber-800">
                      <div className="flex items-start gap-3">
                        <Info className="h-5 w-5 text-amber-600 dark:text-amber-400 flex-shrink-0 mt-0.5" />
                        <div className="space-y-2">
                          <h4 className="font-semibold text-amber-900 dark:text-amber-100">重要提示</h4>
                          <div className="text-sm text-amber-800 dark:text-amber-200 space-y-2">
                            <div>
                              <p className="font-semibold mb-1">DuckCoding API Key 分组:</p>
                              <ul className="list-disc list-inside space-y-1 ml-2">
                                {selectedTool && groupNameMap[selectedTool] && (
                                  <li>当前工具需要使用 <span className="font-mono bg-amber-100 dark:bg-amber-900 px-1.5 py-0.5 rounded">{groupNameMap[selectedTool]}</span> 的 API Key</li>
                                )}
                                <li>每个工具必须使用其专用分组的 API Key</li>
                                <li>API Key 不能混用</li>
                              </ul>
                            </div>
                            <div>
                              <p className="font-semibold mb-1">获取 API Key:</p>
                              <button
                                onClick={() => openExternalLink("https://duckcoding.com/console/token")}
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
                                {installedTools.map(tool => (
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
                                  <><Loader2 className="mr-2 h-4 w-4 animate-spin" />生成中...</>
                                ) : (
                                  <><Sparkles className="mr-2 h-4 w-4" />一键生成</>
                                )}
                              </Button>
                            </div>
                            <p className="text-xs text-muted-foreground">点击"一键生成"可自动创建 DuckCoding API Key（需先配置全局设置）</p>
                          </div>

                          {provider === "duckcoding" && (
                            <div className="rounded-lg bg-blue-50 dark:bg-blue-950/30 border border-blue-200 dark:border-blue-800 p-4">
                              <div className="flex items-start gap-3">
                                <Info className="h-5 w-5 text-blue-600 dark:text-blue-400 flex-shrink-0 mt-0.5" />
                                <div className="space-y-2">
                                  <p className="text-sm font-medium text-blue-900 dark:text-blue-100">
                                    DuckCoding 默认配置
                                  </p>
                                  <div className="text-xs text-blue-700 dark:text-blue-300 space-y-1">
                                    <p>• Base URL: <code className="bg-white/50 dark:bg-slate-900/50 px-1.5 py-0.5 rounded">https://jp.duckcoding.com</code></p>
                                    <p>• 无需手动填写 Base URL，将自动使用默认端点</p>
                                    <p>• 切换配置后，请<strong>重启相关 CLI</strong> 以使新配置生效</p>
                                  </div>
                                </div>
                              </div>
                            </div>
                          )}

                          {provider === "custom" && (
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
                          onClick={() => { setApiKey(""); setBaseUrl(""); setProfileName(""); }}
                          className="shadow-sm"
                        >
                          清空
                        </Button>
                        <Button
                          onClick={handleConfigureApi}
                          disabled={configuring || !selectedTool || !apiKey}
                          className="shadow-sm hover:shadow-md transition-all"
                        >
                          {configuring ? <><Loader2 className="mr-2 h-4 w-4 animate-spin" />保存中...</> : <><Save className="mr-2 h-4 w-4" />保存配置</>}
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
                        onClick={() => setActiveTab("install")}
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

            {activeTab === "switch" && (
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
                        切换配置后，如果工具正在运行，<strong>需要重启对应的工具</strong>才能使新配置生效。
                      </p>
                    </div>
                  </div>
                </div>

                {installedTools.length > 0 ? (
                  <Tabs value={selectedSwitchTab} onValueChange={setSelectedSwitchTab}>
                    <TabsList className="grid w-full grid-cols-3 mb-6">
                      {installedTools.map(tool => (
                        <TabsTrigger key={tool.id} value={tool.id} className="gap-2">
                          <img src={logoMap[tool.id]} alt={tool.name} className="w-4 h-4" />
                          {tool.name}
                        </TabsTrigger>
                      ))}
                    </TabsList>

                    {installedTools.map(tool => {
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
                                    <h4 className="font-semibold text-blue-900 dark:text-blue-100">当前生效配置</h4>
                                  </div>
                                  <div className="space-y-2 text-sm">
                                    {activeConfig.profile_name && (
                                      <div className="flex items-start gap-2">
                                        <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">配置名称:</span>
                                        <span className="font-semibold text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded">
                                          {activeConfig.profile_name}
                                        </span>
                                      </div>
                                    )}
                                    <div className="flex items-start gap-2">
                                      <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">API Key:</span>
                                      <span className="font-mono text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50 px-2 py-0.5 rounded">
                                        {maskApiKey(activeConfig.api_key)}
                                      </span>
                                    </div>
                                    <div className="flex items-start gap-2">
                                      <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">Base URL:</span>
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
                                        {toolProfiles.map(profile => (
                                          <SortableProfileItem
                                            key={profile}
                                            profile={profile}
                                            toolId={tool.id}
                                            switching={switching}
                                            deleting={deleting}
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
                                  <p className="text-sm text-muted-foreground">在"配置 API"页面保存配置时填写名称即可创建多个配置</p>
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
                        onClick={() => setActiveTab("install")}
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
                  <p>1. 访问 <button onClick={() => openExternalLink("https://duckcoding.com/console/personal")} className="underline hover:text-blue-600 cursor-pointer bg-transparent border-0 p-0 inline">个人中心</button> 查看用户ID</p>
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
                <><Loader2 className="mr-2 h-4 w-4 animate-spin" />保存中...</>
              ) : (
                <><Save className="mr-2 h-4 w-4" />保存设置</>
              )}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}

export default App;
