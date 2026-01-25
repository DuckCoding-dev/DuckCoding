import { AppProvider } from '@/contexts/AppContext';
import { MainLayout } from '@/components/layout/MainLayout';
import { AppContent } from '@/components/logic/AppContent';
import { AppEventsHandler } from '@/components/logic/AppEventsHandler';
import { ConfigWatchHandler } from '@/components/logic/ConfigWatchHandler';
import { UpdateManager } from '@/components/logic/UpdateManager';
import { OnboardingManager } from '@/components/logic/OnboardingManager';

function App() {
  return (
    <AppProvider>
      <AppEventsHandler />
      <ConfigWatchHandler />
      <UpdateManager />
      <OnboardingManager />
      <MainLayout>
        <AppContent />
      </MainLayout>
    </AppProvider>
  );
}

export default App;

