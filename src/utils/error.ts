export type UnknownRecord = Record<string, unknown>;

function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === 'object' && value !== null;
}

function getStringField(obj: UnknownRecord, key: string): string | undefined {
  const value = obj[key];
  if (typeof value !== 'string') return undefined;
  const trimmed = value.trim();
  return trimmed ? trimmed : undefined;
}

/**
 * 将 unknown 错误转换为可读的字符串。
 *
 * 说明：
 * - Tauri commands 可能抛出结构化对象（如 AppError 序列化结果），直接 String(err) 会变成 "[object Object]".
 */
export function getErrorMessage(error: unknown): string {
  if (error == null) return '未知错误';
  if (typeof error === 'string') return error;
  if (error instanceof Error) return error.message || String(error);
  if (typeof error === 'number' || typeof error === 'boolean' || typeof error === 'bigint') {
    return String(error);
  }

  if (isRecord(error)) {
    const message = getStringField(error, 'message');
    if (message) return message;

    const type = getStringField(error, 'type');
    const field = getStringField(error, 'field');
    const reason = getStringField(error, 'reason');

    if (type === 'ValidationError' && field && reason) {
      return `验证失败：${field}，原因：${reason}`;
    }

    if (reason) return reason;

    const err = getStringField(error, 'error');
    if (err) return err;

    try {
      return JSON.stringify(error);
    } catch {
      return String(error);
    }
  }

  return String(error);
}
