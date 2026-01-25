import { useAppContext } from '@/hooks/useAppContext';
import { UpdateDialog } from '@/components/dialogs/UpdateDialog';

export function UpdateManager() {
  const { updateInfo, setUpdateInfo, isUpdateDialogOpen, setIsUpdateDialogOpen, checkAppUpdates } =
    useAppContext();

  return (
    <UpdateDialog
      open={isUpdateDialogOpen}
      onOpenChange={setIsUpdateDialogOpen}
      updateInfo={updateInfo}
      onCheckForUpdate={() => {
        setUpdateInfo(null);
        checkAppUpdates(true);
      }}
    />
  );
}
