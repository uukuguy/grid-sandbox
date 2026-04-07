# Phase BG — Enterprise SDK 基石 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the foundation layer (S1) of the EAASP Enterprise SDK — specs, models, authoring tools, sandbox adapters, and CLI — so enterprise developers can create, validate, and test Skills.

**Architecture:** Python SDK (`eaasp-sdk`) with JSON Schema specs as cross-language source of truth. Authoring tools parse/validate/generate SKILL.md files. Sandbox adapters connect to Grid product matrix (grid-cli, grid-runtime gRPC, multi-runtime comparison) for testing. CLI wraps everything into `eaasp init/validate/test/submit` commands.

**Tech Stack:** Python 3.12+, Pydantic v2, PyYAML, click, rich, grpcio (optional), httpx (optional), pytest + pytest-asyncio.

> **创建日期**: 2026-04-07
> **分支**: Grid
> **基线**: Phase BF 完成 @ ff5ad56（L2 统一资产层 + L1 抽象机制）
> **设计蓝图**: `docs/design/Grid/EAASP_SDK_DESIGN.md`
> **目标**: 构建 EAASP Enterprise SDK 的基石层（S1），让企业开发者可以创作、校验、推演 Skill

---

## 一、目标与范围

### 1.1 Phase BG 交付物

1. `sdk/specs/` — 7 个抽象概念的 JSON Schema（跨语言契约）
2. `sdk/python/` — eaasp-sdk Python 包（models + authoring + sandbox + CLI）
3. 创作工具链 — Skill 解析、校验、脚手架、Hook 生成
4. 沙盒推演 — GridCliSandbox + RuntimeSandbox + MultiRuntimeSandbox
5. CLI — `eaasp init / validate / test / compare / submit`
6. 企业场景示例 — 1 个 HR 入职 workflow-skill
7. 设计文档 — SDK 演进蓝图 + BG 实施方案

### 1.2 不在 BG 范围

- Policy DSL（需 L3 策略编译器，BH）
- Playbook 编排引擎（需 L4 事件总线，BI）
- 5 个 REST API 完整客户端（需 5 API 契约，BH）
- TypeScript SDK（需 Python SDK 稳定）
- GridServerSandbox（需 grid-server Skill API 补全）
- PlatformSandbox（需 L4 API 网关）

---

## 二、Wave 分解

### W1: specs/ + Python 模型

**目标**: 建立 7 个抽象概念的跨语言规范和 Python 数据模型

**产出**:
- `sdk/specs/` — 7 个 JSON Schema 文件
- `sdk/python/src/eaasp/models/` — 7 个 Pydantic v2 模型文件
- `sdk/python/pyproject.toml` — 包配置（包名 `eaasp-sdk`）
- `sdk/python/src/eaasp/__init__.py` — 顶层导出

**关键实现**:
- JSON Schema 定义字段类型、必填/可选、枚举值、嵌套结构
- Pydantic 模型与 Schema 严格对齐
- `Skill.to_skill_md()` / `Skill.from_skill_md()` 双向序列化
- 与 proto `SessionPayload` / `ResponseChunk` 等类型对齐但不暴露 gRPC

**测试**: ~15 tests
- 模型创建、序列化、反序列化
- Schema 验证
- Skill 与 SKILL.md 双向转换

---

### W2: authoring 创作工具链

**目标**: Skill 创作全流程工具：解析 → 校验 → 脚手架 → Hook 生成

**产出**:
- `sdk/python/src/eaasp/authoring/skill_parser.py` — SKILL.md 双向解析器
- `sdk/python/src/eaasp/authoring/skill_validator.py` — 多层校验器（8 条规则）
- `sdk/python/src/eaasp/authoring/skill_scaffold.py` — 脚手架生成（4 种模板）
- `sdk/python/src/eaasp/authoring/hook_builder.py` — Hook handler 脚本生成

**校验规则**:
1. frontmatter 结构完整性（必填字段非空）
2. hook event 合法性（仅 PreToolUse / PostToolUse / Stop）
3. handler_type 合法性（command / http / prompt / agent）
4. 依赖 ID 格式（`org/name` 格式）
5. scope 层级合法性（global / bu / dept / team）
6. prose 非空且有实质内容（>50 字符）
7. 运行时亲和性 × hook handler 兼容性
8. skill_type × hook 组合合理性

**脚手架模板**:
- workflow: 带 Stop hook（完成性校验）
- production: 带 PostToolUse command hook（输出格式校验）
- domain: 带 PreToolUse prompt hook（合规检查）
- meta: 带 agent hook（深度评估）

**测试**: ~15 tests
- 解析合法/非法 SKILL.md
- 校验通过/失败的各种场景
- 脚手架生成 + 文件结构验证
- Hook 脚本生成 + 格式验证

---

### W3: sandbox 核心 + GridCliSandbox

**目标**: 沙盒推演抽象 + 本地 grid-cli 推演后端

**产出**:
- `sdk/python/src/eaasp/sandbox/base.py` — SandboxAdapter ABC + 结果模型
- `sdk/python/src/eaasp/sandbox/grid_cli.py` — GridCliSandbox 实现

**GridCliSandbox 实现要点**:
- 通过 `subprocess` 调用 `grid` binary
- 将 Skill 内容和 SessionConfig 通过临时文件传入
- 解析 grid-cli 输出为 `TelemetrySummary`
- 支持流式输出（逐行读取 stdout）
- 自动检测 grid binary 是否可用

**测试**: ~5 tests
- SandboxAdapter 接口合约
- GridCliSandbox 子进程调用 mock
- 输出解析
- 错误处理（binary 不存在、超时）

---

### W4: RuntimeSandbox + MultiRuntimeSandbox

**目标**: gRPC 直连 L1 运行时 + 跨运行时对比推演

**产出**:
- `sdk/python/src/eaasp/sandbox/runtime.py` — RuntimeSandbox（gRPC 客户端）
- `sdk/python/src/eaasp/sandbox/multi_runtime.py` — MultiRuntimeSandbox（并行对比）

**RuntimeSandbox 实现要点**:
- 使用 `grpcio` 连接 L1 Runtime 的 gRPC 端口
- 复用 proto 生成的 Python stubs（`proto/eaasp/runtime/v1/`）
- 实现 initialize → send(stream) → terminate 完整流程
- 将 proto ResponseChunk 映射为 SDK ResponseChunk

**MultiRuntimeSandbox 实现要点**:
- 并行 asyncio.gather 多个 RuntimeSandbox
- 收集各运行时的 TelemetrySummary
- 生成 ConsistencyReport（工具调用差异、hook 触发差异、完成状态对比）
- 复用 BF 盲盒对比的思路

**测试**: ~8 tests
- gRPC 连接（mock server）
- proto 类型映射
- 多运行时并行执行
- ConsistencyReport 生成逻辑
- 连接失败处理

---

### W5: CLI + submit + 示例

**目标**: 命令行工具 + L2 Skill Registry 提交 + 企业示例

**产出**:
- `sdk/python/src/eaasp/cli/` — CLI 5 个命令
- `sdk/examples/hr-onboarding/` — HR 入职示例 Skill
- `sdk/python/src/eaasp/client/skill_registry.py` — L2 Skill Registry 轻量客户端

**CLI 命令**:
```bash
eaasp init <name> [--type workflow]           # 创建 Skill 骨架
eaasp validate <path>                          # 校验 SKILL.md
eaasp test <path> [--sandbox local|grpc://]    # 推演
eaasp test <path> --compare <addrs>            # 多运行时对比
eaasp submit <path> --registry <url>           # 提交到 L2
```

**L2 客户端**: 仅实现 `POST /api/v1/skills`（submit_draft），复用 BF Skill Registry REST API。

**示例 Skill**:
```
sdk/examples/hr-onboarding/
├── SKILL.md                # 完整的 workflow-skill
├── hooks/
│   └── check_pii.py       # command handler: PII 检查
└── tests/
    └── test_cases.jsonl    # 3 个测试用例
```

**测试**: ~7 tests
- CLI 命令 smoke test（--help、基本调用）
- init 生成目录结构验证
- validate 输出格式验证
- submit HTTP 调用 mock
- 示例 Skill 校验通过

---

### W6: 设计文档 + Makefile + 收尾

**目标**: 文档收尾、构建目标、ROADMAP 更新

**产出**:
- `docs/design/Grid/EAASP_SDK_DESIGN.md` — ✅ 已完成
- `docs/plans/2026-04-07-phase-bg-enterprise-sdk.md` — ✅ 本文件
- Makefile 新增 targets:
  ```makefile
  sdk-setup:    # uv pip install -e sdk/python
  sdk-test:     # pytest sdk/python/tests
  sdk-validate: # eaasp validate sdk/examples/hr-onboarding/
  sdk-build:    # python -m build sdk/python
  ```
- EAASP_ROADMAP.md 更新 BG 状态
- NEXT_SESSION_GUIDE.md 更新

**测试**: —（文档无测试）

---

## 三、测试策略

**总测试目标**: ~50 tests

| Wave | 模块 | 测试数 | 类型 |
|------|------|--------|------|
| W1 | models | ~15 | 单元测试（模型序列化/反序列化） |
| W2 | authoring | ~15 | 单元测试（解析、校验、生成） |
| W3 | sandbox/grid_cli | ~5 | 单元测试 + mock subprocess |
| W4 | sandbox/runtime | ~8 | 单元测试 + mock gRPC |
| W5 | cli + submit | ~7 | 集成测试（CLI smoke + HTTP mock） |

**测试框架**: pytest + pytest-asyncio
**测试位置**: `sdk/python/tests/`
**运行命令**: `cd sdk/python && pytest -xvs`

---

## 四、依赖矩阵

### 外部依赖

| 包 | 版本 | 用途 | 必需/可选 |
|---|------|------|---------|
| pydantic | >=2.0 | 数据模型 | 必需 |
| pyyaml | >=6.0 | YAML frontmatter 解析 | 必需 |
| jsonschema | >=4.0 | JSON Schema 校验 | 必需 |
| click | >=8.0 | CLI 框架 | 必需（cli 子包） |
| rich | >=13.0 | CLI 输出美化 | 必需（cli 子包） |
| grpcio | >=1.60 | RuntimeSandbox | 可选（sandbox 子包） |
| grpcio-tools | >=1.60 | proto Python stubs | 可选（开发时） |
| httpx | >=0.27 | L2 Registry 客户端 | 可选（submit 命令） |

### 内部依赖

| 组件 | SDK 如何使用 | 已就绪 |
|------|-----------|--------|
| proto stubs | RuntimeSandbox 使用 | ✅ `lang/claude-code-runtime-python/` 已有生成方式 |
| grid-cli binary | GridCliSandbox 调用 | ✅ `make build-cli` |
| grid-runtime gRPC | RuntimeSandbox 连接 | ✅ port 50051 |
| claude-code-runtime gRPC | RuntimeSandbox 连接 | ✅ port 50052 |
| L2 Skill Registry REST | submit 命令调用 | ✅ port 8081 |
| certifier blindbox | MultiRuntimeSandbox 参考 | ✅ `tools/eaasp-certifier/src/blindbox.rs` |

---

## 五、Deferred Items

| ID | 内容 | 前置条件 | 目标阶段 |
|----|------|---------|---------|
| BG-D1 | Policy DSL（声明式策略语言） | L3 策略编译器 | S3 (BH) |
| BG-D2 | Playbook DSL（多 Skill 编排） | L4 事件总线 | S4 (BI) |
| BG-D3 | 5 个 L3/L4 REST API 完整客户端 | 5 个 API 契约 | S5 (BH+) |
| BG-D4 | TypeScript SDK | Python SDK 稳定 | S6 (BI/BJ) |
| BG-D5 | GridServerSandbox（HTTP/WS） | grid-server Skill API | S2 (BG/BH) |
| BG-D6 | PlatformSandbox（L4 API 网关） | L4 API 网关 | S5 (BH+) |
| BG-D7 | MCP Tool 封装辅助 | L2 MCP Orchestrator 成熟 | S7 (BJ+) |
| BG-D8 | `eaasp promote` 命令 | L3 RBAC 角色体系 | S3 (BH) |
| BG-D9 | Skill 依赖解析（本地+远程） | L2 依赖解析器 | S2 (BH) |
| BG-D10 | Java/Go/C# SDK | 平台开放 API | S7 (BJ+) |

---

## 六、关键设计决策

| ID | 决策 | 理由 |
|----|------|------|
| BG-KD1 | Python SDK 先行 | AI 生态最成熟 |
| BG-KD2 | specs/ JSON Schema 是跨语言源头 | 避免多语言模型不一致 |
| BG-KD3 | SDK 不内嵌运行模拟器 | 规范反模式"双重抽象" |
| BG-KD4 | sandbox 支持 gRPC 直连 L1 | 跨运行时 Skill 可移植性 |
| BG-KD5 | 核心零运行时依赖 | authoring 纯离线 |
| BG-KD6 | CLI 基于 click + rich | Python 生态标配 |
| BG-KD7 | 包名 eaasp-sdk，import eaasp | 简短清晰 |
| BG-KD8 | 示例放 sdk/examples/ | 与代码同仓库，保持同步 |
| BG-KD9 | proto stubs 复用现有生成方式 | 不重复造 |
| BG-KD10 | 后续 SDK 在独立分支并行 | 里程碑式合流 |

---

## 七、验收标准

1. `pip install -e sdk/python` 成功安装
2. `eaasp init test-skill --type workflow` 生成正确的项目骨架
3. `eaasp validate sdk/examples/hr-onboarding/` 校验通过
4. `eaasp test sdk/examples/hr-onboarding/ --sandbox local` 可本地推演（需 grid binary）
5. `eaasp test ... --sandbox grpc://localhost:50051` 可连接 grid-runtime
6. `eaasp test ... --compare grpc://localhost:50051,grpc://localhost:50052` 可对比推演
7. `eaasp submit ... --registry http://localhost:8081` 可提交到 L2
8. `pytest sdk/python/tests/ -xvs` 全部通过（~50 tests）
9. JSON Schema 与 Pydantic 模型字段一一对应
10. 设计蓝图文档完整

---

## 八、详细实施步骤

> 以下为 TDD 风格的 bite-sized 实施步骤。每个 Task 对应一个 Wave。

---

### Task 1: W1 — Project Skeleton + JSON Schema + Pydantic Models

**Files:**
- Create: `sdk/python/pyproject.toml`
- Create: `sdk/python/src/eaasp/__init__.py`
- Create: `sdk/python/src/eaasp/models/__init__.py`
- Create: `sdk/python/src/eaasp/models/skill.py`
- Create: `sdk/python/src/eaasp/models/policy.py`
- Create: `sdk/python/src/eaasp/models/playbook.py`
- Create: `sdk/python/src/eaasp/models/tool.py`
- Create: `sdk/python/src/eaasp/models/message.py`
- Create: `sdk/python/src/eaasp/models/session.py`
- Create: `sdk/python/src/eaasp/models/agent.py`
- Create: `sdk/specs/skill.schema.json`
- Create: `sdk/specs/policy.schema.json`
- Create: `sdk/specs/playbook.schema.json`
- Create: `sdk/specs/tool.schema.json`
- Create: `sdk/specs/message.schema.json`
- Create: `sdk/specs/session.schema.json`
- Create: `sdk/specs/agent.schema.json`
- Test: `sdk/python/tests/test_models.py`

**Step 1: Create project skeleton**

Create `sdk/python/pyproject.toml`:
```toml
[project]
name = "eaasp-sdk"
version = "0.1.0"
description = "EAASP Enterprise SDK — create, validate, and test Skills"
requires-python = ">=3.12"
dependencies = [
    "pydantic>=2.0",
    "pyyaml>=6.0",
]

[project.optional-dependencies]
cli = ["click>=8.0", "rich>=13.0"]
grpc = ["grpcio>=1.60"]
submit = ["httpx>=0.27"]
dev = ["pytest>=8.0", "pytest-asyncio>=0.24", "jsonschema>=4.0"]
all = ["eaasp-sdk[cli,grpc,submit]"]

[project.scripts]
eaasp = "eaasp.cli.__main__:main"

[build-system]
requires = ["hatchling"]
build-backend = "hatchling.build"

[tool.hatch.build.targets.wheel]
packages = ["src/eaasp"]

[tool.pytest.ini_options]
asyncio_mode = "auto"
testpaths = ["tests"]
```

Create empty `__init__.py` files for package structure.

**Step 2: Write Skill model + JSON Schema**

`sdk/python/src/eaasp/models/skill.py` — Complete Skill model with:
- `ScopedHook(BaseModel)`: event (Literal), handler_type (Literal), config (dict), match (dict optional)
- `SkillFrontmatter(BaseModel)`: name, version, description, author, tags, skill_type (Literal 4 types), preferred_runtime, compatible_runtimes, hooks (list[ScopedHook]), dependencies (list[str]), scope (Literal 4 levels)
- `Skill(BaseModel)`: frontmatter (SkillFrontmatter), prose (str)
  - `to_skill_md() -> str`: render as SKILL.md (YAML frontmatter between `---` + prose)
  - `from_skill_md(content: str) -> Skill`: classmethod parse
  - `from_file(path: Path) -> Skill`: classmethod file reader

`sdk/specs/skill.schema.json` — JSON Schema matching Skill model exactly.

**Step 3: Write remaining 6 models + schemas**

- `policy.py` + `policy.schema.json`: PolicyRule, Policy
- `playbook.py` + `playbook.schema.json`: PlaybookStep, Playbook
- `tool.py` + `tool.schema.json`: ToolDef, McpServerConfig
- `message.py` + `message.schema.json`: UserMessage, ResponseChunk
- `session.py` + `session.schema.json`: SessionConfig, SessionState
- `agent.py` + `agent.schema.json`: AgentCapability

These are simpler models — straight Pydantic BaseModel with Literal enums.

**Step 4: Write `__init__.py` re-exports**

`sdk/python/src/eaasp/__init__.py`:
```python
from eaasp.models.skill import Skill, SkillFrontmatter, ScopedHook
from eaasp.models.policy import Policy, PolicyRule
from eaasp.models.playbook import Playbook, PlaybookStep
from eaasp.models.tool import ToolDef, McpServerConfig
from eaasp.models.message import UserMessage, ResponseChunk
from eaasp.models.session import SessionConfig, SessionState
from eaasp.models.agent import AgentCapability

__version__ = "0.1.0"
```

**Step 5: Write tests**

`sdk/python/tests/test_models.py` — ~15 tests:
```python
# Test Skill model creation, serialization, SKILL.md round-trip
# Test Skill.from_skill_md() with valid SKILL.md
# Test Skill.to_skill_md() produces valid format
# Test round-trip: from_skill_md(to_skill_md(skill)) == skill
# Test invalid frontmatter raises ValidationError
# Test ScopedHook validation (invalid event rejected)
# Test Policy model creation
# Test SessionConfig model creation
# Test ResponseChunk model creation
# Test AgentCapability with all tiers
# Test McpServerConfig with stdio and sse transports
# Test Skill.from_file() with temp file
# Test JSON Schema validation matches Pydantic validation
```

**Step 6: Run tests**

```bash
cd sdk/python && uv pip install -e ".[dev]" && pytest tests/test_models.py -xvs
```
Expected: ~15 PASS

**Step 7: Commit**

```bash
git add sdk/
git commit -m "feat(sdk): W1 — project skeleton + JSON Schema + Pydantic models (7 abstractions)"
```

---

### Task 2: W2 — Authoring Toolkit (Parser + Validator + Scaffold + HookBuilder)

**Files:**
- Create: `sdk/python/src/eaasp/authoring/__init__.py`
- Create: `sdk/python/src/eaasp/authoring/skill_parser.py`
- Create: `sdk/python/src/eaasp/authoring/skill_validator.py`
- Create: `sdk/python/src/eaasp/authoring/skill_scaffold.py`
- Create: `sdk/python/src/eaasp/authoring/hook_builder.py`
- Test: `sdk/python/tests/test_authoring.py`

**Step 1: Write SkillParser**

`skill_parser.py`:
- `parse(content: str) -> Skill`: Split on `---` delimiters, YAML parse frontmatter, rest is prose
- `render(skill: Skill) -> str`: YAML dump frontmatter between `---`, append prose
- `parse_file(path: Path) -> Skill`: Read file and call parse

**Step 2: Write SkillValidator**

`skill_validator.py`:
- `ValidationError(BaseModel)`: rule (str), message (str), severity ("error")
- `ValidationWarning(BaseModel)`: rule (str), message (str), severity ("warning")
- `ValidationResult(BaseModel)`: valid (bool), errors (list), warnings (list)
- `SkillValidator.validate(skill: Skill) -> ValidationResult`

8 validation rules:
1. Required fields: name, description, author non-empty
2. Hook events: only PreToolUse, PostToolUse, Stop
3. Handler types: only command, http, prompt, agent
4. Dependency format: must match `[a-z0-9-]+/[a-z0-9-]+` pattern
5. Scope: only global, bu, dept, team
6. Prose length: >50 characters
7. Affinity × handler: agent handler warns if preferred_runtime not set
8. Type × hook: workflow without Stop hook gets warning

**Step 3: Write SkillScaffold**

`skill_scaffold.py`:
- `create(name, skill_type="workflow", output_dir=Path(".")) -> Path`
- Creates: `{name}/SKILL.md` (from template), `{name}/hooks/` dir, `{name}/tests/test_cases.jsonl`
- 4 templates (workflow/production/domain/meta) with appropriate default hooks

**Step 4: Write HookBuilder**

`hook_builder.py`:
- `command_handler(name, event) -> str`: Generate Python script template (read stdin JSON, decide, print result)
- `http_handler(name, event) -> str`: Generate FastAPI endpoint template
- `prompt_handler(prompt) -> dict`: Return config dict for prompt handler

**Step 5: Write tests**

`sdk/python/tests/test_authoring.py` — ~15 tests:
```python
# SkillParser tests:
# - parse valid SKILL.md → correct Skill
# - parse SKILL.md with no hooks → empty hooks list
# - parse invalid YAML → raises error
# - render Skill → valid SKILL.md string
# - round-trip parse(render(skill)) identity

# SkillValidator tests:
# - valid skill → valid=True, no errors
# - empty name → error rule "required_fields"
# - invalid hook event → error rule "hook_event"
# - bad dependency format → error rule "dependency_format"
# - short prose → error rule "prose_length"
# - workflow without Stop hook → warning
# - all 8 rules pass for well-formed skill

# SkillScaffold tests:
# - create workflow scaffold → directory structure correct
# - SKILL.md in scaffold is parseable

# HookBuilder tests:
# - command_handler produces valid Python
# - prompt_handler returns correct dict structure
```

**Step 6: Run tests**

```bash
cd sdk/python && pytest tests/test_authoring.py -xvs
```
Expected: ~15 PASS

**Step 7: Commit**

```bash
git add sdk/python/src/eaasp/authoring/ sdk/python/tests/test_authoring.py
git commit -m "feat(sdk): W2 — authoring toolkit (parser + validator + scaffold + hook builder)"
```

---

### Task 3: W3 — Sandbox Core + GridCliSandbox

**Files:**
- Create: `sdk/python/src/eaasp/sandbox/__init__.py`
- Create: `sdk/python/src/eaasp/sandbox/base.py`
- Create: `sdk/python/src/eaasp/sandbox/grid_cli.py`
- Test: `sdk/python/tests/test_sandbox_cli.py`

**Step 1: Write SandboxAdapter ABC + result models**

`base.py`:
- `HookFiredEvent(BaseModel)`: event, hook_source, decision, tool_name, latency_ms
- `TelemetrySummary(BaseModel)`: session_id, total_turns, tools_called, hooks_fired, input/output_tokens, duration_ms, skill_loaded, completed_normally
- `SandboxAdapter(ABC)`: initialize, send (async iterator), terminate, validate_skill

**Step 2: Write GridCliSandbox**

`grid_cli.py`:
- `GridCliSandbox(SandboxAdapter)`:
  - `__init__(grid_bin="grid")`: store binary path
  - `initialize()`: Write Skill content + SessionConfig to temp files, start `grid` subprocess
  - `send()`: Write message to subprocess stdin, yield lines from stdout as ResponseChunk
  - `terminate()`: Send terminate signal, parse final output into TelemetrySummary
  - `validate_skill()`: Run `grid eval config --validate` with skill content
  - Helper: `_check_binary()` — verify grid binary exists in PATH

**Step 3: Write tests**

`sdk/python/tests/test_sandbox_cli.py` — ~5 tests:
```python
# - TelemetrySummary model creation
# - HookFiredEvent model creation
# - GridCliSandbox._check_binary() when binary missing → SandboxError
# - GridCliSandbox.initialize() with mocked subprocess → returns session_id
# - GridCliSandbox.terminate() parses mock output → TelemetrySummary
```

Use `unittest.mock.patch("subprocess.Popen")` to mock grid binary calls.

**Step 4: Run tests**

```bash
cd sdk/python && pytest tests/test_sandbox_cli.py -xvs
```
Expected: ~5 PASS

**Step 5: Commit**

```bash
git add sdk/python/src/eaasp/sandbox/ sdk/python/tests/test_sandbox_cli.py
git commit -m "feat(sdk): W3 — sandbox core + GridCliSandbox (local subprocess)"
```

---

### Task 4: W4 — RuntimeSandbox + MultiRuntimeSandbox

**Files:**
- Create: `sdk/python/src/eaasp/sandbox/runtime.py`
- Create: `sdk/python/src/eaasp/sandbox/multi_runtime.py`
- Create: `sdk/python/src/eaasp/_proto/` (symlink or copy from lang/claude-code-runtime-python)
- Test: `sdk/python/tests/test_sandbox_runtime.py`

**Step 1: Set up proto stubs access**

Two options (decide at implementation time):
- Option A: Symlink `sdk/python/src/eaasp/_proto/` → `lang/claude-code-runtime-python/src/claude_code_runtime/_proto/`
- Option B: Add proto compilation script in `sdk/python/scripts/compile_proto.sh` (same as claude-code-runtime's approach)

The stubs provide: `runtime_pb2`, `runtime_pb2_grpc`, `common_pb2`.

**Step 2: Write RuntimeSandbox**

`runtime.py`:
- `RuntimeSandbox(SandboxAdapter)`:
  - `__init__(endpoint: str)`: Parse `grpc://host:port`
  - `initialize()`: Create gRPC channel, call `RuntimeService.Initialize` with mapped SessionPayload
  - `send()`: Call `RuntimeService.Send` (server streaming), yield ResponseChunk from proto chunks
  - `terminate()`: Call `RuntimeService.Terminate`, map response to TelemetrySummary
  - `validate_skill()`: Call `RuntimeService.LoadSkill`, return ValidationResult

Mapping functions:
- `_to_proto_payload(config: SessionConfig, skills: list[Skill]) -> InitializeRequest`
- `_to_proto_message(msg: UserMessage) -> SendRequest`
- `_from_proto_chunk(chunk: proto.ResponseChunk) -> ResponseChunk`

**Step 3: Write MultiRuntimeSandbox**

`multi_runtime.py`:
- `ConsistencyReport(BaseModel)`: all_completed, tools_diff, hooks_diff, output_similarity
- `ComparisonResult(BaseModel)`: results (dict[str, TelemetrySummary]), consistency (ConsistencyReport)
- `MultiRuntimeSandbox`:
  - `__init__(endpoints: list[str])`
  - `compare(config, skill, message) -> ComparisonResult`:
    1. Create RuntimeSandbox for each endpoint
    2. `asyncio.gather` all initialize + send + terminate
    3. Compute ConsistencyReport from collected summaries
  - `_compute_consistency(summaries: dict) -> ConsistencyReport`:
    - `all_completed`: all TelemetrySummary.completed_normally
    - `tools_diff`: symmetric difference of tools_called sets
    - `hooks_diff`: differences in hooks_fired events
    - `output_similarity`: 1.0 if same tool sets, degrade by diff count

**Step 4: Write tests**

`sdk/python/tests/test_sandbox_runtime.py` — ~8 tests:
```python
# RuntimeSandbox:
# - _to_proto_payload maps SessionConfig correctly
# - _from_proto_chunk maps all chunk_types
# - initialize with mock channel returns session_id
# - send with mock stream yields ResponseChunks
# - connection failure → SandboxError

# MultiRuntimeSandbox:
# - _compute_consistency with identical summaries → all_completed=True, empty diffs
# - _compute_consistency with different tools → tools_diff populated
# - compare with 2 mock runtimes → ComparisonResult
```

**Step 5: Run tests**

```bash
cd sdk/python && pytest tests/test_sandbox_runtime.py -xvs
```
Expected: ~8 PASS

**Step 6: Commit**

```bash
git add sdk/python/src/eaasp/sandbox/runtime.py sdk/python/src/eaasp/sandbox/multi_runtime.py sdk/python/tests/test_sandbox_runtime.py
git commit -m "feat(sdk): W4 — RuntimeSandbox + MultiRuntimeSandbox (gRPC + cross-runtime compare)"
```

---

### Task 5: W5 — CLI + Submit + Example Skill

**Files:**
- Create: `sdk/python/src/eaasp/cli/__init__.py`
- Create: `sdk/python/src/eaasp/cli/__main__.py`
- Create: `sdk/python/src/eaasp/cli/init_cmd.py`
- Create: `sdk/python/src/eaasp/cli/validate_cmd.py`
- Create: `sdk/python/src/eaasp/cli/test_cmd.py`
- Create: `sdk/python/src/eaasp/cli/submit_cmd.py`
- Create: `sdk/python/src/eaasp/client/__init__.py`
- Create: `sdk/python/src/eaasp/client/skill_registry.py`
- Create: `sdk/examples/hr-onboarding/SKILL.md`
- Create: `sdk/examples/hr-onboarding/hooks/check_pii.py`
- Create: `sdk/examples/hr-onboarding/tests/test_cases.jsonl`
- Test: `sdk/python/tests/test_cli.py`

**Step 1: Write CLI entry point**

`__main__.py`:
```python
import click
from eaasp.cli.init_cmd import init_cmd
from eaasp.cli.validate_cmd import validate_cmd
from eaasp.cli.test_cmd import test_cmd
from eaasp.cli.submit_cmd import submit_cmd

@click.group()
@click.version_option(version="0.1.0")
def main():
    """EAASP Enterprise SDK — create, validate, and test Skills."""

main.add_command(init_cmd, "init")
main.add_command(validate_cmd, "validate")
main.add_command(test_cmd, "test")
main.add_command(submit_cmd, "submit")

if __name__ == "__main__":
    main()
```

**Step 2: Write init command**

`init_cmd.py`: Uses `SkillScaffold.create()` from authoring.

**Step 3: Write validate command**

`validate_cmd.py`: Uses `SkillParser.parse_file()` + `SkillValidator.validate()`, rich output.

**Step 4: Write test command**

`test_cmd.py`:
- `--sandbox local` → GridCliSandbox
- `--sandbox grpc://addr` → RuntimeSandbox
- `--compare addr1,addr2` → MultiRuntimeSandbox
- Default input: reads from stdin or `--input` flag

**Step 5: Write submit command + L2 client**

`submit_cmd.py` + `skill_registry.py`:
- `SkillRegistryClient.__init__(base_url: str)`
- `submit_draft(skill: Skill) -> dict`: POST to `/skills/draft` with JSON body matching `SubmitDraftRequest`:
  ```json
  {
    "id": "{frontmatter.name}",
    "name": "{frontmatter.name}",
    "description": "{frontmatter.description}",
    "version": "{frontmatter.version}",
    "author": "{frontmatter.author}",
    "tags": ["{frontmatter.tags}"],
    "frontmatter_yaml": "{yaml_dump(frontmatter)}",
    "prose": "{skill.prose}"
  }
  ```

**Step 6: Write HR onboarding example**

`sdk/examples/hr-onboarding/SKILL.md`:
```markdown
---
name: hr-onboarding
version: "1.0.0"
description: 新员工入职流程自动化
author: hr-team
tags: [hr, onboarding, workflow]
skill_type: workflow
preferred_runtime: grid
scope: bu
hooks:
  - event: PreToolUse
    handler_type: command
    config:
      command: "python hooks/check_pii.py"
      match:
        tool_name: file_write
  - event: Stop
    handler_type: prompt
    config:
      prompt: "验证入职清单是否全部完成，包括：IT账号、门禁、培训安排"
dependencies:
  - org/it-account-setup
  - org/badge-provisioning
---

你是一位经验丰富的 HR 专家，负责协助新员工完成入职流程。

## 工作流程

1. 收集新员工信息（姓名、部门、入职日期、直属上级）
2. 创建 IT 账号（调用 it-account-setup skill）
3. 申请门禁卡（调用 badge-provisioning skill）
4. 安排入职培训
5. 发送欢迎邮件

## 质量标准

- 所有个人信息必须经过 PII 检查后才能写入文件
- 入职清单 100% 完成才允许结束会话
- 每一步操作必须记录审计日志
```

`hooks/check_pii.py`: Example command handler that reads stdin, checks for PII patterns.

`tests/test_cases.jsonl`: 3 test cases with input/expected_output pairs.

**Step 7: Write tests**

`sdk/python/tests/test_cli.py` — ~7 tests:
```python
# - CLI --help returns 0
# - CLI --version shows 0.1.0
# - init creates directory structure (tmp dir)
# - validate on example skill → exit code 0
# - validate on broken skill → exit code 1
# - submit with mock httpx → correct POST body
# - example hr-onboarding/SKILL.md parses and validates
```

Use `click.testing.CliRunner` for CLI tests.

**Step 8: Run tests**

```bash
cd sdk/python && pip install -e ".[cli,submit,dev]" && pytest tests/test_cli.py -xvs
```
Expected: ~7 PASS

**Step 9: Commit**

```bash
git add sdk/python/src/eaasp/cli/ sdk/python/src/eaasp/client/ sdk/examples/ sdk/python/tests/test_cli.py
git commit -m "feat(sdk): W5 — CLI (init/validate/test/submit) + HR onboarding example"
```

---

### Task 6: W6 — Makefile + ROADMAP + Final Polish

**Files:**
- Modify: `Makefile`
- Modify: `docs/design/Grid/EAASP_ROADMAP.md`
- Modify: `docs/dev/NEXT_SESSION_GUIDE.md`
- Modify: `docs/plans/.checkpoint.json`

**Step 1: Add Makefile targets**

Add to `Makefile`:
```makefile
# ── EAASP SDK ──
sdk-setup:
	cd sdk/python && uv pip install -e ".[all,dev]"

sdk-test:
	cd sdk/python && pytest tests/ -xvs

sdk-validate:
	cd sdk/python && python -m eaasp.cli validate ../../sdk/examples/hr-onboarding/

sdk-build:
	cd sdk/python && python -m build
```

**Step 2: Update ROADMAP**

Update `docs/design/Grid/EAASP_ROADMAP.md` Phase BG section with actual status.

**Step 3: Run full SDK test suite**

```bash
cd sdk/python && pytest tests/ -xvs
```
Expected: ~50 PASS total across all test files.

**Step 4: Update checkpoint and session guide**

Mark all tasks W1-W6 as done in `.checkpoint.json`.

**Step 5: Commit**

```bash
git add Makefile docs/
git commit -m "docs: complete Phase BG — Enterprise SDK foundation (6/6, ~50 tests)"
```
