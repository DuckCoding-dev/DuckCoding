import { useConfigWatch } from '@/hooks/useConfigWatch';
import { ConfigChangeDialog } from '@/components/dialogs/ConfigChangeDialog';

export function ConfigWatchHandler() {
  const {
    change: configChange,
    showDialog: showConfigDialog,
    closeDialog: closeConfigDialog,
    queueLength,
  } = useConfigWatch();

  return (
    <ConfigChangeDialog
      open={showConfigDialog}
      onClose={closeConfigDialog}
      change={configChange}
      queueLength={queueLength}
    />
  );
}
