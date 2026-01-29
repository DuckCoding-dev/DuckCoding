import { ReactNode } from 'react';
import { cn } from '@/lib/utils';

interface PageContainerProps {
  children: ReactNode;
  className?: string;
  header?: ReactNode;
  title?: string;
  description?: string;
  actions?: ReactNode;
}

/**
 * Enhanced Page Container Component
 * Provides a unified layout structure for all pages with optional header, title, and description.
 */
export function PageContainer({
  children,
  className = '',
  header,
  title,
  description,
  actions,
}: PageContainerProps) {
  return (
    <div className={cn('space-y-4 pb-6', className)}>
      {/* Optional Standard Header Section */}
      {(title || header || actions) && (
        <div className="flex flex-col gap-3 md:flex-row md:items-center md:justify-between">
          <div className="space-y-1">
            {title && <h1 className="text-xl font-bold tracking-tight">{title}</h1>}
            {description && <p className="text-sm text-muted-foreground">{description}</p>}
            {header}
          </div>
          {actions && <div className="flex items-center gap-2">{actions}</div>}
        </div>
      )}

      {/* Main Content */}
      <div className="space-y-4">{children}</div>
    </div>
  );
}
