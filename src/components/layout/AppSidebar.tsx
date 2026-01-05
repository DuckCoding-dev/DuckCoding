import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
// 预留
// import { Avatar, AvatarFallback, AvatarImage } from '@/components/ui/avatar';
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
  //预留
  // User,
} from 'lucide-react';
import DuckLogo from '@/assets/duck-logo.png';
import { useToast } from '@/hooks/use-toast';
import { useTheme } from '@/hooks/useThemeHook';
import { useState, useEffect } from 'react';
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
  DropdownMenuSeparator,
  DropdownMenuLabel,
} from '@/components/ui/dropdown-menu';
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip';

interface AppSidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  restrictNavigation?: boolean;
  allowedPage?: string; // 新增：允许访问的页面（引导模式下）
}

// 导航项配置
const navigationItems = [
  { id: 'dashboard', label: '仪表板', icon: LayoutDashboard },
  { id: 'tool-management', label: '工具管理', icon: Wrench },
  { id: 'profile-management', label: '配置管理', icon: Settings2 },
  { id: 'balance', label: '余额查询', icon: Wallet },
  { id: 'transparent-proxy', label: '透明代理', icon: Radio },
];

const secondaryItems = [
  { id: 'provider-management', label: '供应商', icon: Building2 },
  { id: 'help', label: '帮助', icon: HelpCircle },
  { id: 'settings', label: '设置', icon: SettingsIcon },
];

export function AppSidebar({
  activeTab,
  onTabChange,
  restrictNavigation,
  allowedPage,
}: AppSidebarProps) {
  const { toast } = useToast();
  const { theme, actualTheme, setTheme } = useTheme();

  const [isCollapsed, setIsCollapsed] = useState(() => {
    const stored = localStorage.getItem('duckcoding-sidebar-collapsed');
    return stored === 'true';
  });

  useEffect(() => {
    localStorage.setItem('duckcoding-sidebar-collapsed', String(isCollapsed));
  }, [isCollapsed]);

  const handleTabChange = (tab: string) => {
    if (restrictNavigation) {
      // 只允许访问 allowedPage
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

  // 导航按钮组件
  const NavButton = ({ item }: { item: (typeof navigationItems)[0] }) => {
    const Icon = item.icon;
    const isActive = activeTab === item.id;

    const button = (
      <Button
        variant={isActive ? 'default' : 'ghost'}
        size={isCollapsed ? 'icon' : 'default'}
        className={`w-full ${isCollapsed ? 'h-11 w-11' : 'justify-start h-11'} transition-all group relative ${
          isActive
            ? 'shadow-lg shadow-primary/20'
            : 'hover:bg-accent hover:shadow-md hover:scale-[1.02]'
        }`}
        onClick={() => handleTabChange(item.id)}
        disabled={restrictNavigation && allowedPage ? item.id !== allowedPage : false}
      >
        {isActive && !isCollapsed && (
          <div className="absolute left-0 top-0 bottom-0 w-1 bg-primary rounded-r-full" />
        )}
        <Icon
          className={`h-5 w-5 ${isCollapsed ? '' : 'mr-3'} transition-transform group-hover:scale-110`}
        />
        {!isCollapsed && <span className="text-sm font-medium">{item.label}</span>}
      </Button>
    );

    if (isCollapsed) {
      return (
        <Tooltip delayDuration={300}>
          <TooltipTrigger asChild>{button}</TooltipTrigger>
          <TooltipContent side="right" className="font-medium">
            <p>{item.label}</p>
          </TooltipContent>
        </Tooltip>
      );
    }

    return button;
  };

  return (
    <TooltipProvider>
      <aside
        className={`${
          isCollapsed ? 'w-[72px]' : 'w-64'
        } border-r bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60 shadow-xl transition-all duration-300 ease-in-out flex flex-col`}
      >
        {/* Logo 区域 */}
        <div
          className={`p-4 flex items-center ${isCollapsed ? 'justify-center' : 'gap-3'} min-h-[80px]`}
        >
          {isCollapsed ? (
            <Tooltip delayDuration={300}>
              <TooltipTrigger asChild>
                <div className="relative group cursor-pointer">
                  <div className="absolute inset-0 bg-primary/20 rounded-lg blur-xl group-hover:blur-2xl transition-all opacity-0 group-hover:opacity-100" />
                  <img
                    src={DuckLogo}
                    alt="DuckCoding"
                    className="relative w-11 h-11 drop-shadow-2xl group-hover:scale-110 transition-transform"
                  />
                </div>
              </TooltipTrigger>
              <TooltipContent side="right" className="font-semibold">
                <p>DuckCoding</p>
                <p className="text-xs text-muted-foreground font-normal">一键配置中心</p>
              </TooltipContent>
            </Tooltip>
          ) : (
            <>
              <div className="relative group">
                <div className="absolute inset-0 bg-primary/20 rounded-lg blur-xl group-hover:blur-2xl transition-all opacity-0 group-hover:opacity-100" />
                <img
                  src={DuckLogo}
                  alt="DuckCoding"
                  className="relative w-11 h-11 drop-shadow-2xl"
                />
              </div>
              <div className="overflow-hidden">
                <h1 className="text-lg font-bold bg-gradient-to-r from-slate-900 to-slate-700 dark:from-slate-100 dark:to-slate-300 bg-clip-text text-transparent whitespace-nowrap">
                  DuckCoding
                </h1>
                <p className="text-xs text-muted-foreground whitespace-nowrap">一键配置中心</p>
              </div>
            </>
          )}
        </div>

        <Separator className="mb-2" />

        {/* 主导航区域 */}
        <nav
          className={`flex-1 overflow-y-auto py-2 ${isCollapsed ? 'px-3 space-y-2' : 'px-3 space-y-1'}`}
        >
          {navigationItems.map((item) => (
            <NavButton key={item.id} item={item} />
          ))}

          <div className="py-2">
            <Separator />
          </div>

          {secondaryItems.map((item) => (
            <NavButton key={item.id} item={item} />
          ))}
        </nav>

        <Separator />

        {/*预留UI*/}
        {/*/!* 用户信息区域 *!/*/}
        {/*<div className={`${isCollapsed ? 'p-3 flex justify-center' : 'p-3'}`}>*/}
        {/*  {isCollapsed ? (*/}
        {/*    <Tooltip delayDuration={300}>*/}
        {/*      <TooltipTrigger asChild>*/}
        {/*        <div className="relative group cursor-pointer">*/}
        {/*          <Avatar className="h-11 w-11 border-2 border-transparent group-hover:border-primary transition-all group-hover:shadow-lg group-hover:shadow-primary/20">*/}
        {/*            <AvatarImage src="" alt="User" />*/}
        {/*            <AvatarFallback className="bg-gradient-to-br from-blue-500 via-purple-500 to-pink-500 text-white">*/}
        {/*              <User className="h-5 w-5" />*/}
        {/*            </AvatarFallback>*/}
        {/*          </Avatar>*/}
        {/*        </div>*/}
        {/*      </TooltipTrigger>*/}
        {/*      <TooltipContent side="right">*/}
        {/*        <div>*/}
        {/*          <p className="font-medium">访客用户</p>*/}
        {/*          <p className="text-xs text-muted-foreground">点击管理账户</p>*/}
        {/*        </div>*/}
        {/*      </TooltipContent>*/}
        {/*    </Tooltip>*/}
        {/*  ) : (*/}
        {/*      <div className="flex items-center gap-3 p-2 rounded-lg hover:bg-accent cursor-pointer transition-all hover:shadow-md group">*/}
        {/*        <Avatar className="h-10 w-10 border-2 border-transparent group-hover:border-primary transition-all">*/}
        {/*          <AvatarImage src="" alt="User" />*/}
        {/*          <AvatarFallback className="bg-gradient-to-br from-blue-500 via-purple-500 to-pink-500 text-white">*/}
        {/*            <User className="h-5 w-5" />*/}
        {/*          </AvatarFallback>*/}
        {/*        </Avatar>*/}
        {/*        <div className="flex-1 overflow-hidden">*/}
        {/*          <p className="text-sm font-medium truncate">访客用户</p>*/}
        {/*          <p className="text-xs text-muted-foreground truncate">点击管理账户</p>*/}
        {/*        </div>*/}
        {/*      </div>*/}
        {/*  )}*/}
        {/*</div>*/}

        <Separator />

        {/* 底部控制按钮 */}
        <div className={`p-3 flex ${isCollapsed ? 'flex-col gap-2' : 'gap-2 justify-between'}`}>
          {/* 主题切换 */}
          <DropdownMenu>
            <Tooltip delayDuration={300}>
              <TooltipTrigger asChild>
                <DropdownMenuTrigger asChild>
                  <Button
                    variant="ghost"
                    size="icon"
                    className="h-10 w-10 hover:bg-accent hover:shadow-md transition-all hover:scale-105"
                  >
                    <ThemeIcon className="h-5 w-5" />
                  </Button>
                </DropdownMenuTrigger>
              </TooltipTrigger>
              <TooltipContent side="right">
                <p>切换主题</p>
              </TooltipContent>
            </Tooltip>
            <DropdownMenuContent align="end" side="right">
              <DropdownMenuLabel>主题设置</DropdownMenuLabel>
              <DropdownMenuSeparator />
              <DropdownMenuItem onClick={() => setTheme('light')}>
                <Sun className="mr-2 h-4 w-4" />
                浅色模式
                {theme === 'light' && <span className="ml-auto text-primary">✓</span>}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setTheme('dark')}>
                <Moon className="mr-2 h-4 w-4" />
                深色模式
                {theme === 'dark' && <span className="ml-auto text-primary">✓</span>}
              </DropdownMenuItem>
              <DropdownMenuItem onClick={() => setTheme('system')}>
                <Monitor className="mr-2 h-4 w-4" />
                跟随系统
                {theme === 'system' && <span className="ml-auto text-primary">✓</span>}
              </DropdownMenuItem>
            </DropdownMenuContent>
          </DropdownMenu>

          {/* 折叠按钮 */}
          <Tooltip delayDuration={300}>
            <TooltipTrigger asChild>
              <Button
                variant="ghost"
                size="icon"
                className="h-10 w-10 hover:bg-accent hover:shadow-md transition-all hover:scale-105"
                onClick={() => setIsCollapsed(!isCollapsed)}
              >
                {isCollapsed ? (
                  <ChevronsRight className="h-5 w-5" />
                ) : (
                  <ChevronsLeft className="h-5 w-5" />
                )}
              </Button>
            </TooltipTrigger>
            <TooltipContent side="right">
              <p>{isCollapsed ? '展开侧边栏' : '折叠侧边栏'}</p>
            </TooltipContent>
          </Tooltip>
        </div>
      </aside>
    </TooltipProvider>
  );
}
