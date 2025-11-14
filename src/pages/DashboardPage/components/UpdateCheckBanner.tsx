import { Alert, AlertDescription } from '@/components/ui/alert';
import { AlertCircle } from 'lucide-react';

interface UpdateCheckBannerProps {
  message: {
    type: 'success' | 'error';
    text: string;
  };
}

export function UpdateCheckBanner({ message }: UpdateCheckBannerProps) {
  return (
    <Alert className={`mb-6 ${message.type === 'error' ? 'border-red-500' : ''}`}>
      <AlertCircle className="h-4 w-4" />
      <AlertDescription>{message.text}</AlertDescription>
    </Alert>
  );
}
