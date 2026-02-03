import { Button } from '@/components/ui/button';
import {
  LayoutDashboard,
  Wrench,
  Settings2,
  Wallet,
  Radio,
  Settings as SettingsIcon,
  HelpCircle,
  Building2,
  ChevronsLeft,
  ChevronsRight,
  Sun,
  Moon,
  Monitor,
  Info,
  BarChart3,
} from 'lucide-react';
import DuckLogo from '@/assets/duck-logo.png';
import { useToast } from '@/hooks/use-toast';
import { useTheme } from '@/hooks/useThemeHook';
import { useState, useEffect, useCallback } from 'react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from '@/components/ui/dropdown-menu';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';
import { cn } from '@/lib/utils';

interface AppSidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  restrictNavigation?: boolean;
  allowedPage?: string;
}

// 导航组配置
const navigationGroups = [
  {
    label: '概览',
    items: [{ id: 'dashboard', label: '仪表板', icon: LayoutDashboard }],
  },
  {
    label: '核心工具',
    items: [
      { id: 'tool-management', label: '工具管理', icon: Wrench },
      { id: 'profile-management', label: '配置方案', icon: Settings2 },
    ],
  },
  {
    label: '网关与监控',
    items: [
      { id: 'transparent-proxy', label: '透明代理', icon: Radio },
      { id: 'provider-management', label: '模型供应商', icon: Building2 },
      { id: 'token-statistics', label: '用量统计', icon: BarChart3 },
      { id: 'balance', label: '余额监控', icon: Wallet },
    ],
  },
  {
    label: '系统',
    items: [
      { id: 'settings', label: '全局设置', icon: SettingsIcon },
      { id: 'help', label: '帮助中心', icon: HelpCircle },
      { id: 'about', label: '关于应用', icon: Info },
    ],
  },
];

// 响应式折叠阈值
const COLLAPSE_BREAKPOINT = 1024;

export function AppSidebar({
  activeTab,
  onTabChange,
  restrictNavigation,
  allowedPage,
}: AppSidebarProps) {
  const { toast } = useToast();
  const { actualTheme, setTheme } = useTheme();

  // 用户是否手动操作过侧边栏（优先级高于自动折叠）
  const [userHasInteracted, setUserHasInteracted] = useState(() => {
    return localStorage.getItem('duckcoding-sidebar-user-interacted') === 'true';
  });

  const [isCollapsed, setIsCollapsed] = useState(() => {
    const stored = localStorage.getItem('duckcoding-sidebar-collapsed');
    // 如果用户没有手动操作过，根据窗口大小决定初始状态
    if (!localStorage.getItem('duckcoding-sidebar-user-interacted')) {
      return typeof window !== 'undefined' && window.innerWidth < COLLAPSE_BREAKPOINT;
    }
    return stored === 'true';
  });

  // 持久化折叠状态
  useEffect(() => {
    localStorage.setItem('duckcoding-sidebar-collapsed', String(isCollapsed));
  }, [isCollapsed]);

  // 响应式自动折叠（仅在用户未手动操作时生效）
  useEffect(() => {
    if (userHasInteracted) return; // 用户手动操作过，不自动折叠

    let timeoutId: ReturnType<typeof setTimeout>;

    const handleResize = () => {
      clearTimeout(timeoutId);
      timeoutId = setTimeout(() => {
        const isSmallScreen = window.innerWidth < COLLAPSE_BREAKPOINT;
        setIsCollapsed(isSmallScreen);
      }, 150);
    };

    window.addEventListener('resize', handleResize);
    return () => {
      clearTimeout(timeoutId);
      window.removeEventListener('resize', handleResize);
    };
  }, [userHasInteracted]);

  // 手动切换折叠状态（标记用户已交互）
  const handleToggleCollapse = useCallback(() => {
    setIsCollapsed((prev) => !prev);
    setUserHasInteracted(true);
    localStorage.setItem('duckcoding-sidebar-user-interacted', 'true');
  }, []);

  const handleTabChange = (tab: string) => {
    if (restrictNavigation) {
      if (allowedPage && tab !== allowedPage) {
        toast({
          title: '请先完成引导',
          description: '完成当前引导步骤后即可访问其他页面',
          variant: 'default',
        });
        return;
      }
    }
    onTabChange(tab);
  };

  const ThemeIcon = actualTheme === 'dark' ? Moon : Sun;

  const NavButton = ({ item }: { item: { id: string; label: string; icon: any } }) => {
    const Icon = item.icon;
    const isActive = activeTab === item.id;
    const isDisabled = restrictNavigation && allowedPage ? item.id !== allowedPage : false;

    return (
      <Tooltip delayDuration={0}>
        <TooltipTrigger asChild>
          <Button
            variant={isActive ? 'secondary' : 'ghost'}
            size={isCollapsed ? 'icon' : 'default'}
            className={cn(
              'w-full transition-all duration-200 relative overflow-hidden group mb-1',
              isCollapsed ? 'h-10 w-10 mx-auto' : 'justify-start h-9 px-3',
              isActive && 'bg-primary/10 text-primary hover:bg-primary/15 font-semibold',
              !isActive && !isDisabled && 'hover:bg-muted/60 hover:text-foreground',
              isDisabled && 'opacity-50 cursor-not-allowed',
            )}
            onClick={() => handleTabChange(item.id)}
            disabled={isDisabled}
          >
            {isActive && !isCollapsed && (
              <div className="absolute left-0 top-1/2 -translate-y-1/2 h-4 w-1 bg-primary rounded-r-full" />
            )}
            <Icon
              className={cn(
                'h-4 w-4 shrink-0 transition-transform duration-300',
                isCollapsed ? '' : 'mr-3',
                isActive && 'text-primary scale-110',
                !isActive && 'group-hover:scale-105',
              )}
            />
            {!isCollapsed && <span className="truncate text-sm">{item.label}</span>}
          </Button>
        </TooltipTrigger>
        {isCollapsed && (
          <TooltipContent
            side="right"
            className="font-medium bg-popover text-popover-foreground border-border shadow-lg"
          >
            <p>{item.label}</p>
          </TooltipContent>
        )}
      </Tooltip>
    );
  };

  return (
    <TooltipProvider>
      <aside
        className={cn(
          'flex flex-col border border-border/50 bg-card/50 backdrop-blur-xl shadow-sm transition-all duration-300 ease-in-out z-50',
          'my-3 ml-3 rounded-2xl',
          isCollapsed ? 'w-[68px]' : 'w-44',
        )}
      >
        {/* Logo Header */}
        <div
          className={cn(
            'flex items-center h-16 px-4 border-b border-border/40',
            isCollapsed ? 'justify-center' : 'gap-3',
          )}
        >
          <div className="relative group cursor-pointer flex-shrink-0">
            <div className="absolute inset-0 bg-primary/20 rounded-full blur-md group-hover:blur-lg transition-all opacity-0 group-hover:opacity-100" />
            <img
              src={DuckLogo}
              alt="DuckCoding"
              className="relative w-8 h-8 object-contain transition-transform group-hover:scale-110"
            />
          </div>
          {!isCollapsed && (
            <div className="flex flex-col overflow-hidden animate-in fade-in slide-in-from-left-2 duration-300">
              <span className="font-bold text-base tracking-tight text-foreground">DuckCoding</span>
              <span className="text-[10px] text-muted-foreground uppercase tracking-wider font-medium">
                Configuration
              </span>
            </div>
          )}
        </div>

        {/* Navigation */}
        <div className="flex-1 overflow-y-auto py-4 px-3 space-y-4 custom-scrollbar">
          {navigationGroups.map((group, index) => (
            <div key={index}>
              {!isCollapsed && (
                <div className="px-3 mb-2">
                  <h4 className="text-xs font-semibold text-muted-foreground/70 uppercase tracking-wider">
                    {group.label}
                  </h4>
                </div>
              )}
              <div className="space-y-0.5">
                {group.items.map((item) => (
                  <NavButton key={item.id} item={item} />
                ))}
              </div>
            </div>
          ))}
        </div>

        {/* Footer */}
        <div className="p-3 border-t border-border/40 bg-muted/20 rounded-b-2xl">
          <div
            className={cn(
              'flex items-center',
              isCollapsed ? 'flex-col gap-3' : 'justify-between px-1',
            )}
          >
            {/* Theme Toggle */}
            <DropdownMenu>
              <Tooltip>
                <TooltipTrigger asChild>
                  <DropdownMenuTrigger asChild>
                    <Button
                      variant="ghost"
                      size="icon"
                      className="h-8 w-8 hover:bg-background/80 hover:text-primary transition-colors"
                    >
                      <ThemeIcon className="h-4 w-4" />
                    </Button>
                  </DropdownMenuTrigger>
                </TooltipTrigger>
                <TooltipContent side="right">切换主题</TooltipContent>
              </Tooltip>
              <DropdownMenuContent
                align="start"
                side={isCollapsed ? 'right' : 'top'}
                className="w-32"
              >
                <DropdownMenuItem onClick={() => setTheme('light')}>
                  <Sun className="mr-2 h-3.5 w-3.5" /> 浅色
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => setTheme('dark')}>
                  <Moon className="mr-2 h-3.5 w-3.5" /> 深色
                </DropdownMenuItem>
                <DropdownMenuItem onClick={() => setTheme('system')}>
                  <Monitor className="mr-2 h-3.5 w-3.5" /> 系统
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>

            {/* Collapse Toggle */}
            <Tooltip>
              <TooltipTrigger asChild>
                <Button
                  variant="ghost"
                  size="icon"
                  className="h-8 w-8 hover:bg-background/80 hover:text-primary transition-colors"
                  onClick={handleToggleCollapse}
                >
                  {isCollapsed ? (
                    <ChevronsRight className="h-4 w-4" />
                  ) : (
                    <ChevronsLeft className="h-4 w-4" />
                  )}
                </Button>
              </TooltipTrigger>
              <TooltipContent side="right">{isCollapsed ? '展开' : '收起'}</TooltipContent>
            </Tooltip>
          </div>
        </div>
      </aside>
    </TooltipProvider>
  );
}
