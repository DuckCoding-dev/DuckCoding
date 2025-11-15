/**
 * 版本号格式化
 * 保留 preview/beta 等标记
 */
export function formatVersionLabel(version: string | null): string {
  if (!version) return '未知';
  const trimmed = version.trim();

  // 只要包含非数字的版本标记，就直接保留原样（如 preview、beta、rust-v 等）
  if (/[a-zA-Z]/.test(trimmed.replace(/^v/i, '')) || trimmed.includes('-')) {
    return trimmed;
  }

  const match = trimmed.match(/(\d+\.\d+\.\d+)/);
  return match ? match[1] : trimmed;
}

/**
 * 脱敏显示 API Key
 */
export function maskApiKey(key: string): string {
  if (!key) return '';
  if (key.length <= 10) {
    return '*'.repeat(key.length);
  }
  const start = key.substring(0, 4);
  const end = key.substring(key.length - 4);
  const middle = '*'.repeat(Math.min(key.length - 8, 20)); // 最多显示20个星号
  return `${start}${middle}${end}`;
}

/**
 * 打开外部链接
 */
export async function openExternalLink(url: string): Promise<void> {
  try {
    // 动态导入 shell 插件
    const { open } = await import('@tauri-apps/plugin-shell');
    await open(url);
    console.log('链接已在浏览器中打开:', url);
  } catch (error) {
    console.error('打开链接失败:', error);
    // 降级方案：在浏览器环境中使用 window.open
    if (typeof window !== 'undefined') {
      window.open(url, '_blank');
    }
  }
}
