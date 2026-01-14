// 工具 Logo 映射
import ClaudeLogo from '@/assets/claude-logo.png';
import CodexLogo from '@/assets/codex-logo.png';
import GeminiLogo from '@/assets/gemini-logo.png';
import AmpLogo from '@/assets/amp-logo.svg';

export const logoMap: Record<string, string> = {
  'claude-code': ClaudeLogo,
  codex: CodexLogo,
  'gemini-cli': GeminiLogo,
  'amp-code': AmpLogo,
};

// 工具描述映射
export const descriptionMap: Record<string, string> = {
  'claude-code': 'Anthropic 官方 CLI - AI 代码助手',
  codex: 'OpenAI 代码助手 - GPT-5 Codex',
  'gemini-cli': 'Google Gemini 命令行工具',
};

// 工具组名映射
export const groupNameMap: Record<string, string> = {
  'claude-code': 'Claude Code 专用分组',
  codex: 'CodeX 专用分组',
  'gemini-cli': 'Gemini CLI 专用分组',
};

// 获取工具显示名称
export function getToolDisplayName(toolId: string): string {
  switch (toolId) {
    case 'claude-code':
      return 'Claude Code';
    case 'codex':
      return 'CodeX';
    case 'gemini-cli':
      return 'Gemini CLI';
    default:
      return toolId;
  }
}
