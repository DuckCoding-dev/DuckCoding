#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import fs from 'node:fs';
import path from 'node:path';

const mode = process.argv.includes('--fix') ? 'fix' : 'check';
const projectRoot = process.cwd();

const steps =
  mode === 'fix'
    ? [
        {
          name: 'AI Agent 配置修复',
          command: ['node', 'scripts/ensure-agent-config.mjs', '--mode=fix'],
        },
        { name: '前端 ESLint 修复', command: ['npm', 'run', 'lint:ts:fix'] },
        {
          name: '后端 Rust Clippy 修复',
          command: ['npm', 'run', 'lint:rs:fix'],
          needsDist: true,
        },
        { name: '前端 Prettier 修复', command: ['npm', 'run', 'fmt:ts:fix'] },
        { name: '后端 Rust fmt 修复', command: ['npm', 'run', 'fmt:rs:fix'] },
      ]
    : [
        {
          name: 'AI Agent 配置检查',
          command: ['node', 'scripts/ensure-agent-config.mjs', '--mode=check'],
        },
        { name: '前端 ESLint 检查', command: ['npm', 'run', 'lint:ts'] },
        {
          name: '后端 Rust Clippy 检查',
          command: ['npm', 'run', 'lint:rs'],
          needsDist: true,
        },
        { name: '前端 Prettier 检查', command: ['npm', 'run', 'fmt:ts'] },
        { name: '后端 Rust fmt 检查', command: ['npm', 'run', 'fmt:rs'] },
      ];

const failures = [];
let distPrepared = false;

for (const step of steps) {
  banner(`开始 ${step.name}…`);
  if (step.needsDist && !distPrepared) {
    const prepareResult = ensureDist();
    if (!prepareResult) {
      failures.push({ name: `${step.name} 前置：npm run build`, status: 1 });
      continue;
    }
    distPrepared = true;
  }
  const result = spawnSync(step.command[0], step.command.slice(1), {
    stdio: 'inherit',
    cwd: projectRoot,
    shell: process.platform === 'win32',
  });
  if (result.status !== 0) {
    console.log('DEBUG: Command failed');
    console.log('Error:', result.error);
    console.log('Status:', result.status);
    console.log('Signal:', result.signal);
    banner(`⚠ ${step.name} 失败`);
    failures.push({ name: step.name, status: result.status ?? 1 });
    continue;
  }
  banner(`✓ ${step.name} 完成`);
}

if (failures.length > 0) {
  banner(
    `${mode === 'fix' ? '修复流程已结束' : '检查流程已结束'}，共有 ${
      failures.length
    } 项失败：\n${failures.map((item) => `- ${item.name}（退出码 ${item.status}）`).join('\n')}`,
  );
  process.exit(failures[0]?.status ?? 1);
}

banner(mode === 'fix' ? '全部修复步骤已完成，请重新运行 npm run check 确认' : '全部检查已通过');

function banner(message) {
  const line = '-'.repeat(message.length);
  console.log(`\n${line}\n${message}\n${line}`);
}

function ensureDist() {
  const distPath = path.join(projectRoot, 'dist');
  try {
    fs.accessSync(distPath, fs.constants.R_OK);
    return true;
  } catch {
    banner('未找到 dist/，尝试运行 npm run build 以便 Tauri Clippy 能找到前端产物…');
    const result = spawnSync('npm', ['run', 'build'], {
      stdio: 'inherit',
      cwd: projectRoot,
      shell: process.platform === 'win32',
    });
    if (result.status !== 0) {
      banner('⚠ npm run build 失败，后续 Rust Clippy 将跳过');
      return false;
    }
    return true;
  }
}
