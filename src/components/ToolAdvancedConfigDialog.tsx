import { Dialog, DialogContent, DialogHeader, DialogTitle } from '@/components/ui/dialog';
import {
  ClaudeConfigManager,
  CodexConfigManager,
  GeminiConfigManager,
} from '@/components/config-managers';
import { logoMap } from '@/utils/constants';

interface ToolAdvancedConfigDialogProps {
  toolId: string;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

// 工具名称映射
const TOOL_NAME_MAP: Record<string, string> = {
  'claude-code': 'Claude Code',
  codex: 'Codex',
  'gemini-cli': 'Gemini CLI',
};

export function ToolAdvancedConfigDialog({
  toolId,
  open,
  onOpenChange,
}: ToolAdvancedConfigDialogProps) {
  const toolName = TOOL_NAME_MAP[toolId] || toolId;

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent
        className="sm:max-w-[90vw] max-h-[90vh] flex flex-col"
        onInteractOutside={(e) => e.preventDefault()}
      >
        <DialogHeader className="flex-shrink-0">
          <DialogTitle className="flex items-center gap-2">
            <img src={logoMap[toolId]} alt={toolName} className="w-5 h-5" />
            <span>{toolName} 高级配置</span>
          </DialogTitle>
        </DialogHeader>
        <div className="mt-4 flex-1 overflow-y-auto min-h-0">
          {toolId === 'claude-code' && <ClaudeConfigManager />}
          {toolId === 'codex' && <CodexConfigManager />}
          {toolId === 'gemini-cli' && <GeminiConfigManager />}
        </div>
      </DialogContent>
    </Dialog>
  );
}
