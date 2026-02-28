# octo-workbench v1.0 完整实施规划 (v3)

> **创建日期**: 2026-03-01
> **状态**: 方案设计 v3 - 含测试案例
> **基于**: 验证结果 + 设计文档 + 竞品分析 + 行业调研

---

## 一、验证结果总结

### 1.1 功能验证状态

| 功能 | 验证结果 | 状态 |
|------|----------|------|
| AI 对话 | 发送 "list files in /tmp" 成功执行 | ✅ 正常 |
| Bash 工具 | ls 命令执行成功 | ✅ 正常 |
| Session 记忆 | 6 条消息历史保存 | ✅ 正常 |
| Working Memory | 4 blocks | ✅ 正常 |
| WebSocket | 连接失败 | ❌ 失败 |
| MCP Server | 启动失败 | ❌ 失败 |
| Skills | 未配置 | ⚠️ 未配置 |

---

## 二、自主智能体必备功能 (根据调研)

### 2.1 工具集 (38+ 项)

| 类别 | 必需工具 | 当前 | 差距 |
|------|----------|------|------|
| 文件系统 | read, write, edit, glob, grep, find | 部分 | 缺 3 |
| Shell | bash | ✅ | - |
| 网络 | web_search, fetch_url, http_request | ❌ | 缺 3 |
| 浏览器 | browser_navigate, click, type | ❌ | 缺 3 |
| 代码执行 | python, javascript | ❌ | 缺 2 |
| 搜索 | search, vector_search | ❌ | 缺 2 |
| Git | git_clone, git_commit, git_push | ❌ | 缺 3 |

---

## 三、v1.0 完整功能清单

### 3.1 核心引擎

| 功能 | 当前 | 目标 | 优先级 |
|------|------|------|--------|
| 工具扩展 | 8 | 25+ | P0 |
| Semantic Memory | 无 | 知识图谱 | P0 |
| Loop 增强 | 10 轮 | 30 轮 | P0 |

### 3.2 MCP 集成

| 功能 | 当前 | 目标 | 优先级 |
|------|------|------|--------|
| MCP 启动修复 | ❌ | 可用 | P0 |
| filesystem 验证 | 未 | 跑通 | P0 |
| fetch 验证 | 未 | 跑通 | P1 |
| sqlite 验证 | 未 | 跑通 | P1 |

### 3.3 Skills 系统

| 功能 | 当前 | 目标 | 优先级 |
|------|------|------|--------|
| Skills 加载 | 空 | ≥3 个 | P0 |
| Skill: 代码调试 | 无 | 具体实现 | P0 |
| Skill: Git 操作 | 无 | 具体实现 | P1 |
| Skill: 文件处理 | 无 | 具体实现 | P1 |

---

## 四、完整测试案例 (v1.0)

### 4.1 测试数据准备

#### 4.1.1 测试文件结构

```
/tmp/octo-test/
├── README.md                    # 测试用 README
├── hello.py                     # Python 文件
├── hello.js                     # JS 文件
├── data/
│   └── config.json             # JSON 配置
└── logs/
    └── app.log                 # 日志文件
```

#### 4.1.2 测试 Skills (3 个)

**Skill 1: Code Debugger**
```
skills/code-debugger/SKILL.md:
---
name: code-debugger
description: 帮助调试代码问题，提供错误分析和修复建议
capabilities: [FileRead, ShellExec]
triggers: [debug, error, fix, bug]
---

# Code Debugger Skill

## 触发条件
用户请求调试代码、修复错误时触发。

## 执行步骤
1. 读取相关代码文件
2. 分析错误信息
3. 提供修复建议
4. 可选择执行修复命令
```

**Skill 2: Git Helper**
```
skills/git-helper/SKILL.md:
---
name: git-helper
description: Git 操作辅助，提供提交、推送、拉取等帮助
capabilities: [ShellExec]
triggers: [git commit, git push, git pull, git branch]
---

# Git Helper Skill

## 触发条件
用户请求 Git 操作时触发。

## 执行步骤
1. 解析 Git 命令
2. 执行 git 操作
3. 返回操作结果
```

**Skill 3: File Organizer**
```
skills/file-organizer/SKILL.md:
---
name: file-organizer
description: 文件整理助手，帮助归类和整理文件
capabilities: [FileRead, FileWrite]
triggers: [organize, sort,整理]
---

# File Organizer Skill

## 触发条件
用户请求文件整理时触发。

## 执行步骤
1. 扫描目标目录
2. 按类型分类
3. 生成整理报告
```

#### 4.1.3 测试 MCP Servers (6 个 - 主流推荐)

根据 GitHub Stars 和社区调研，选取最流行的 MCP 服务器:

**MCP 1: Filesystem** (⭐⭐⭐⭐⭐ 必备)
- command: `npx -y @modelcontextprotocol/server-filesystem /tmp/octo-test`
- tools: read_file, write_file, list_directory, create_directory, move_file

**MCP 2: Fetch** (⭐⭐⭐⭐⭐ 必备)
- command: `npx -y @modelcontextprotocol/server-fetch`
- tools: fetch (获取网页内容)

**MCP 3: SQLite** (⭐⭐⭐⭐ 常用)
- command: `npx -y @modelcontextprotocol/server-sqlite /tmp/octo-test/test.db`
- tools: execute, list_tables

**MCP 4: GitHub** (⭐⭐⭐⭐ 企业必备)
- command: `npx -y @modelcontextprotocol/server-github`
- tools: list_issues, create_issue, list_prs, search_code (需要 token)

**MCP 5: Notion** (⭐⭐⭐⭐ 知识管理)
- command: `npx -y @notionhq/notion-mcp-server`
- tools: search, get_page, create_page (需要 token)

**MCP 6: Brave Search** (⭐⭐⭐⭐ Web 搜索)
- command: `npx -y @modelcontextprotocol/server-brave-search`
- tools: search (需要 API key)

---

#### 4.1.4 测试 Skills (6 个 - 主流推荐)

根据 awesome-agent-skills 和 OpenFang 实践:

| # | Skill | 描述 | Capabilities | Triggers | 优先级 |
|---|-------|------|--------------|----------|--------|
| 1 | code-debugger | 代码调试助手 | FileRead, ShellExec | debug, error, fix | P0 |
| 2 | git-helper | Git 操作 | ShellExec | git commit, push | P0 |
| 3 | readme-writer | README 生成 | FileRead, FileWrite | readme, 文档 | P1 |
| 4 | test-generator | 测试生成 | FileRead, FileWrite | test, 测试 | P1 |
| 5 | code-review | 代码审查 | FileRead | review, 审查 | P1 |
| 6 | file-organizer | 文件整理 | FileRead, FileWrite | organize, sort | P2 |

---

### 4.2 测试案例矩阵 (33 个)

| # | 测试案例 | 覆盖功能 | 测试数据 | 预期结果 |
|---|----------|----------|----------|----------|
| T01 | 基础对话 | AI对话, Session记忆 | "你好，帮我列出文件" | 返回文件列表，记忆保存 |
| T02 | Bash工具-文件操作 | bash, ls, cd | "ls /tmp" | 列出 /tmp 目录内容 |
| T03 | 文件读取 | file_read | "读取 /tmp/octo-test/README.md" | 返回文件内容 |
| T04 | 文件写入 | file_write | "写入 hello.txt 到 /tmp" | 文件创建成功 |
| T05 | 文件编辑 | file_edit | 修改已有文件内容 | 内容更新成功 |
| T06 | Grep搜索 | grep | "搜索 function 关键字" | 返回匹配行 |
| T07 | Glob模式匹配 | glob | "找所有 .py 文件" | 返回文件列表 |
| T08 | Find查找 | find | "找 /tmp 下的目录" | 返回目录列表 |
| T09 | 连续工具调用 | bash→read→grep | "找到 py 文件并搜索关键字" | 多步骤成功 |
| T10 | MCP-filesystem | MCP, list_tools | 启动 filesystem MCP | 列出目录成功 |
| T11 | MCP-fetch | MCP, web_fetch | "获取 http://example.com" | 返回网页内容 |
| T12 | MCP-sqlite | MCP, sqlite | "创建表并查询" | 返回查询结果 |
| T13 | MCP-github | MCP, GitHub API | "列出仓库 issues" | 返回 issues 列表 |
| T14 | MCP-brave-search | MCP, web search | "搜索 AI 最新新闻" | 返回搜索结果 |
| T15 | Skill加载 | SkillLoader | 配置 skills 目录 | 6 个 Skill 加载 |
| T16 | Skill触发-CodeDebugger | Skill执行 | "帮我调试这段代码" | Skill 执行 |
| T17 | Skill触发-GitHelper | Skill执行 | "git commit -m 'fix'" | Git 操作执行 |
| T18 | Skill触发-ReadmeWriter | Skill执行 | "生成 README" | 生成文档 |
| T19 | memory_store | 记忆存储 | "记住我的名字是小明" | 存储成功 |
| T20 | memory_recall | 记忆检索 | "我叫什么？" | 召回"小明" |
| T21 | memory_search | 记忆搜索 | "搜索之前的项目" | 返回相关记忆 |
| T22 | memory_forget | 记忆删除 | "忘记那个名字" | 删除成功 |
| T23 | Session持久化 | SQLite | 重启服务器 | Session 恢复 |
| T24 | LoopGuard-重复检测 | 循环防护 | 重复调用同一工具 5 次 | 触发警告 |
| T25 | LoopGuard-乒乓检测 | 乒乓防护 | A→B→A→B 循环 | 触发警告 |
| T26 | Context-70%阈值 | 上下文管理 | 超过 70% token | 触发软修剪 |
| T27 | Context-90%阈值 | 上下文管理 | 超过 90% token | 触发硬清理 |
| T28 | LLM错误-重试 | 错误处理 | 模拟 rate limit | 自动重试成功 |
| T29 | LLM错误-不可重试 | 错误处理 | 模拟 auth 错误 | 返回明确错误 |
| T30 | TokenBudget显示 | Debug面板 | 对话后 | 显示消耗百分比 |
| T31 | ToolExecution记录 | 执行记录 | 执行工具后 | Tools页显示记录 |
| T32 | 30轮对话 | 长期对话 | 连续 30 轮对话 | 不中断响应 |
| T33 | WebSocket重连 | 连接管理 | 断开连接后 | 自动重连成功 |

---

### 4.3 详细测试案例

#### T01: 基础对话测试

**测试数据**:
```
输入: "你好，帮我列出 /tmp 目录的文件"
```

**验证点**:
- [ ] AI 响应正常
- [ ] 执行了 bash ls 命令
- [ ] Session 记忆保存了对话

**预期输出**:
```
AI: 你好！让我帮你列出 /tmp 目录的文件。
[执行工具: bash ls /tmp]
[返回文件列表]
```

---

#### T10: MCP-Filesystem 测试

**前置条件**:
1. 启动 MCP server: `npx -y @modelcontextprotocol/server-filesystem /tmp/octo-test`

**测试数据**:
```
输入: "列出 octo-test 目录下的所有文件"
```

**验证点**:
- [ ] MCP server 状态为 running
- [ ] list_tools 返回工具列表
- [ ] 调用 tool 成功返回结果

**预期输出**:
```
MCP: filesystem
- tools: [read_file, write_file, list_directory, ...]
[调用 tool 成功]
```

---

#### T14: Skill-CodeDebugger 测试

**前置条件**:
1. 配置 skills 目录包含 code-debugger
2. 放置测试文件 `/tmp/octo-test/hello.py`

**测试数据**:
```
输入: "帮我调试 hello.py 文件中的错误"
内容:
def add(a, b)
    return a + b

print(add(1,2))
```

**验证点**:
- [ ] Skill 被加载
- [ ] Skill 被触发
- [ ] 执行了相关工具 (file_read)
- [ ] 返回调试建议

**预期输出**:
```
[检测到调试请求]
[Skill: code-debugger 触发]
[读取文件 /tmp/octo-test/hello.py]
[分析错误: 缺少冒号]
[返回修复建议: 在 return 语句前加冒号]
```

---

#### T17: Memory-Recall 测试

**前置条件**:
1. 执行 T16 (memory_store)

**测试数据**:
```
输入: "我的名字是什么？"
```

**验证点**:
- [ ] 从 Session 层检索
- [ ] 返回 "小明"
- [ ] 显示来源标记

**预期输出**:
```
[从 Session Memory 检索]
找到: user_profile.name = "小明"
AI: 你的名字是小明。
```

---

#### T21: LoopGuard-重复检测测试

**测试数据**:
```
连续输入:
1. "读取 /tmp/file1.txt"
2. "读取 /tmp/file1.txt"
3. "读取 /tmp/file1.txt"
4. "读取 /tmp/file1.txt"
5. "读取 /tmp/file1.txt"
```

**验证点**:
- [ ] 第 5 次调用前触发警告
- [ ] 返回 LoopGuardViolation 错误
- [ ] 阻止执行

**预期输出**:
```
[警告] 检测到重复调用: file_read 重复 5 次
[阻止执行] LoopGuard: RepetitiveCall
```

---

#### T25: LLM错误重试测试

**测试数据**:
```
模拟: Anthropic API 返回 429 (rate_limit)
```

**验证点**:
- [ ] 捕获 RateLimitError
- [ ] 触发指数退避重试
- [ ] 最终成功或重试耗尽

**预期输出**:
```
[错误] RateLimitError: Too many requests
[重试] 等待 1s...
[重试] 等待 2s...
[重试] 等待 4s...
[成功] 返回正常响应
```

---

### 4.4 Skills 详细规格

#### Skill 1: code-debugger

```yaml
name: code-debugger
version: 1.0.0
description: 代码调试助手，帮助分析错误并提供修复建议
capabilities:
  - FileRead
  - ShellExec
triggers:
  - debug
  - error
  - fix
  - bug
  - 调试
  - 错误
  - 修复

# 输入
input:
  - name: code
    type: string
    required: false
    description: 要调试的代码片段
  - name: error
    type: string
    required: false
    description: 错误信息

# 输出
output:
  - name: analysis
    type: string
    description: 错误分析
  - name: fix
    type: string
    description: 修复建议

# 执行步骤
steps:
  1. 读取相关代码文件 (如果提供了路径)
  2. 分析错误信息
  3. 定位问题行
  4. 生成修复建议
  5. 可选: 应用修复 (需要额外确认)
```

#### Skill 2: git-helper

```yaml
name: git-helper
version: 1.0.0
description: Git 操作助手
capabilities:
  - ShellExec
triggers:
  - git commit
  - git push
  - git pull
  - git branch
  - git status

# 输入
input:
  - name: command
    type: string
    required: true
    description: Git 命令 (不含 git 前缀)

# 输出
output:
  - name: result
    type: string
    description: 命令执行结果

# 执行步骤
steps:
  1. 解析命令
  2. 验证安全性 (防止危险命令)
  3. 执行 git 命令
  4. 返回结果
```

#### Skill 3: file-organizer

```yaml
name: file-organizer
version: 1.0.0
description: 文件整理助手
capabilities:
  - FileRead
  - FileWrite
triggers:
  - organize
  - sort
  - 整理
  - 分类

# 输入
input:
  - name: directory
    type: string
    required: true
    description: 要整理的目录路径

# 输出
output:
  - name: report
    type: string
    description: 整理报告

# 执行步骤
steps:
  1. 扫描目录
  2. 按扩展名分类
  3. 生成分类报告
  4. 可选: 执行移动 (需要确认)
```

---

### 4.5 MCP Servers 详细规格

#### MCP 1: Filesystem Server

```yaml
name: filesystem
transport: stdio
command: npx
args: ["-y", "@modelcontextprotocol/server-filesystem", "/tmp/octo-test"]
enabled: true

# 提供的工具
tools:
  - name: read_file
    description: Read contents of a file
    input_schema:
      path: string
  - name: write_file
    description: Write contents to a file
    input_schema:
      path: string
      content: string
  - name: list_directory
    description: List files in a directory
    input_schema:
      path: string
  - name: create_directory
    description: Create a directory
    input_schema:
      path: string
  - name: move_file
    description: Move/rename a file
    input_schema:
      source: string
      destination: string
```

#### MCP 2: Fetch Server

```yaml
name: fetch
transport: stdio
command: npx
args: ["-y", "@modelcontextprotocol/server-fetch"]
enabled: false

# 提供的工具
tools:
  - name: fetch
    description: Fetch a URL and get its contents
    input_schema:
      url: string
      max_length: number
      timeout: number
```

#### MCP 3: SQLite Server

```yaml
name: sqlite
transport: stdio
command: npx
args: ["-y", "@modelcontextprotocol/server-sqlite", "/tmp/octo-test/test.db"]
enabled: false

# 提供的工具
tools:
  - name: execute
    description: Execute a SQL query
    input_schema:
      sql: string
  - name: list_tables
    description: List all tables
    input_schema: {}
```

---

## 五、实施计划 (含测试)

### 阶段 A: 阻塞问题修复 + 测试准备 (2 天)

```
Day 1:
├── P0.1 WebSocket 连接修复
├── P0.2 MCP Server 启动修复
└── 测试数据准备:
    ├── /tmp/octo-test/ 目录
    ├── 3 个 Skill 文件
    └── 3 个 MCP 配置

Day 2:
├── P0.3 Skills 配置 (3 个)
└── T01-T03 基础测试
```

### 阶段 B: 核心功能 + 测试 (5 天)

```
Day 3:
├── T04-T09 工具测试
└── T10-T12 MCP 测试 (filesystem, fetch, sqlite)

Day 4:
├── T13-T15 Skills 测试
└── T16-T19 Memory 测试

Day 5:
├── T20-T22 安全测试 (LoopGuard)
└── T23-T26 错误处理测试

Day 6-7:
├── T27-T30 调试面板 + 稳定性测试
└── Bug 修复
```

### 阶段 C: 完善与发布 (2 天)

```
Day 8-9:
├── 补测失败案例
├── 文档完善
└── v1.0 Release
```

---

## 六、测试验收标准

### 6.1 通过标准

| 类别 | 测试数 | 通过率 | 说明 |
|------|--------|--------|------|
| 基础功能 | T01-T09 | 100% | 必须全部通过 |
| MCP | T10-T14 | 100% | 必须全部通过 |
| Skills | T15-T18 | 100% | 必须全部通过 |
| Memory | T19-T22 | 100% | 必须全部通过 |
| 安全 | T23-T29 | 100% | 必须全部通过 |
| 调试 | T30-T33 | 100% | 必须全部通过 |
| **总计** | **33** | **100%** | |

### 6.2 失败处理

- **P0 失败**: 阻塞发布，必须修复
- **P1 失败**: 影响体验，优先修复
- **P2 失败**: 记录为已知限制

---

## 七、下一步

确认方案后开始实施。

测试案例是否完整？需要补充哪些具体测试？
