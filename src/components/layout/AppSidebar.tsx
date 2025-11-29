import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
  LayoutDashboard,
  Package,
  Key,
  ArrowRightLeft,
  BarChart3,
  Wallet,
  Radio,
  Settings as SettingsIcon,
} from 'lucide-react';
import DuckLogo from '@/assets/duck-logo.png';

interface AppSidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
}

export function AppSidebar({ activeTab, onTabChange }: AppSidebarProps) {
  return (
    <aside className="w-64 border-r bg-white/80 dark:bg-slate-900/80 backdrop-blur-xl shadow-xl">
      {/* Logo */}
      <div className="p-6 flex items-center gap-3">
        <img src={DuckLogo} alt="DuckCoding" className="w-12 h-12 drop-shadow-lg" />
        <div>
          <h1 className="text-xl font-bold text-slate-900 dark:text-slate-100">DuckCoding</h1>
          <p className="text-xs text-muted-foreground">一键配置中心</p>
        </div>
      </div>

      <Separator />

      {/* 导航菜单 */}
      <nav className="space-y-1 p-3">
        <Button
          variant={activeTab === 'dashboard' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('dashboard')}
        >
          <LayoutDashboard className="mr-2 h-4 w-4" />
          仪表板
        </Button>

        <Button
          variant={activeTab === 'install' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('install')}
        >
          <Package className="mr-2 h-4 w-4" />
          安装工具
        </Button>

        <Button
          variant={activeTab === 'config' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('config')}
        >
          <Key className="mr-2 h-4 w-4" />
          配置 API
        </Button>

        <Button
          variant={activeTab === 'switch' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('switch')}
        >
          <ArrowRightLeft className="mr-2 h-4 w-4" />
          切换配置
        </Button>

        <Button
          variant={activeTab === 'statistics' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('statistics')}
        >
          <BarChart3 className="mr-2 h-4 w-4" />
          用量统计
        </Button>

        <Button
          variant={activeTab === 'balance' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('balance')}
        >
          <Wallet className="mr-2 h-4 w-4" />
          余额查询
        </Button>

        <Button
          variant={activeTab === 'transparent-proxy' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('transparent-proxy')}
        >
          <Radio className="mr-2 h-4 w-4" />
          透明代理
        </Button>

        <Separator className="my-3" />

        <Button
          variant={activeTab === 'settings' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => onTabChange('settings')}
        >
          <SettingsIcon className="mr-2 h-4 w-4" />
          全局设置
        </Button>
      </nav>
    </aside>
  );
}
