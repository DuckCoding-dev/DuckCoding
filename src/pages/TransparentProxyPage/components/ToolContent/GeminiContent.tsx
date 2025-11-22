// Gemini CLI 代理内容组件（占位）
// 等待后续开发

import { Sparkles } from 'lucide-react';

/**
 * Gemini CLI 代理内容组件
 *
 * 当前状态：占位组件，显示"等待后续开发"提示
 */
export function GeminiContent() {
  return (
    <div className="flex flex-col items-center justify-center py-20 text-center">
      <Sparkles className="h-16 w-16 text-muted-foreground/50 mb-4" />
      <h3 className="text-lg font-semibold text-muted-foreground mb-2">等待后续开发</h3>
      <p className="text-sm text-muted-foreground/80 max-w-md">
        Gemini CLI 透明代理的会话管理功能正在规划中
      </p>
    </div>
  );
}
