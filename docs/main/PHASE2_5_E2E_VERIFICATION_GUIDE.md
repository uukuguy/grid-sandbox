# Phase 2.5 L1 Runtime Ecosystem — 人工 E2E 验证指南

> **⚠️ ARCHIVED (2026-04-17)** — 本文为 Phase 2.5 收尾一次性 artifact。
> **后续所有人工 E2E 验证请使用长期指南**: `docs/design/EAASP/E2E_VERIFICATION_GUIDE.md`
> Phase 2.5 收尾历史见该指南第 7 节。

---

> **执行时机**: Phase 2.5 自动化门控（`make v2-phase2_5-e2e` 合约套件 4 runtimes + `make goose-runtime-container-verify-f1`）通过后，准备进入 `/end-phase` 前。
> **目标**: 证明 Phase 2.5 关键能力（goose-runtime 容器基线 + nanobot-runtime 真实 agent loop + ADR-V2-019 部署模式 + ADR-V2-006 hook envelope 跨 runtime 一致）在真实 LLM + 真实 MCP 链路上端到端工作。
> **参考提交**: `844664d` (S2.T2+S3+S4.T1) / `1d6dca2` (checkpoint 24/25)
> **范围外**: goose-runtime 完整 agent loop ACP 接线 → Phase 3（当前 Send 为 stub）

---

## 一、Prerequisites

### 1.1 工作目录

```bash
cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox
```

### 1.2 `.env` 必需变量

```bash
# claude-code-runtime
ANTHROPIC_API_KEY=sk-ant-xxx
ANTHROPIC_BASE_URL=https://api.anthropic.com

# grid-runtime + nanobot-runtime 经 OpenRouter
OPENAI_API_KEY=sk-or-xxx
OPENAI_BASE_URL=https://openrouter.ai/api/v1
OPENAI_MODEL_NAME=<model-id>
LLM_PROVIDER=openai
```

### 1.3 CLI 别名

```bash
alias eaasp='/Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox/tools/eaasp-cli-v2/.venv/bin/eaasp'
```

### 1.4 L4 端口映射确认

`tools/eaasp-l4-orchestration/src/eaasp_l4_orchestration/l1_client.py` 中 `RUNTIME_ENDPOINTS` 已包含：

| runtime_id | 默认端口 | 启动方式 |
|---|---|---|
| `grid-runtime` | :50051 | `make dev-eaasp` 自带 |
| `claude-code-runtime` | :50052 | `make dev-eaasp` 自带 |
| `nanobot-runtime` | :50054 | 手动启动（见 Step 3） |
| `goose-runtime` | :50063 | 容器启动（见 Step 4） |

---

## 二、Execution Steps

### Step 1（首次，只跑一次）: 构建 goose-runtime 容器镜像

```bash
make goose-runtime-container-build
```

**预期**: Docker image `eaasp-goose-runtime:dev` 构建完成（5-10 分钟）。

> 若容器已构建可跳过。验证：`docker image inspect eaasp-goose-runtime:dev`。

---

### Step 2（Terminal A）: 启动 EAASP 全栈（含 4 个 runtime）

```bash
make dev-eaasp
```

**预期状态表**（脚本尾部）:

```
SERVICE                  PORT   PID    PROVIDER     STATUS
skill-registry           18081  ...    -            UP
L2 memory-engine         18085  ...    -            UP
L3 governance            18083  ...    -            UP
mock-scada(SSE)          18090  ...    tool-sandbox UP
L2 MCP Orchestrator      18082  ...    -            UP
grid-runtime             50051  ...    OPENAI_*     UP
claude-code-runtime      50052  ...    ANTHROPIC_*  UP
nanobot-runtime          50054  ...    OPENAI_*     UP
goose-runtime(docker)    50063  ...    OPENAI_*     UP
L4 orchestration         18084  ...    -            UP
```

**失败标志**:
- `port already in use` → 先 `make dev-eaasp-stop`
- `nanobot .venv missing` → `cd lang/nanobot-runtime-python && uv sync`
- `eaasp-goose-runtime:dev image not found` → 回 Step 1

> ⚠️ **goose-runtime Send 为 stub**：当前 `Send` RPC 返回单个 `done` chunk，完整 ACP 接线归 Phase 3。本阶段只验基线（Initialize/Terminate/Health + F1 容器 CLI）。

---

### Step 3（Terminal B）: 注册 skills（仅首次或变更后）

```bash
eaasp skill submit examples/skills/threshold-calibration
eaasp skill submit examples/skills/skill-extraction
```

**预期**: 两个 skill 返回 `registered: true`。

---

## 三、验证核心功能（Terminal B，续 Step 3）

### Test 1 — grid-runtime threshold-calibration（主力，ADR-V2-017）

```bash
eaasp session create --skill threshold-calibration --runtime grid-runtime
export SID_GRID='<session_id>'
eaasp session send $SID_GRID "校准 Transformer-001 的温度阈值"
eaasp session events $SID_GRID
```

**必须出现的事件**:

| Event Type | 来源 | 验证点 |
|---|---|---|
| `SESSION_CREATED` | L4 | Phase 0 |
| `RUNTIME_INITIALIZED` | L4 | Phase 0 |
| `SESSION_START` | interceptor | Phase 1 |
| `SESSION_MCP_CONNECTED` | Phase 0.75 | `mock-scada` + `eaasp-l2-memory` |
| `USER_MESSAGE` | L4 | — |
| `RESPONSE_CHUNK` × N | L4 | — |
| **`PRE_TOOL_USE` ≥ 4** | interceptor | **D87 多步 workflow 生效** |
| **`POST_TOOL_USE` ≥ 4** | interceptor | 与 PRE 配对 |
| `STOP` = 1 | interceptor | 干净终止 |

**关键验收断言**:
1. ✅ `PRE_TOOL_USE ≥ 4`（scada_read_snapshot + memory_search + memory_write_anchor + memory_write_file）
2. ✅ `scada_write` 被 `block_write_scada.sh` PreToolUse hook 拒绝（应不出现）
3. ✅ `require_anchor.sh` Stop hook 检查 evidence_anchor_id 通过
4. ✅ L2 memory 有新写入的 anchor + file（scope `org:eaasp-mvp`, category `threshold_calibration`）

---

### Test 2 — claude-code-runtime threshold-calibration（样板）

```bash
eaasp session create --skill threshold-calibration --runtime claude-code-runtime
export SID_CC='<session_id>'
eaasp session send $SID_CC "校准 Transformer-001 的温度阈值"
eaasp session events $SID_CC
```

**同 Test 1** 验收标准。证明 **ADR-V2-001 拦截器路径跨 runtime 一致**。

---

### Test 3 — nanobot-runtime threshold-calibration（Phase 2.5 新增 ★ 样板 OpenAI-compat）

```bash
eaasp session create --skill threshold-calibration --runtime nanobot-runtime
export SID_NB='<session_id>'
eaasp session send $SID_NB "校准 Transformer-001 的温度阈值"
eaasp session events $SID_NB
```

**验收标准（降级）**:
1. ✅ `SESSION_START` + `USER_MESSAGE` + `CHUNK` ≥ 1 + `STOP` = 1
2. ⚠️ `PRE_TOOL_USE ≥ 1` 为加分项（nanobot MCP 工具接线视 W2.T6 skill-extraction E2E smoke 实际情况）
3. ✅ 真实 LLM 回复文本（不是 stub done）

**证明**: nanobot 的 OpenAI-compat provider + agent loop + ADR-V2-006 PostToolUse hook envelope 跨语言（Python）一致性。

---

### Test 4 — goose-runtime 基线（Phase 2.5 新增 ★ 仅基线）

```bash
eaasp session create --skill threshold-calibration --runtime goose-runtime
export SID_GS='<session_id>'
eaasp session send $SID_GS "校准 Transformer-001 的温度阈值"
eaasp session events $SID_GS
```

**基线验收（不要求 agent loop）**:
1. ✅ `SESSION_CREATED` + `RUNTIME_INITIALIZED` 通
2. ✅ `SESSION_START` + `USER_MESSAGE` + `STOP` 事件链完整
3. ⚠️ `RESPONSE_CHUNK` 最多 1 条（`done`）— 这是 stub 预期
4. ❌ 不要求 `PRE_TOOL_USE` — Send stub 不调工具

**额外基线（F1 gate，已通过）**:

```bash
make goose-runtime-container-verify-f1
# 期望: docker exec goose info 退出码 0
```

---

### Test 5 — skill-extraction meta-skill（Phase 2.S3.T2-T3 能力）

**目的**: 验证 skill-extraction 这个 Phase 2 新增的 meta-skill 真实可用，而不仅仅是合约测试里的 fixture replay。

```bash
# 接 Test 1 的 SID_GRID（已有 threshold-calibration 的 memory）
eaasp session create --skill skill-extraction --runtime grid-runtime
export SID_EX='<session_id>'
eaasp session send $SID_EX "从 Transformer-001 阈值校准的会话记忆中，抽取一个可复用的 skill 草稿"
eaasp session events $SID_EX
```

**验收**:
1. ✅ 事件流含 `PRE_TOOL_USE` for `memory_search` + `memory_read` + `memory_write_anchor` + `memory_write_file`
2. ✅ `STOP` = 1，且 Stop hook `verify_skill_draft.sh` 通过（ADR-V2-006 envelope 正确）
3. ✅ L2 写入的 memory_file 里 `content` 是结构化 skill draft（YAML frontmatter + workflow 段）

---

## 四、JSON 格式深度验证（可选）

```bash
eaasp session events $SID_GRID --format json | jq '.events[] | select(.event_type=="PRE_TOOL_USE")' | head -40
```

**ADR-V2-006 hook envelope 字段清单**（应出现在 payload 内）:

```json
{
  "seq": 7,
  "event_type": "PRE_TOOL_USE",
  "payload": {
    "tool_name": "scada_read_snapshot",
    "arguments": {"device_id": "Transformer-001", "time_window": "1h"}
  },
  "source": "interceptor:grid-runtime",
  "cluster_id": "c-xxxx"
}
```

---

## 五、反馈清单

执行完后请提交：

1. **Test 1/2/3/4 的事件计数摘要**（PRE/POST/STOP）
2. **Test 5 skill-extraction 是否成功**
3. **goose 容器 F1 gate 输出**
4. **nanobot/grid/claude-code 至少各跑一轮真实 LLM 成功**
5. **错误点**（如果有）

---

## 六、Sign-off 门控

| 结果 | 后续动作 |
|------|---------|
| ✅ Test 1/2/3 全通过 + Test 4 基线通过 + Test 5 通过 | 进入 `/end-phase`，标 Phase 2.5 🟢 Completed |
| ✅ Test 1/2/3 通过，Test 5 跳过 | 可 sign-off，但建议 Phase 3 首个任务重跑 |
| ⚠️ Test 3 (nanobot) 通过但无 PRE_TOOL_USE | 可 sign-off（视 W2.T6 实际接线状态），记 Deferred |
| ❌ Test 1 或 Test 2 失败 | 阻塞 sign-off — 核心拦截器回归，必须根因 |

**最低门槛**: Test 1 + Test 2 + Test 3 三个真实 agent loop 验证通过 + Test 4 基线通过。

---

## 七、自动化测试覆盖的断言（不用手动验）

以下已被 `make v2-phase2_5-e2e` 覆盖，不用再人工验：

- 合约套件 v1（4 runtimes × Initialize/Send/Terminate/Health/GetCapabilities）
- 各 runtime gRPC 16 方法响应 shape 正确
- ADR-V2-006 hook envelope `event / session_id / skill_id / tool_args / tool_result / is_error` 字段（contract_v1/test_hook_envelope.py）
- `ANTHROPIC_BASE_URL` 处理（no /v1 suffix）
- 双 Terminate 语义（D139 XFAIL）

**人工 E2E 的价值**: 验证**真实 LLM + 真实 MCP server + 真实容器部署**在端到端路径上正确工作，单元/集成测试用 mock LLM 覆盖不到这些。

---

## 八、验收后的 end-phase 流程

通过后执行：

1. 更新 `docs/plans/.checkpoint.json`: 标 S4.T2 DONE (25/25)
2. 更新 `docs/design/EAASP/EAASP_v2_0_EVOLUTION_PATH.md`: Phase 2.5 → 🟢 Completed (2026-04-XX)
3. 写 WORK_LOG 条目（`docs/main/`）
4. mem-save: `project_eaasp_v2_phase2_5_complete.md`
5. 提交 commit: `docs(eaasp): end-phase Phase 2.5 🟢 — L1 Runtime Ecosystem E2E verified`
6. 运行 `/dev-phase-manager:end-phase`
7. 制定 Phase 3（goose ACP full wiring + pydantic-ai + claw-code + ccb）启动计划

---

## 九、Phase 2.5 关键能力 × 人工 E2E 覆盖矩阵

| Phase 2.5 能力 | 对应 Test | 证据 |
|---|---|---|
| ADR-V2-017 L1 生态策略（主力+样板+对比三轨） | 1/2/3/4 | 四 runtime 全启动 |
| ADR-V2-019 容器部署模式 | 4 | `make goose-runtime-container-verify-f1` |
| W1 goose-runtime（基线） | 4 | Initialize/Terminate/Health + F1 |
| W2 nanobot-runtime（完整 agent loop） | 3 | CHUNK + STOP + 真实 LLM 回复 |
| S0 合约套件 v1（4 runtimes） | — | `make v2-phase2_5-e2e`（自动） |
| D120 Rust HookContext ADR-V2-006 parity | 1 | Test 1 `PRE_TOOL_USE` payload 完整 |
| S2.T2 L1_RUNTIME_COMPARISON_MATRIX | — | 文档，静态 |
| S3 CI 门控 | — | `.github/workflows/phase2_5-contract.yml` |
| S4.T1 runbook | 本文档 | — |

**Phase 3 scope（本阶段不验）**:
- goose-runtime 完整 ACP agent loop 接线
- pydantic-ai / claw-code / ccb 对比 runtime
- `D120` 跨 runtime hook envelope 合约长期回归
