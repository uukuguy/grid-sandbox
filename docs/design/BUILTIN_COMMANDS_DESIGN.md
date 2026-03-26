# 内置斜杠命令设计文档

## 概述

Octo 作为企业级 AI Agent 工作台，内置斜杠命令需要覆盖两个维度：
1. **代码工程类** — 日常开发任务的标准化 prompt
2. **企业增值类** — 结合 Octo 内部机制（安全策略、Agent 目录、上下文管理等）的高价值命令

## 调研依据

设计基于以下数据源的调研：

| 数据源 | 关键发现 |
|--------|---------|
| GitHub Copilot 内置命令 | `/fix`, `/explain`, `/tests`, `/doc`, `/new` 是最高频命令 |
| Claude Code bundled skills | `/simplify`, `/review` 是 Claude 特色，`/compact` 已在 TUI 内置 |
| Awesome Claude Skills (9.8k stars) | 安全审计、代码审计、项目脚手架是社区最热门类别 |
| Claude Skills Marketplace | `feature-planning`, `code-auditor`, `project-bootstrapper` 最受欢迎 |
| SkillsMP (50万+ skills) | 安全类、文档化类、工程工作流类排名靠前 |
| 企业平台（Devin, Codex, Codegen） | 治理控制、审计追踪、合规是企业区分点 |

## 命令清单

### A. 代码工程类（6 个）

| 命令 | 用途 | 调研来源 |
|------|------|---------|
| `/review` | 代码审查（含安全/性能/合规维度） | Copilot + Claude Code 均有 |
| `/test` | 测试生成（覆盖边界、错误、性能） | Copilot `/tests` |
| `/fix` | Bug 修复（根因分析 + 修复） | Copilot `/fix` |
| `/refactor` | 代码重构 | Copilot + 社区通用 |
| `/doc` | 文档生成 | Copilot `/doc` |
| `/commit` | 规范化 Git 提交 | Skills Marketplace `git-pushing` |

### B. 企业增值类（4 个）

| 命令 | 用途 | 调研来源 |
|------|------|---------|
| `/security` | 安全审计（OWASP Top 10、敏感信息检测） | Trail of Bits Security Skills（社区高星）|
| `/plan` | 需求分解为可执行计划 | Skills Marketplace `feature-planning`（热门）|
| `/audit` | 代码库综合审计（架构+质量+安全+性能） | Skills Marketplace `code-auditor`（热门）|
| `/bootstrap` | 项目/模块脚手架创建 | Skills Marketplace `project-bootstrapper`（热门）|

### C. 移除的命令

| 命令 | 移除原因 |
|------|---------|
| `/summarize` | 太通用，不是 coding agent 核心职责 |
| `/translate` | 太通用，企业场景无特殊价值 |
| `/explain` | 功能覆盖：项目级理解 → `/audit`，代码级理解 → `/review` |
| `/optimize` | 功能覆盖：合并到 `/review` 的性能维度 |

## 模板设计原则

### 旧模板问题

旧的内置命令模板过于简陋，每个只有一句话。例如：
```
Review the following code for bugs, security issues, and improvements:

$ARGUMENTS
```

### 新模板要求

1. **结构化指引** — 明确的分析步骤和维度
2. **输出格式规范** — 指定 Markdown 格式、分级标签
3. **企业关注维度** — 安全性、可维护性、合规性
4. **可操作性** — 输出应包含具体的改进建议和代码示例
5. **上下文感知** — 提示 Agent 考虑项目整体架构

## 显示优化

### 问题

执行斜杠命令时，展开后的完整 prompt 模板会作为用户消息显示在聊天区域，
造成大段冗余文本干扰用户视觉。

### 修复方案

将 `key_handler.rs` 中 `ChatMessage::user(&expanded)` 改为显示简短的命令摘要：
- 有参数：`/review src/main.rs`
- 无参数：`/commit`

展开后的完整 prompt 仅发送给 Agent，不在 UI 中显示。

## 目录结构

```
crates/octo-engine/builtin/commands/
├── review.md        # 代码审查
├── test.md          # 测试生成
├── fix.md           # Bug 修复
├── refactor.md      # 代码重构
├── doc.md           # 文档生成
├── commit.md        # 规范提交
├── security.md      # 安全审计
├── plan.md          # 需求分解
├── audit.md         # 代码库审计
└── bootstrap.md     # 项目脚手架
```

## 优先级体系（不变）

```
项目命令 (.octo/commands/) > 全局命令 (~/.octo/commands/) > 内置命令 (编译嵌入)
```

用户可以在任何层级覆盖内置命令，实现企业定制化。
