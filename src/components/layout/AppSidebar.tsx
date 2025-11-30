import { Button } from '@/components/ui/button';
import { Separator } from '@/components/ui/separator';
import {
  LayoutDashboard,
  Wrench,
  Key,
  ArrowRightLeft,
  BarChart3,
  Radio,
  Settings as SettingsIcon,
  HelpCircle,
} from 'lucide-react';
import DuckLogo from '@/assets/duck-logo.png';
import { useToast } from '@/hooks/use-toast';

interface AppSidebarProps {
  activeTab: string;
  onTabChange: (tab: string) => void;
  restrictNavigation?: boolean; // 是否限制导航（引导模式）
}

export function AppSidebar({ activeTab, onTabChange, restrictNavigation }: AppSidebarProps) {
  const { toast } = useToast();

  const handleTabChange = (tab: string) => {
    if (restrictNavigation && tab !== activeTab) {
      toast({
        title: '请先完成引导',
        description: '完成当前引导步骤后即可访问其他页面',
        variant: 'default',
      });
      return;
    }
    onTabChange(tab);
  };
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
          onClick={() => handleTabChange('dashboard')}
          disabled={restrictNavigation && activeTab !== 'dashboard'}
        >
          <LayoutDashboard className="mr-2 h-4 w-4" />
          仪表板
        </Button>

        <Button
          variant={activeTab === 'tool-management' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('tool-management')}
          disabled={restrictNavigation && activeTab !== 'tool-management'}
        >
          <Wrench className="mr-2 h-4 w-4" />
          工具管理
        </Button>

        {/* 安装工具页面已废弃，保留代码供参考 */}
        {/* <Button
          variant={activeTab === 'install' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('install')}
          disabled={restrictNavigation && activeTab !== 'install'}
        >
          <Package className="mr-2 h-4 w-4" />
          安装工具
        </Button> */}

        <Button
          variant={activeTab === 'config' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('config')}
          disabled={restrictNavigation && activeTab !== 'config'}
        >
          <Key className="mr-2 h-4 w-4" />
          配置 API
        </Button>

        <Button
          variant={activeTab === 'switch' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('switch')}
          disabled={restrictNavigation && activeTab !== 'switch'}
        >
          <ArrowRightLeft className="mr-2 h-4 w-4" />
          切换配置
        </Button>

        <Button
          variant={activeTab === 'statistics' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('statistics')}
          disabled={restrictNavigation && activeTab !== 'statistics'}
        >
          <BarChart3 className="mr-2 h-4 w-4" />
          用量统计
        </Button>

        <Button
          variant={activeTab === 'transparent-proxy' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('transparent-proxy')}
          disabled={restrictNavigation && activeTab !== 'transparent-proxy'}
        >
          <Radio className="mr-2 h-4 w-4" />
          透明代理
        </Button>

        <Separator className="my-3" />

        <Button
          variant={activeTab === 'help' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('help')}
          disabled={restrictNavigation && activeTab !== 'help'}
        >
          <HelpCircle className="mr-2 h-4 w-4" />
          帮助
        </Button>

        <Button
          variant={activeTab === 'settings' ? 'default' : 'ghost'}
          className="w-full justify-start transition-all hover:scale-105"
          onClick={() => handleTabChange('settings')}
          disabled={restrictNavigation && activeTab !== 'settings'}
        >
          <SettingsIcon className="mr-2 h-4 w-4" />
          设置
        </Button>
      </nav>
    </aside>
  );
}
