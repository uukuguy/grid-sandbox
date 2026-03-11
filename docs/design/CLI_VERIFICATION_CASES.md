# CLI 验证用例

> 本文档提供 Octo CLI 核心命令的验证用例，由用户在实际环境中手动执行。

---

## 前置条件

1. 已完成 Rust 构建：`cargo build -p octo-cli`
2. CLI 二进制位于 `target/debug/octo` 或已安装到 PATH
3. 当前目录存在 `.env` 文件（包含 `ANTHROPIC_API_KEY`）
4. 如使用 SQLite，确保 `./data/` 目录存在且可写

```bash
# 确认 CLI 可执行
./target/debug/octo --version
# 预期: octo <version>
```

---

## 验证命令清单

### 1. octo doctor

**命令**:
```bash
./target/debug/octo doctor
```

**预期输出**: 显示环境诊断信息，检查数据库连接、配置有效性等。

**成功判定**: 命令正常退出（exit code 0），输出包含检查项列表。

**带修复模式**:
```bash
./target/debug/octo doctor --repair
```

**预期**: 尝试自动修复可修复的问题（如创建缺失目录）。

---

### 2. octo config show

**命令**:
```bash
./target/debug/octo config show
```

**预期输出**: 显示当前生效的配置，包含以下字段：
- `server.host`
- `server.port`
- `provider.name`
- `database.path`
- `logging.level`

**成功判定**: 命令正常退出，输出包含上述配置字段。

**失败指标**: `Error` 或 `panic` 出现在输出中。

---

### 3. octo config validate

**命令**:
```bash
./target/debug/octo config validate
```

**预期输出**: 配置校验结果，显示各项检查通过/失败状态。

**成功判定**: 命令正常退出，输出指示配置有效。

---

### 4. octo config paths

**命令**:
```bash
./target/debug/octo config paths
```

**预期输出**: 显示配置文件搜索路径和当前使用的配置文件位置。

**成功判定**: 命令正常退出，输出包含文件路径信息。

---

### 5. octo agent list

**命令**:
```bash
./target/debug/octo agent list
```

**预期输出**: Agent 列表（可能为空列表或包含默认 agent）。

**成功判定**: 命令正常退出，输出为有效的列表格式（文本或 JSON）。

**JSON 格式**:
```bash
./target/debug/octo --output json agent list
```

---

### 6. octo session list

**命令**:
```bash
./target/debug/octo session list
```

**预期输出**: Session 列表（可能为空列表）。

**成功判定**: 命令正常退出，输出为有效的列表格式。

**限制结果数**:
```bash
./target/debug/octo session list --limit 5
```

---

### 7. octo tool list

**命令**:
```bash
./target/debug/octo tool list
```

**预期输出**: 内置工具列表，应包含以下工具：
- `bash` — 执行 shell 命令
- `file_read` — 读取文件
- `file_write` — 写入文件
- `file_edit` — 编辑文件
- `grep` — 搜索文件内容
- `glob` — 文件模式匹配
- `find` — 查找文件

**成功判定**: 命令正常退出，输出包含上述内置工具名称。

---

### 8. octo mcp list

**命令**:
```bash
./target/debug/octo mcp list
```

**预期输出**: MCP 服务器列表（初始可能为空）。

**成功判定**: 命令正常退出，输出为有效的列表格式。

---

### 9. octo memory list

**命令**:
```bash
./target/debug/octo memory list
```

**预期输出**: 记忆条目列表（初始可能为空）。

**成功判定**: 命令正常退出。

---

### 10. octo completions generate

**命令**:
```bash
./target/debug/octo completions generate zsh
```

**预期输出**: Zsh shell 补全脚本（大量 shell 代码）。

**成功判定**: 命令正常退出，输出包含 `#compdef` 或类似 shell 补全语法。

**其他 shell**:
```bash
./target/debug/octo completions generate bash
./target/debug/octo completions generate fish
```

---

### 11. octo --help

**命令**:
```bash
./target/debug/octo --help
```

**预期输出**: 命令帮助信息，列出所有可用子命令：
- `run` — 交互式 REPL
- `ask` — 单次查询
- `agent` — Agent 管理
- `session` — Session 管理
- `memory` — 记忆管理
- `tool` — 工具管理
- `mcp` — MCP 服务器管理
- `config` — 配置管理
- `doctor` — 健康诊断
- `tui` — 全屏 TUI 模式
- `completions` — Shell 补全
- `dashboard` — Web 仪表板

**成功判定**: 命令正常退出，输出包含上述所有子命令。

---

## 验证执行说明

### 执行顺序

建议按以下顺序执行验证：

1. `--help` — 确认 CLI 基本可用
2. `doctor` — 检查环境
3. `config show` / `config validate` — 确认配置
4. `agent list` / `session list` / `tool list` — 确认数据层
5. `mcp list` / `memory list` — 确认扩展功能
6. `completions generate` — 确认代码生成

### 结果记录模板

| 命令 | 预期 | 实际 | 状态 |
|------|------|------|------|
| `octo --help` | 显示帮助 | | PASS / FAIL |
| `octo doctor` | 诊断通过 | | PASS / FAIL |
| `octo config show` | 显示配置 | | PASS / FAIL |
| `octo config validate` | 配置有效 | | PASS / FAIL |
| `octo config paths` | 显示路径 | | PASS / FAIL |
| `octo agent list` | Agent 列表 | | PASS / FAIL |
| `octo session list` | Session 列表 | | PASS / FAIL |
| `octo tool list` | 工具列表 | | PASS / FAIL |
| `octo mcp list` | MCP 列表 | | PASS / FAIL |
| `octo memory list` | 记忆列表 | | PASS / FAIL |
| `octo completions generate zsh` | 补全脚本 | | PASS / FAIL |

### 反馈格式

请将以下信息反馈给开发团队：
1. 每个命令的完整输出（stdout + stderr）
2. 退出码（`echo $?`）
3. 如果失败，提供错误信息和运行环境信息（OS、Rust 版本等）
