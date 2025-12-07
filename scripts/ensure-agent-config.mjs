#!/usr/bin/env node

/**
 * AI Agent 配置检查脚本
 *
 * 检查 Codex 和 Gemini CLI 的配置是否正确引用了 CLAUDE.md 作为项目文档。
 *
 * 检查逻辑：
 * 1. 先检查项目级配置，没有则检查用户级配置
 * 2. 如果用户根本没有该工具的配置目录/文件，说明不使用该工具，跳过检查
 * 3. 配置不正确时显示警告，但不阻断 check 流程
 *
 * 用法:
 *   node scripts/ensure-agent-config.mjs --mode=check   # 检查模式（默认）
 *   node scripts/ensure-agent-config.mjs --mode=fix     # 自动修复模式
 */

import fs from 'node:fs';
import path from 'node:path';
import os from 'node:os';

const MODE = parseMode(process.argv.slice(2));
const PROJECT_ROOT = process.cwd();
const HOME_DIR = os.homedir();
const TARGET_FILE = 'CLAUDE.md';

// ==================== 主流程 ====================

const results = {
  codex: checkCodex(),
  gemini: checkGemini(),
};

// 过滤掉跳过的工具
const activeResults = Object.entries(results).filter(([, r]) => !r.skipped);
const warnings = activeResults.filter(([, r]) => !r.ok);

console.log('\nAI Agent 配置检查结果：');

if (activeResults.length === 0) {
  console.log('  （未检测到 Codex 或 Gemini CLI 配置，跳过检查）');
} else {
  for (const [tool, result] of activeResults) {
    const icon = result.ok ? '✓' : '⚠';
    console.log(`  ${icon} ${tool}: ${result.message}`);
  }
}

// 显示跳过的工具
const skippedTools = Object.entries(results).filter(([, r]) => r.skipped);
if (skippedTools.length > 0) {
  const skippedNames = skippedTools.map(([name]) => name).join(', ');
  console.log(`  - 跳过: ${skippedNames}（未安装或未配置）`);
}

if (warnings.length > 0 && MODE === 'check') {
  console.log('\n提示: 运行 npm run check:fix 可自动修复上述警告。');
}

// 警告不阻断流程，始终返回成功
console.log('');

// ==================== Codex 检查 ====================

function checkCodex() {
  const projectConfigDir = path.join(PROJECT_ROOT, '.codex');
  const userConfigDir = path.join(HOME_DIR, '.codex');
  const configFileName = 'config.toml';

  const projectConfig = path.join(projectConfigDir, configFileName);
  const userConfig = path.join(userConfigDir, configFileName);

  const hasProjectConfig = fs.existsSync(projectConfig);
  const hasUserConfig = fs.existsSync(userConfig);

  // 如果两个级别都没有配置文件，说明用户不使用 Codex，跳过
  if (!hasProjectConfig && !hasUserConfig) {
    return { ok: true, skipped: true, message: '未安装或未配置' };
  }

  // 增量覆盖逻辑：先检查项目级，如果项目级没有该字段则回退到用户级
  const configsToCheck = [];
  if (hasProjectConfig) configsToCheck.push({ path: projectConfig, level: '项目级' });
  if (hasUserConfig) configsToCheck.push({ path: userConfig, level: '用户级' });

  for (const { path: configPath, level: configLevel } of configsToCheck) {
    const content = fs.readFileSync(configPath, 'utf8');
    const arrayValue = parseTomlArray(content, 'project_doc_fallback_filenames');

    // 如果该级别没有这个字段，继续检查下一级
    if (arrayValue === null) {
      continue;
    }

    // 找到了字段，检查是否包含目标文件
    if (arrayValue.includes(TARGET_FILE)) {
      return {
        ok: true,
        skipped: false,
        message: `project_doc_fallback_filenames 包含 ${TARGET_FILE} (${configLevel})`,
      };
    }

    // 字段存在但不包含目标文件
    if (MODE === 'fix') {
      return fixCodexConfig(configPath, content, configLevel, arrayValue);
    }

    return {
      ok: false,
      skipped: false,
      message: `${configLevel}配置的 project_doc_fallback_filenames 未包含 ${TARGET_FILE}`,
    };
  }

  // 所有级别都没有该字段，需要添加（fix 模式添加到优先级最高的配置）
  if (MODE === 'fix') {
    const targetConfig = configsToCheck[0];
    const content = fs.readFileSync(targetConfig.path, 'utf8');
    return fixCodexConfig(targetConfig.path, content, targetConfig.level);
  }

  return {
    ok: false,
    skipped: false,
    message: '所有配置级别都缺少 project_doc_fallback_filenames 字段',
  };
}

function fixCodexConfig(configPath, content, level, existingArray = null) {
  const newArray = existingArray ? [...existingArray, TARGET_FILE] : [TARGET_FILE];
  const arrayStr = JSON.stringify(newArray);

  let newContent;
  if (existingArray !== null) {
    // 替换现有数组
    newContent = content.replace(
      /project_doc_fallback_filenames\s*=\s*\[[^\]]*\]/,
      `project_doc_fallback_filenames = ${arrayStr}`,
    );
  } else {
    // 在文件开头添加字段
    newContent = `project_doc_fallback_filenames = ${arrayStr}\n${content}`;
  }

  fs.writeFileSync(configPath, newContent, 'utf8');
  return {
    ok: true,
    skipped: false,
    message: `已修复${level}配置，添加 ${TARGET_FILE} 到 project_doc_fallback_filenames`,
  };
}

// ==================== Gemini 检查 ====================

function checkGemini() {
  const projectConfigDir = path.join(PROJECT_ROOT, '.gemini');
  const userConfigDir = path.join(HOME_DIR, '.gemini');
  const configFileName = 'settings.json';

  const projectConfig = path.join(projectConfigDir, configFileName);
  const userConfig = path.join(userConfigDir, configFileName);

  const hasProjectConfig = fs.existsSync(projectConfig);
  const hasUserConfig = fs.existsSync(userConfig);

  // 如果两个级别都没有配置文件，说明用户不使用 Gemini，跳过
  if (!hasProjectConfig && !hasUserConfig) {
    return { ok: true, skipped: true, message: '未安装或未配置' };
  }

  // 增量覆盖逻辑：先检查项目级，如果项目级没有该字段则回退到用户级
  const configsToCheck = [];
  if (hasProjectConfig) configsToCheck.push({ path: projectConfig, level: '项目级' });
  if (hasUserConfig) configsToCheck.push({ path: userConfig, level: '用户级' });

  for (const { path: configPath, level: configLevel } of configsToCheck) {
    const content = fs.readFileSync(configPath, 'utf8');
    let config;
    try {
      config = JSON.parse(content);
    } catch {
      // JSON 解析失败，继续检查下一级
      continue;
    }

    const fileName = config?.context?.fileName;

    // 如果该级别没有这个字段，继续检查下一级
    if (fileName === undefined) {
      continue;
    }

    // 找到了字段，检查是否包含目标文件
    const hasTarget = Array.isArray(fileName)
      ? fileName.includes(TARGET_FILE)
      : fileName === TARGET_FILE;

    if (hasTarget) {
      const displayValue = Array.isArray(fileName) ? `[${fileName.join(', ')}]` : fileName;
      return {
        ok: true,
        skipped: false,
        message: `context.fileName = ${displayValue} (${configLevel})`,
      };
    }

    // 字段存在但不包含目标文件
    if (MODE === 'fix') {
      return fixGeminiConfig(configPath, config, configLevel);
    }

    const displayValue = Array.isArray(fileName) ? `[${fileName.join(', ')}]` : fileName;
    return {
      ok: false,
      skipped: false,
      message: `${configLevel}配置的 context.fileName (${displayValue}) 未包含 ${TARGET_FILE}`,
    };
  }

  // 所有级别都没有该字段，需要添加（fix 模式添加到优先级最高的配置）
  if (MODE === 'fix') {
    const targetConfig = configsToCheck[0];
    const content = fs.readFileSync(targetConfig.path, 'utf8');
    let config;
    try {
      config = JSON.parse(content);
    } catch {
      config = {};
    }
    return fixGeminiConfig(targetConfig.path, config, targetConfig.level);
  }

  return {
    ok: false,
    skipped: false,
    message: '所有配置级别都缺少 context.fileName 字段',
  };
}

function fixGeminiConfig(configPath, config, level) {
  // 确保 context 对象存在
  if (!config.context) {
    config.context = {};
  }

  const existingFileName = config.context.fileName;

  if (Array.isArray(existingFileName)) {
    // 如果是数组，添加到数组中
    if (!existingFileName.includes(TARGET_FILE)) {
      config.context.fileName = [...existingFileName, TARGET_FILE];
    }
  } else if (typeof existingFileName === 'string' && existingFileName !== TARGET_FILE) {
    // 如果是字符串且不是目标值，转为数组
    config.context.fileName = [existingFileName, TARGET_FILE];
  } else {
    // 设置为目标值
    config.context.fileName = TARGET_FILE;
  }

  fs.writeFileSync(configPath, JSON.stringify(config, null, 2) + '\n', 'utf8');
  return {
    ok: true,
    skipped: false,
    message: `已修复${level}配置，设置 context.fileName 包含 ${TARGET_FILE}`,
  };
}

// ==================== 工具函数 ====================

function parseMode(args) {
  for (const arg of args) {
    if (arg === '--fix' || arg === '--mode=fix') return 'fix';
    if (arg === '--check' || arg === '--mode=check') return 'check';
  }
  return 'check';
}

/**
 * 简单解析 TOML 数组值
 * 仅支持字符串数组，如: key = ["a", "b"]
 */
function parseTomlArray(content, key) {
  const regex = new RegExp(`^${key}\\s*=\\s*\\[([^\\]]*)]`, 'm');
  const match = content.match(regex);

  if (!match) return null;

  const arrayContent = match[1].trim();
  if (!arrayContent) return [];

  // 解析数组元素
  const elements = [];
  const elementRegex = /"([^"]*?)"/g;
  let elementMatch;
  while ((elementMatch = elementRegex.exec(arrayContent)) !== null) {
    elements.push(elementMatch[1]);
  }

  return elements;
}
