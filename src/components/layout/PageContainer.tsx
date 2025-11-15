import { ReactNode } from 'react';

interface PageContainerProps {
  children: ReactNode;
  className?: string;
}

/**
 * 页面容器组件
 * 为所有页面提供统一的布局和样式
 */
export function PageContainer({ children, className = '' }: PageContainerProps) {
  return (
    <div className={`p-8 ${className}`}>
      <div className="max-w-6xl mx-auto">{children}</div>
    </div>
  );
}
