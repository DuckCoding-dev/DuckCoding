import { AppSidebar } from '@/components/layout/AppSidebar';
import { useAppContext } from '@/hooks/useAppContext';
import { Toaster } from '@/components/ui/toaster';

interface MainLayoutProps {
  children: React.ReactNode;
}

export function MainLayout({ children }: MainLayoutProps) {
  const { activeTab, setActiveTab, settingsRestrictToTab, restrictedPage } = useAppContext();

  return (
    <div className="flex h-screen w-full bg-background overflow-hidden">
      {/* Sidebar */}
      <AppSidebar
        activeTab={activeTab}
        onTabChange={(tab) => setActiveTab(tab as any)}
        restrictNavigation={!!settingsRestrictToTab || !!restrictedPage}
        allowedPage={restrictedPage || (settingsRestrictToTab ? 'settings' : undefined)}
      />

      {/* Main Content Area */}
      <main className="flex-1 flex flex-col h-full overflow-hidden relative transition-all duration-300 ease-in-out">
        {/* Background Gradients/Effects can go here */}
        <div className="absolute inset-0 -z-10 bg-gradient-to-br from-background via-muted/35 to-accent/30" />

        {/* Scrollable Content */}
        <div className="flex-1 overflow-y-auto overflow-x-hidden p-4 custom-scrollbar">
          <div className="mx-auto max-w-7xl">{children}</div>
        </div>
      </main>

      <Toaster />
    </div>
  );
}
