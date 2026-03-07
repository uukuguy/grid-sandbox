#!/usr/bin/env node
/**
 * ADR/DDD Auto-Generator
 *
 * Reads architecture changes accumulated by intelligence.cjs and generates
 * ADR documents following the naming convention in settings.json.
 *
 * Config (from .claude/settings.json → claudeFlow.adr):
 *   directory:      "/docs/adr"
 *   filePattern:    "ADR_*.md"
 *   sectionPattern: "^## ADR-\\d+"
 *   naming:         "ADR_{TOPIC}.md"
 *   template:       "madr"
 *
 * Called by hook-handler.cjs post-task when architecture changes are detected.
 */

'use strict';

const fs = require('fs');
const path = require('path');

const CWD = process.cwd();

// ── Config ──────────────────────────────────────────────────────────────────

function getSettings() {
  const settingsPath = path.join(CWD, '.claude', 'settings.json');
  try {
    if (fs.existsSync(settingsPath)) return JSON.parse(fs.readFileSync(settingsPath, 'utf-8'));
  } catch { /* ignore */ }
  return null;
}

function getAdrConfig() {
  const settings = getSettings();
  const cf = (settings && settings.claudeFlow) || {};
  return {
    directory: ((cf.adr && cf.adr.directory) || '/docs/adr').replace(/^\//, ''),
    naming: (cf.adr && cf.adr.naming) || 'ADR-{NUM}-{TOPIC}.md',
    template: (cf.adr && cf.adr.template) || 'madr',
    sectionPattern: (cf.adr && cf.adr.sectionPattern) || '^## ADR-\\d+',
    autoGenerate: cf.adr && cf.adr.autoGenerate !== undefined ? cf.adr.autoGenerate : true,
    language: (cf.adr && cf.adr.language) || 'en',
  };
}

function getDddConfig() {
  const settings = getSettings();
  const cf = (settings && settings.claudeFlow) || {};
  return {
    directory: ((cf.ddd && cf.ddd.directory) || '/docs/ddd').replace(/^\//, ''),
    trackDomains: cf.ddd && cf.ddd.trackDomains !== undefined ? cf.ddd.trackDomains : true,
    validateBoundedContexts: cf.ddd && cf.ddd.validateBoundedContexts !== undefined ? cf.ddd.validateBoundedContexts : true,
    language: (cf.ddd && cf.ddd.language) || 'en',
  };
}

// ── Helpers ─────────────────────────────────────────────────────────────────

function ensureDir(dir) {
  if (!fs.existsSync(dir)) fs.mkdirSync(dir, { recursive: true });
}

function getNextAdrNumber(adrDir, sectionPattern) {
  let maxNum = 0;
  try {
    if (!fs.existsSync(adrDir)) return 1;
    const regex = new RegExp(sectionPattern, 'gm');
    const files = fs.readdirSync(adrDir).filter(f => f.endsWith('.md'));
    for (const file of files) {
      const content = fs.readFileSync(path.join(adrDir, file), 'utf-8');
      const matches = content.matchAll(/ADR-(\d+)/g);
      for (const m of matches) {
        const num = parseInt(m[1], 10);
        if (num > maxNum) maxNum = num;
      }
    }
  } catch { /* ignore */ }
  return maxNum + 1;
}

// Map category to human-readable topic name
const CATEGORY_TOPICS = {
  'security': 'SECURITY',
  'agent-architecture': 'AGENT_ARCHITECTURE',
  'mcp-integration': 'MCP_INTEGRATION',
  'memory-architecture': 'MEMORY_ARCHITECTURE',
  'provider-chain': 'PROVIDER_CHAIN',
  'dependency-change': 'DEPENDENCY',
  'api-change': 'API_CHANGE',
  'structural-change': 'STRUCTURAL',
  // ==== NEW: Expanded category coverage ====
  'hooks-system': 'HOOKS_SYSTEM',
  'event-system': 'EVENT_SYSTEM',
  'scheduler-system': 'SCHEDULER_SYSTEM',
  'secret-manager': 'SECRET_MANAGER',
  'observability': 'OBSERVABILITY',
  'sandbox-system': 'SANDBOX_SYSTEM',
  'extension-system': 'EXTENSION_SYSTEM',
  'session-management': 'SESSION_MANAGEMENT',
  'audit-system': 'AUDIT_SYSTEM',
  'context-engineering': 'CONTEXT_ENGINEERING',
  'logging-system': 'LOGGING_SYSTEM',
  'skill-system': 'SKILL_SYSTEM',
  'tools-system': 'TOOLS_SYSTEM',
  'database-layer': 'DATABASE_LAYER',
};

// ── ADR Generation ──────────────────────────────────────────────────────────

/**
 * Generate ADR file for a set of architecture changes.
 *
 * @param {Array<{file: string, category: string, timestamp: number}>} changes
 * @returns {{ created: string[], appended: string[], skipped: string[] }}
 */
function generateAdr(changes) {
  const config = getAdrConfig();

  // Respect autoGenerate flag from settings.json
  if (!config.autoGenerate) {
    return { created: [], appended: [], skipped: [], disabled: true };
  }

  const adrDir = path.join(CWD, config.directory);
  ensureDir(adrDir);

  const result = { created: [], appended: [], skipped: [] };

  // Group changes by category
  const byCategory = {};
  for (const change of changes) {
    const cat = change.category || 'structural-change';
    if (!byCategory[cat]) byCategory[cat] = [];
    byCategory[cat].push(change);
  }

  for (const [category, catChanges] of Object.entries(byCategory)) {
    const topic = CATEGORY_TOPICS[category] || category.toUpperCase().replace(/-/g, '_');
    const nextNum = getNextAdrNumber(adrDir, config.sectionPattern);
    // Support both {TOPIC} and {NUM} placeholders
    let fileName = config.naming
      .replace('{TOPIC}', topic)
      .replace('{NUM}', String(nextNum).padStart(3, '0'));
    const filePath = path.join(adrDir, fileName);

    // Check if ADR file for this topic already exists
    if (fs.existsSync(filePath)) {
      // Append new section to existing file
      const existing = fs.readFileSync(filePath, 'utf-8');

      const newSection = generateAdrSection(nextNum, category, catChanges);

      // Append after last ADR section
      fs.appendFileSync(filePath, '\n---\n\n' + newSection);
      result.appended.push(filePath);
    } else {
      // Create new ADR file
      const content = generateAdrFile(nextNum, topic, category, catChanges);
      fs.writeFileSync(filePath, content, 'utf-8');
      result.created.push(filePath);
    }
  }

  return result;
}

function generateAdrFile(startNum, topic, category, changes) {
  const config = getAdrConfig();
  const isEnglish = config.language === 'en';
  const date = new Date().toISOString().split('T')[0];
  const title = topic.replace(/_/g, ' ');
  const section = generateAdrSection(startNum, category, changes);

  if (isEnglish) {
    return `# ADR-${String(startNum).padStart(3, '0')}: ${title}

**Project**: octo-sandbox
**Date**: ${date}
**Status**: Pending Review
**Auto-generated**: By RuFlo post-task hook

---

${section}
`;
  }

  return `# ADR：${title} 架构决策记录

**项目**：octo-sandbox
**日期**：${date}
**状态**：待审阅
**自动生成**：由 RuFlo post-task hook 触发

---

${section}
`;
}

function generateAdrSection(num, category, changes) {
  const config = getAdrConfig();
  const isEnglish = config.language === 'en';
  const date = new Date().toISOString().split('T')[0];
  const padNum = String(num).padStart(3, '0');
  const files = changes.map(c => c.file);
  const title = getCategoryTitle(category);

  if (isEnglish) {
    return `## ADR-${padNum}: ${title}

### Status

**Pending Review** — ${date} (auto-generated)

### Context

The following files have architecture-level changes that require decision recording:

${files.map(f => '- `' + f + '`').join('\n')}

### Change Category

- **Category**: ${category}
- **Impact Scope**: ${files.length} files
- **Detection Time**: ${date}

### Decision

> **TODO**: Please review the above changes and document the architecture decision, alternatives, and rationale.

### Consequences

#### Positive
- (To be added)

#### Negative
- (To be added)

### Affected Files

${files.map(f => '| `' + f + '` | Change |').join('\n')}
`;
  }

  return `## ADR-${padNum}：${title}

### 状态

**待审阅** — ${date}（自动生成）

### 上下文

以下文件发生了架构级变更，需要记录决策：

${files.map(f => '- `' + f + '`').join('\n')}

### 变更类别

- **类别**：${category}
- **影响范围**：${files.length} 个文件
- **检测时间**：${date}

### 决策

> **待补充**：请审阅上述变更并补充架构决策的具体内容、替代方案和理由。

### 后果

#### 正面
- （待补充）

#### 负面
- （待补充）

### 涉及文件

${files.map(f => '| `' + f + '` | 变更 |').join('\n')}
`;
}

function getCategoryTitle(category) {
  const titles = {
    'security': '安全策略变更',
    'agent-architecture': 'Agent 架构变更',
    'mcp-integration': 'MCP 集成变更',
    'memory-architecture': '记忆架构变更',
    'provider-chain': 'Provider 链变更',
    'dependency-change': '依赖变更',
    'api-change': 'API 接口变更',
    'structural-change': '结构性变更',
    // ==== NEW: Expanded category titles (Chinese) ====
    'hooks-system': 'Hooks 系统变更',
    'event-system': 'Event 系统变更',
    'scheduler-system': '调度器系统变更',
    'secret-manager': '密钥管理器变更',
    'observability': '可观测性变更',
    'sandbox-system': '沙箱系统变更',
    'extension-system': '扩展系统变更',
    'session-management': '会话管理变更',
    'audit-system': '审计系统变更',
    'context-engineering': '上下文工程变更',
    'logging-system': '日志系统变更',
    'skill-system': 'Skill 系统变更',
    'tools-system': '工具系统变更',
    'database-layer': '数据库层变更',
  };
  return titles[category] || category;
}

// ── DDD Auto-Update ─────────────────────────────────────────────────────────

// Map architecture changes to DDD bounded contexts
const CONTEXT_MAPPING = {
  'agent-architecture': 'Agent Execution Context',
  'security':           'Security Policy Context',
  'mcp-integration':    'MCP Integration Context',
  'memory-architecture': 'Memory Management Context',
  'provider-chain':     'Provider Context',
  'api-change':         'API Interface Context',
  'structural-change':  'Common Structure',
  // ==== NEW: Expanded context mapping ====
  'hooks-system':       'Orchestration Context',
  'event-system':       'Observability Context',
  'scheduler-system':    'Scheduler Context',
  'secret-manager':     'Security Policy Context',
  'observability':      'Observability Context',
  'sandbox-system':    'Sandbox Execution Context',
  'extension-system':   'Extension System Context',
  'session-management': 'Session Management Context',
  'audit-system':       'Observability Context',
  'context-engineering': 'Agent Execution Context',
  'logging-system':     'Observability Context',
  'skill-system':       'Skill Execution Context',
  'tools-system':       'Tool Execution Context',
  'database-layer':     'Persistence Context',
};

const CONTEXT_MAPPING_ZH = {
  'agent-architecture': 'Agent 执行上下文',
  'security':           '安全策略上下文',
  'mcp-integration':    'MCP 集成上下文',
  'memory-architecture': '记忆管理上下文',
  'provider-chain':     'Provider 上下文',
  'api-change':         'API 接口上下文',
  'structural-change':  '通用结构',
  // ==== NEW: Expanded context mapping (Chinese) ====
  'hooks-system':       '编排上下文',
  'event-system':       '可观测性上下文',
  'scheduler-system':    '调度器上下文',
  'secret-manager':     '安全策略上下文',
  'observability':      '可观测性上下文',
  'sandbox-system':    '沙箱执行上下文',
  'extension-system':   '扩展系统上下文',
  'session-management': '会话管理上下文',
  'audit-system':       '可观测性上下文',
  'context-engineering': 'Agent 执行上下文',
  'logging-system':     '可观测性上下文',
  'skill-system':       'Skill 执行上下文',
  'tools-system':       '工具执行上下文',
  'database-layer':     '持久化上下文',
};

// File path patterns → bounded context
const PATH_TO_CONTEXT = [
  { pattern: /agent\//,    context: 'Agent Execution Context' },
  { pattern: /security\//,  context: 'Security Policy Context' },
  { pattern: /mcp\//,       context: 'MCP Integration Context' },
  { pattern: /memory\//,    context: 'Memory Management Context' },
  { pattern: /tools\//,     context: 'Tool Execution Context' },
  { pattern: /providers?\//,context: 'Provider Context' },
  { pattern: /auth\//,      context: 'Authentication Context' },
  { pattern: /event\//,     context: 'Observability Context' },
  { pattern: /session\//,   context: 'Session Management Context' },
  { pattern: /hooks?\//,    context: 'Orchestration Context' },
  { pattern: /orchestrat/,  context: 'Orchestration Context' },
  { pattern: /sandbox\//,   context: 'Sandbox Execution Context' },
  // ==== NEW: Expanded path mapping ====
  { pattern: /scheduler\//, context: 'Scheduler Context' },
  { pattern: /secret\//,   context: 'Security Policy Context' },
  { pattern: /metrics\//,  context: 'Observability Context' },
  { pattern: /metering\//,  context: 'Observability Context' },
  { pattern: /extension\//, context: 'Extension System Context' },
  { pattern: /audit\//,     context: 'Observability Context' },
  { pattern: /context\//,   context: 'Agent Execution Context' },
  { pattern: /logging\//,   context: 'Observability Context' },
  { pattern: /skill_runtime\//, context: 'Skill Execution Context' },
  { pattern: /skills\//,    context: 'Skill Execution Context' },
  { pattern: /db\//,        context: 'Persistence Context' },
];

const PATH_TO_CONTEXT_ZH = [
  { pattern: /agent\//,    context: 'Agent 执行上下文' },
  { pattern: /security\//,  context: '安全策略上下文' },
  { pattern: /mcp\//,       context: 'MCP 集成上下文' },
  { pattern: /memory\//,    context: '记忆管理上下文' },
  { pattern: /tools\//,     context: '工具执行上下文' },
  { pattern: /providers?\//,context: 'Provider 上下文' },
  { pattern: /auth\//,      context: '认证授权上下文' },
  { pattern: /event\//,     context: '可观测性上下文' },
  { pattern: /session\//,   context: '会话管理上下文' },
  { pattern: /hooks?\//,    context: '编排上下文' },
  { pattern: /orchestrat/,  context: '编排上下文' },
  { pattern: /sandbox\//,   context: '沙箱执行上下文' },
  // ==== NEW: Expanded path mapping (Chinese) ====
  { pattern: /scheduler\//, context: '调度器上下文' },
  { pattern: /secret\//,   context: '安全策略上下文' },
  { pattern: /metrics\//,  context: '可观测性上下文' },
  { pattern: /metering\//,  context: '可观测性上下文' },
  { pattern: /extension\//, context: '扩展系统上下文' },
  { pattern: /audit\//,     context: '可观测性上下文' },
  { pattern: /context\//,   context: 'Agent 执行上下文' },
  { pattern: /logging\//,   context: '可观测性上下文' },
  { pattern: /skill_runtime\//, context: 'Skill 执行上下文' },
  { pattern: /skills\//,    context: 'Skill 执行上下文' },
  { pattern: /db\//,        context: '持久化上下文' },
];

/**
 * Update DDD tracking log when architecture changes affect bounded contexts.
 *
 * @param {Array<{file: string, category: string, timestamp: number}>} changes
 * @returns {{ updated: boolean, contextsAffected: string[], logFile: string }}
 */
function updateDddTracking(changes) {
  const config = getDddConfig();
  const isEnglish = config.language === 'en';

  // Respect trackDomains flag from settings.json
  if (!config.trackDomains) {
    return { updated: false, contextsAffected: [], logFile: '', skipped: 'trackDomains is disabled' };
  }

  const dddDir = path.join(CWD, config.directory);
  ensureDir(dddDir);

  const logFile = path.join(dddDir, 'DDD_CHANGE_LOG.md');
  const result = { updated: false, contextsAffected: [], logFile };

  // Identify affected bounded contexts
  const contextsSet = new Set();
  const mapping = config.language === 'en' ? CONTEXT_MAPPING : CONTEXT_MAPPING_ZH;
  const pathMapping = config.language === 'en' ? PATH_TO_CONTEXT : PATH_TO_CONTEXT_ZH;
  for (const change of changes) {
    // By category
    const catCtx = mapping[change.category];
    if (catCtx) contextsSet.add(catCtx);

    // By file path
    for (const m of pathMapping) {
      if (m.pattern.test(change.file)) {
        contextsSet.add(m.context);
      }
    }
  }

  if (contextsSet.size === 0) return result;

  result.contextsAffected = [...contextsSet];
  result.updated = true;

  const date = new Date().toISOString().split('T')[0];
  const time = new Date().toISOString().split('T')[1].substring(0, 5);
  const files = changes.map(c => c.file);

  let entry;
  if (isEnglish) {
    entry = `
### ${date} ${time} — Bounded Context Change

**Affected Bounded Contexts**: ${result.contextsAffected.join(', ')}

**Changed Files**:
${files.map(f => '- `' + f + '`').join('\n')}

**Change Categories**: ${[...new Set(changes.map(c => getCategoryTitle(c.category)))].join(', ')}

> Please check \`DDD_DOMAIN_ANALYSIS.md\` for updated type definitions and aggregate roots.

---
`;
  } else {
    entry = `
### ${date} ${time} — 限界上下文变更

**受影响的限界上下文**：${result.contextsAffected.join('、')}

**变更文件**：
${files.map(f => '- `' + f + '`').join('\n')}

**变更类别**：${[...new Set(changes.map(c => getCategoryTitle(c.category)))].join('、')}

> 请检查 \`DDD_DOMAIN_ANALYSIS.md\` 中对应限界上下文的类型定义和聚合根是否需要更新。

---
`;
  }

  // Append or create log file
  if (fs.existsSync(logFile)) {
    const existing = fs.readFileSync(logFile, 'utf-8');
    fs.writeFileSync(logFile, existing + entry, 'utf-8');
  } else {
    const header = isEnglish
      ? `# DDD Change Log

> Auto-generated by RuFlo post-task hook.
> Records architecture changes affecting bounded contexts.

---
`
      : `# DDD 变更追踪日志

> 由 RuFlo post-task hook 自动生成。
> 记录每次架构变更对限界上下文的影响，提醒更新 DDD 领域模型。

---
`;
    fs.writeFileSync(logFile, header + entry, 'utf-8');
  }

  return result;
}

// ── Exports ─────────────────────────────────────────────────────────────────

module.exports = { generateAdr, updateDddTracking, getAdrConfig, getDddConfig };

// ── CLI ─────────────────────────────────────────────────────────────────────

if (require.main === module) {
  const cmd = process.argv[2];

  if (cmd === 'generate') {
    let changes;
    try {
      const input = process.argv[3] || '[]';
      changes = JSON.parse(input);
    } catch {
      console.error('Usage: adr-generator.cjs generate \'[{"file":"...","category":"..."}]\'');
      process.exit(1);
    }
    const adrResult = generateAdr(changes);
    const dddResult = updateDddTracking(changes);
    console.log(JSON.stringify({ adr: adrResult, ddd: dddResult }, null, 2));
  } else if (cmd === 'config') {
    console.log('ADR:', JSON.stringify(getAdrConfig(), null, 2));
    console.log('DDD:', JSON.stringify(getDddConfig(), null, 2));
  } else {
    console.log('Usage: adr-generator.cjs <generate|config>');
  }
}
