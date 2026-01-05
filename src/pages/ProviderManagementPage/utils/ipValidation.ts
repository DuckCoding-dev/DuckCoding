/**
 * IP 白名单验证工具
 * 支持 IPv4/IPv6 地址和 CIDR 表达式验证
 */

import * as ipaddr from 'ipaddr.js';

export interface IpValidationResult {
  valid: boolean;
  errors: string[];
  warnings: string[];
}

/**
 * 验证 IP 白名单字符串
 * @param allowIps - IP 白名单字符串（换行符分隔）
 * @returns 验证结果
 */
export function validateIpWhitelist(allowIps: string): IpValidationResult {
  const result: IpValidationResult = {
    valid: true,
    errors: [],
    warnings: [],
  };

  // 空值视为有效（表示不限制 IP）
  if (!allowIps || allowIps.trim() === '') {
    return result;
  }

  // 按行分割并清理空白
  const lines = allowIps
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0);

  if (lines.length === 0) {
    return result;
  }

  // 验证每一行
  for (let i = 0; i < lines.length; i++) {
    const line = lines[i];
    const lineNum = i + 1;

    // 检查是否为 CIDR 表达式
    if (line.includes('/')) {
      const validationResult = validateCIDR(line, lineNum);
      result.errors.push(...validationResult.errors);
      result.warnings.push(...validationResult.warnings);
    } else {
      // 单个 IP 地址
      const validationResult = validateSingleIp(line, lineNum);
      result.errors.push(...validationResult.errors);
      result.warnings.push(...validationResult.warnings);
    }
  }

  result.valid = result.errors.length === 0;
  return result;
}

/**
 * 验证 CIDR 表达式
 */
function validateCIDR(cidr: string, lineNum: number): { errors: string[]; warnings: string[] } {
  const errors: string[] = [];
  const warnings: string[] = [];

  try {
    const parts = cidr.split('/');
    if (parts.length !== 2) {
      errors.push(`第 ${lineNum} 行：CIDR 格式错误，应为 "IP/掩码长度"`);
      return { errors, warnings };
    }

    const [ip, prefixStr] = parts;
    const prefix = parseInt(prefixStr, 10);

    // 解析 IP 地址
    let addr: ipaddr.IPv4 | ipaddr.IPv6;
    try {
      addr = ipaddr.process(ip);
    } catch {
      errors.push(`第 ${lineNum} 行：无效的 IP 地址 "${ip}"`);
      return { errors, warnings };
    }

    // 验证掩码长度
    const maxPrefix = addr.kind() === 'ipv4' ? 32 : 128;
    if (isNaN(prefix) || prefix < 0 || prefix > maxPrefix) {
      errors.push(
        `第 ${lineNum} 行：无效的掩码长度 "${prefixStr}"，${addr.kind() === 'ipv4' ? 'IPv4' : 'IPv6'} 范围应为 0-${maxPrefix}`,
      );
      return { errors, warnings };
    }

    // 警告：过大的网段
    if (addr.kind() === 'ipv4' && prefix < 16) {
      warnings.push(
        `第 ${lineNum} 行：网段 ${cidr} 覆盖范围过大（包含 ${Math.pow(2, 32 - prefix)} 个地址），请确认是否符合安全策略`,
      );
    } else if (addr.kind() === 'ipv6' && prefix < 64) {
      warnings.push(`第 ${lineNum} 行：IPv6 网段 ${cidr} 覆盖范围过大，请确认是否符合安全策略`);
    }
  } catch (error) {
    errors.push(
      `第 ${lineNum} 行：CIDR 解析失败 - ${error instanceof Error ? error.message : String(error)}`,
    );
  }

  return { errors, warnings };
}

/**
 * 验证单个 IP 地址
 */
function validateSingleIp(ip: string, lineNum: number): { errors: string[]; warnings: string[] } {
  const errors: string[] = [];
  const warnings: string[] = [];

  try {
    const addr = ipaddr.process(ip);

    // 警告：私有地址
    if (addr.range() === 'private') {
      warnings.push(`第 ${lineNum} 行：检测到私有 IP 地址 ${ip}`);
    }

    // 警告：本地回环地址
    if (addr.range() === 'loopback') {
      warnings.push(`第 ${lineNum} 行：检测到本地回环地址 ${ip}，该配置可能无实际作用`);
    }
  } catch (error) {
    errors.push(
      `第 ${lineNum} 行：无效的 IP 地址 "${ip}" - ${error instanceof Error ? error.message : String(error)}`,
    );
  }

  return { errors, warnings };
}

/**
 * 格式化 IP 白名单（用于显示）
 * @param allowIps - 原始 IP 白名单字符串
 * @returns 格式化后的字符串
 */
export function formatIpWhitelist(allowIps: string): string {
  return allowIps
    .split('\n')
    .map((line) => line.trim())
    .filter((line) => line.length > 0)
    .join('\n');
}
