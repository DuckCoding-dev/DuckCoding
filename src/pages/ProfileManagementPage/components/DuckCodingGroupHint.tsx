/**
 * DuckCoding API Key 分组说明组件
 *
 * 显示 DuckCoding 专用分组要求、控制台链接和一键生成按钮
 */

import { Alert, AlertDescription, AlertTitle } from '@/components/ui/alert';
import { Button } from '@/components/ui/button';
import { Info, Sparkles, Loader2, ExternalLink } from 'lucide-react';
import type { ToolId } from '@/types/profile';
import { groupNameMap } from '@/utils/constants';
import { openExternalLink } from '@/utils/formatting';

interface DuckCodingGroupHintProps {
  /** 当前工具ID */
  toolId: ToolId;
  /** 一键生成按钮点击回调 */
  onGenerateClick: () => void;
  /** 是否正在生成 */
  generating: boolean;
}

/**
 * DuckCoding 分组说明组件
 */
export function DuckCodingGroupHint({
  toolId,
  onGenerateClick,
  generating,
}: DuckCodingGroupHintProps) {
  return (
    <Alert>
      <Info className="h-4 w-4" />
      <AlertTitle>DuckCoding API Key 分组说明</AlertTitle>
      <AlertDescription className="space-y-3">
        {/* 当前工具需要使用的分组 */}
        <div>
          <p className="text-xs font-semibold mb-1">当前工具需要使用：</p>
          <p className="font-mono bg-muted px-2 py-1 rounded inline-block text-xs">
            {groupNameMap[toolId]} 分组
          </p>
        </div>

        {/* 分组使用规则 */}
        <ul className="list-disc list-inside space-y-1 text-xs">
          <li>每个工具必须使用其专用分组的 API Key</li>
          <li>API Key 不能混用</li>
        </ul>

        {/* 获取 API Key 指引 */}
        <div>
          <p className="text-xs font-semibold mb-1">获取 API Key：</p>
          <button
            type="button"
            onClick={() => openExternalLink('https://duckcoding.com/console/api-providers')}
            className="inline-flex items-center gap-1 text-primary hover:underline font-medium cursor-pointer bg-transparent border-0 p-0 text-xs"
          >
            访问 DuckCoding 控制台 <ExternalLink className="h-3 w-3" />
          </button>
        </div>

        {/* 一键生成按钮 */}
        <Button
          type="button"
          onClick={onGenerateClick}
          disabled={generating}
          variant="outline"
          className="w-full"
        >
          {generating ? (
            <>
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              生成中...
            </>
          ) : (
            <>
              <Sparkles className="mr-2 h-4 w-4" />
              一键生成
            </>
          )}
        </Button>
      </AlertDescription>
    </Alert>
  );
}
