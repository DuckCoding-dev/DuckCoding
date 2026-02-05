import { useAppContext } from '@/hooks/useAppContext';
import { DashboardPage } from '@/pages/DashboardPage';
import { InstallationPage } from '@/pages/InstallationPage';
import ProfileManagementPage from '@/pages/ProfileManagementPage';
import { TransparentProxyPage } from '@/pages/TransparentProxyPage';
import { ToolManagementPage } from '@/pages/ToolManagementPage';
import { BalancePage } from '@/pages/BalancePage';
import TokenStatisticsPage from '@/pages/TokenStatisticsPage';
import { ProviderManagementPage } from '@/pages/ProviderManagementPage';
import { SettingsPage } from '@/pages/SettingsPage';
import { HelpPage } from '@/pages/HelpPage';
import { AboutPage } from '@/pages/AboutPage';

// We need to pass some props that are not yet in context, or refactor pages to use context
// For now, we'll try to get what we can from context.

export function AppContent() {
  const { activeTab, tokenStatsParams, selectedProxyToolId } = useAppContext();

  // Note: Some pages might need props that were handled in App.tsx (like onUpdateCheck)
  // We will need to handle those interactions via context or a global event bus later.
  // For this step, I'll pass dummy or context-derived functions.

  const content = (() => {
    switch (activeTab) {
      case 'dashboard':
        return <DashboardPage />;
      case 'tool-management':
        return <ToolManagementPage />;
      case 'install':
        return <InstallationPage />;
      case 'balance':
        return <BalancePage />;
      case 'profile-management':
        return <ProfileManagementPage />;
      case 'transparent-proxy':
        return <TransparentProxyPage selectedToolId={selectedProxyToolId} />;
      case 'token-statistics':
        return (
          <TokenStatisticsPage
            sessionId={tokenStatsParams.sessionId}
            toolType={tokenStatsParams.toolType}
          />
        );
      case 'settings':
        return <SettingsPage />;
      case 'provider-management':
        return <ProviderManagementPage />;
      case 'help':
        return <HelpPage />;
      case 'about':
        return <AboutPage />;
      default:
        return null;
    }
  })();

  return (
    <div
      // Re-mount on tab change so tailwindcss-animate classes re-trigger
      key={activeTab}
      className="animate-in fade-in slide-in-from-right-2 duration-300 ease-out motion-reduce:animate-none"
    >
      {content}
    </div>
  );
}
