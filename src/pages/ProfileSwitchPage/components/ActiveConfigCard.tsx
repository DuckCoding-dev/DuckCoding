import { Key, AlertTriangle } from 'lucide-react';
import { maskApiKey } from '@/utils/formatting';
import type { ActiveConfig, GlobalConfig } from '@/lib/tauri-commands';

interface ActiveConfigCardProps {
  toolId: string;
  activeConfig: ActiveConfig;
  globalConfig: GlobalConfig | null;
  transparentProxyEnabled: boolean;
}

export function ActiveConfigCard({
  toolId,
  activeConfig,
  globalConfig,
  transparentProxyEnabled,
}: ActiveConfigCardProps) {
  const isClaudeCodeWithProxy = toolId === 'claude-code' && transparentProxyEnabled;
  const hasProxyConfigMissing =
    !globalConfig?.transparent_proxy_real_api_key || !globalConfig?.transparent_proxy_real_base_url;

  return (
    <div className="mb-6 p-4 bg-gradient-to-r from-blue-50 to-cyan-50 dark:from-blue-950 dark:to-cyan-950 rounded-lg border border-blue-200 dark:border-blue-800">
      <div className="flex items-center gap-2 mb-3">
        <Key className="h-5 w-5 text-blue-600 dark:text-blue-400" />
        <h4 className="font-semibold text-blue-900 dark:text-blue-100">
          {isClaudeCodeWithProxy ? '透明代理配置' : '当前生效配置'}
        </h4>
      </div>
      <div className="space-y-2 text-sm">
        {/* 透明代理配置显示 */}
        {isClaudeCodeWithProxy ? (
          <>
            {/* 配置缺失警告 */}
            {hasProxyConfigMissing && (
              <div className="mb-4 p-3 bg-gradient-to-r from-red-50 to-orange-50 dark:from-red-950 dark:to-orange-950 rounded-lg border border-red-200 dark:border-red-800">
                <div className="flex items-start gap-2">
                  <AlertTriangle className="h-5 w-5 text-red-600 dark:text-red-400 flex-shrink-0 mt-0.5" />
                  <div className="space-y-1">
                    <h5 className="font-semibold text-red-900 dark:text-red-100">
                      ⚠️ 透明代理配置缺失
                    </h5>
                    <p className="text-sm text-red-800 dark:text-red-200">
                      检测到透明代理功能已开启，但缺少真实的API配置。请先选择一个有效的配置文件，然后再启动透明代理。
                    </p>
                    <p className="text-xs text-red-700 dark:text-red-300 mt-2">
                      可能导致请求回环或连接问题
                    </p>
                  </div>
                </div>
              </div>
            )}

            <ConfigField label="配置名称:" value="透明代理配置" highlight />
            <ConfigField
              label="API Key:"
              value={
                globalConfig?.transparent_proxy_real_api_key
                  ? maskApiKey(globalConfig.transparent_proxy_real_api_key)
                  : '⚠️ 未配置'
              }
              isError={!globalConfig?.transparent_proxy_real_api_key}
            />
            <ConfigField
              label="Base URL:"
              value={globalConfig?.transparent_proxy_real_base_url || '⚠️ 未配置'}
              isError={!globalConfig?.transparent_proxy_real_base_url}
            />
          </>
        ) : (
          <>
            {activeConfig.profile_name && (
              <ConfigField label="配置名称:" value={activeConfig.profile_name} highlight />
            )}
            <ConfigField label="API Key:" value={maskApiKey(activeConfig.api_key)} />
            <ConfigField label="Base URL:" value={activeConfig.base_url} />
          </>
        )}
      </div>
    </div>
  );
}

// 配置字段显示组件
interface ConfigFieldProps {
  label: string;
  value: string;
  highlight?: boolean;
  isError?: boolean;
}

function ConfigField({ label, value, highlight, isError }: ConfigFieldProps) {
  const valueClassName = isError
    ? 'text-red-600 dark:text-red-400 bg-red-50 dark:bg-red-950'
    : highlight
      ? 'font-semibold text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50'
      : 'font-mono text-blue-900 dark:text-blue-100 bg-white/50 dark:bg-slate-900/50';

  return (
    <div className="flex items-start gap-2">
      <span className="text-blue-700 dark:text-blue-300 font-medium min-w-20">{label}</span>
      <span className={`px-2 py-0.5 rounded break-all ${valueClassName}`}>{value}</span>
    </div>
  );
}
