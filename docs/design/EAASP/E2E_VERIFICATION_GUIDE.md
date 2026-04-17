# EAASP v2.0 — 人工 E2E 验证长期指南

> **性质**: 长期指南（Living Document）。每个大阶段结束前参考本文进行人工 E2E 验证。
> **演进规则**: Phase 推进时**追加**能力行到矩阵；**不改**已有行结构；**不创建**新 `PHASE_X_E2E_VERIFICATION_GUIDE.md`。
> **最近更新**: 2026-04-17（Phase 2.5 sign-off）

---

## 一、验证哲学

### 何时做人工 E2E

| 时机 | 是否要做 |
|------|----------|
| 大阶段（Phase N.0 / N.5）收尾前 | ✅ 必须 |
| 新 L1 runtime 接入后首轮 sign-off | ✅ 必须 |
| 新增 **拦截器事件类型** / **hook 类型** / **Provider** | ✅ 必须 |
| 单 Task 结束 | ❌ 不需要（合约/集成/单元测试已覆盖） |
| Deferred 修复 | ❌ 不需要（对应测试用例覆盖即可） |

### 人工 E2E 的唯一价值

**验证真实 LLM + 真实 MCP server + 真实容器部署在端到端路径上正确工作。** mock-based 的单元/集成/合约测试覆盖不到这些。

### 不做什么

- ❌ 不代替合约套件（`make v2-phase2_5-e2e` 等自动化门控）
- ❌ 不代替单元测试覆盖（`cargo test` + `pytest`）
- ❌ 不覆盖性能/压力测试
- ❌ 不 cover mock 已验证过的字段形状

---

## 二、前提与快速启动

### 2.1 工作目录 + .env

```bash
cd /Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox

# .env 必需（MEMORY.md Env Var Conventions）:
#   OPENAI_API_KEY / OPENAI_BASE_URL / OPENAI_MODEL_NAME  → grid + nanobot
#   ANTHROPIC_API_KEY / ANTHROPIC_BASE_URL                → claude-code
#   LLM_PROVIDER                                          → grid provider 选择
```

### 2.2 CLI 别名

```bash
alias eaasp='/Users/sujiangwen/sandbox/LLM/speechless.ai/SGAI/grid-sandbox/tools/eaasp-cli-v2/.venv/bin/eaasp'
```

### 2.3 一条命令起全栈

```bash
# 首次（或容器镜像变更后）构建 goose 镜像
make goose-runtime-container-build

# 起全栈（含 4 runtime）
make dev-eaasp
```

**预期状态表**：

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

**失败自愈**：`make dev-eaasp-stop && make dev-eaasp`。

---

## 三、Runtime 能力矩阵

> **演进规则**: 新 runtime 接入时加一列；能力升级时改该列对应行的 ✅/⚠️/❌。

| 能力 | grid | claude-code | nanobot | goose |
|---|---|---|---|---|
| gRPC 16 方法合约 | ✅ | ✅ | ✅ | ✅ |
| Initialize / Terminate / Health | ✅ | ✅ | ✅ | ✅ |
| 真实 LLM Provider | ✅ OpenAI-compat | ✅ Anthropic SDK | ✅ OpenAI-compat | ❌ |
| Agent loop（多轮 tool 调用） | ✅ | ✅ | ⚠️ 骨架无工具 | ❌ stub |
| ConnectMCP（工具注入） | ✅ | ✅ | ❌ D144 | ❌ D144 |
| PreToolUse / PostToolUse Hook | ✅ | ✅ | ⚠️ 骨架 | ❌ |
| Stop Hook（ADR-V2-006） | ✅ | ✅ | ❌ D144 | ❌ D144 |
| HookContext envelope parity（D120） | ✅ | ✅ | ⚠️ Python 基础实现 | ❌ D144 |
| 容器部署（ADR-V2-019） | — | — | ❌ native only | ✅ Docker |
| **参与人工 E2E 分类** | **必验** | **必验** | **基线** | **基线** |

### Runtime 角色（ADR-V2-017）

- **主力**: grid-runtime（Rust，自研完整实现）
- **样板**: claude-code-runtime（Python + Anthropic SDK）、nanobot-runtime（Python + OpenAI-compat）
- **对比**: goose-runtime（容器模板，ADR-V2-019 baseline，Phase 3 接完整 ACP）
- **冻结**: hermes-runtime（2026-04-14 冻结）

---

## 四、功能特性 × 验证方法矩阵

> **演进规则**: 每行一特性。Phase 推进时**追加行**，不改已有行。"本次必跑" 在 **七、Phase 收尾历史** 附录按阶段标注。

### 4.1 Group A — threshold-calibration 一轮覆盖

一个 session 就能触发的功能，放在 A 组。**每次大阶段收尾必跑 grid + claude-code 两遍。**

| # | Phase | 能力 | 验收断言（events） |
|---|---|---|---|
| A1 | 0.75 | MCP stdio 连接 + 工具注入 | `SESSION_MCP_CONNECTED` 出现 |
| A2 | 1 | Event Engine 拦截器：SESSION_START | `SESSION_START` 出现 |
| A3 | 1 | Event Engine 拦截器：PRE_TOOL_USE | `PRE_TOOL_USE ≥ 4`（D87 多步 workflow） |
| A4 | 1 | Event Engine 拦截器：POST_TOOL_USE | `POST_TOOL_USE ≥ PRE_TOOL_USE - 1` |
| A5 | 1 | Event Engine 拦截器：STOP | `STOP == 1` |
| A6 | 1 | source metadata | `source == "interceptor:<runtime>"` |
| A7 | 1 | cluster_id 填充 | `cluster_id` 非空（至少部分事件） |
| A8 | 2.S1.T4 | tool_name threading | `PRE_TOOL_USE.payload.tool_name` 非空 |
| A9 | 2.S1.T5 | response_text 抽取 | `RESPONSE_CHUNK.content` 非空 |
| A10 | 2.S3.T4 | Stop Hook InjectAndContinue | `require_anchor.sh` 拒绝空 evidence_anchor |
| A11 | 2.S3.T5 | ScopedHookExecutor + ADR-V2-006 envelope | runtime 日志：hook exit code + stdin 含 `skill_id/event/tool_args` |
| A12 | 2.5.S0.T3 | D120 HookContext envelope parity | PRE_TOOL_USE.payload 含 `event/skill_id/tool_args/tool_result/is_error` |
| A13 | 2.5 | L1 生态扩展（≥3 runtime） | 4 runtime 在状态表 UP |

**A 组验证命令**：

```bash
# grid
eaasp session create --skill threshold-calibration --runtime grid-runtime
export SID=<id>
eaasp session send $SID "校准 Transformer-001 的温度阈值"
eaasp session events $SID       # A2-A12 人工过一遍
eaasp session events $SID --format json | head -80  # A11-A12 深度验

# claude-code
eaasp session create --skill threshold-calibration --runtime claude-code-runtime
export SID2=<id>
eaasp session send $SID2 "校准 Transformer-001 的温度阈值"
eaasp session events $SID2
```

---

### 4.2 Group B — 分项回归（一轮触发不到）

单轮 threshold-calibration 不触发、需要另造会话/环境的能力。**每次完整人工 E2E 必须 B 组全跑**（回归测试原则：Phase N 验过不代表 Phase M 没破坏）。由 `scripts/eaasp-e2e.sh` 自动驱动。

| # | Phase | 能力 | 触发方法 | 验收断言 |
|---|---|---|---|---|
| B1 | 2.S1.T6 | ErrorClassifier 14 FailoverReason | 改 `.env` 造错的 `OPENAI_API_KEY` + send | `RUNTIME_SEND_FAILED.payload.failover_reason` 匹配 |
| B2 | 2.S1.T7 | graduated retry + jitter | 同 B1 | 日志有 2-3 次重试，间隔递增 |
| B3 | 2.S2.T1 | HNSW 向量 + Ollama embedding | 起 Ollama，`eaasp memory search --query` | score 分布合理（非全 0） |
| B4 | 2.S2.T2 | 混合检索权重可调 | `EAASP_HYBRID_WEIGHTS=0.3,0.7 eaasp memory search` | 返回顺序变化可观察 |
| B5 | 2.S2.T3 | memory_confirm MCP 工具 | 第二轮会话："确认建议写为 confirmed" | `PRE_TOOL_USE(memory_confirm)` 出现 |
| B6 | 2.S2.T4 | 状态机 agent_suggested→confirmed→archived | B5 后 `eaasp memory list --status confirmed` | 有新 confirmed 条目 |
| B7 | 2.S2.T5 | L3 聚合溢出 blob ref | skill 里造 >10K 字符 tool output | `RESPONSE_CHUNK.payload` 含 `blob_ref` |
| B8 | 2.S3.T1 | PreCompactHook | 长对话超上下文窗口 | events 含 `PRE_COMPACT` |
| B9 | 2.S3.T2-T3 | skill-extraction meta-skill | `--skill skill-extraction` 另跑一轮 | 写出 skill_draft memory_file |
| B10 | 2.5.W1.T2.5 | goose 容器 F1 gate | `make goose-runtime-container-verify-f1` | exit 0 |
| B11 | 2.5.S0 | 合约套件 v1 四 runtime GREEN | `make v2-phase2_5-e2e` | 全通过 |

---

## 五、核心验证流程

### 5.1 唯一入口：一条命令跑完整 E2E

```bash
# 1. 起全栈 (Terminal A, 常驻)
make dev-eaasp-stop && make dev-eaasp

# 2. 一条命令跑 A + B 全矩阵 (Terminal B)
bash scripts/eaasp-e2e.sh
```

### 5.2 脚本行为契约

`scripts/eaasp-e2e.sh` 的职责（由下一节验证矩阵驱动）：

1. **Pre-flight** — 验 L4 健康 / CLI 可用 / skill 已注册（未注册自动 submit）
2. **A 组** — 对 grid-runtime + claude-code-runtime 各跑一轮 threshold-calibration，逐项断言 A1-A13
3. **B 组** — 顺序执行 B1-B11 各自的触发 + 断言（能自动化的直接跑，需真实数据的复用 A 组 session）
4. **Runtime 基线** — nanobot / goose 跑 Initialize/Terminate/Health 最小合约
5. **汇总** — 写 `.e2e/verify-$(date +%Y%m%d-%H%M).log` + 打印表格：行号 / PASS/FAIL/SKIP/XFAIL / 说明
6. **退出码** — 0 (全 PASS/XFAIL/SKIP) / 1 (任何 FAIL) / 2 (pre-flight 失败)

### 5.3 脚本 flag

```bash
bash scripts/eaasp-e2e.sh                 # 全量
bash scripts/eaasp-e2e.sh --only A        # 只 A 组
bash scripts/eaasp-e2e.sh --only B        # 只 B 组
bash scripts/eaasp-e2e.sh --skip B7,B8    # 跳过耗费大的
bash scripts/eaasp-e2e.sh --runtime grid  # 只测单 runtime
```

### 5.4 手动逐项（紧急排错）

所有行都有独立触发命令，见第四节矩阵的"触发方法"列。脚本失败时复制对应命令手动跑。

---

## 六、Sign-off 门控

### 6.1 必要条件（全矩阵必验）

| # | 条件 | 证据 |
|---|---|---|
| 1 | A 组 grid + claude-code 全通过（A1-A13 全绿） | `.e2e/verify-*.log` |
| 2 | B 组 B1-B11 全部 PASS 或明确 XFAIL（无 FAIL） | `.e2e/verify-*.log` |
| 3 | 所有已接入 runtime 至少基线通过 | 同上 |
| 4 | 脚本退出码 = 0 | `echo $?` |

**原则**：每次完整人工 E2E = 全矩阵回归。不允许"Phase N 验过不用再验" — 这违背回归测试原则。

### 6.2 Sign-off 判定表

| 条件 | 动作 |
|---|---|
| 脚本 exit 0 + A+B 全绿 | → `/end-phase` |
| A 组任一 FAIL | ⛔ 阻塞 — 核心能力回归，根因分析 |
| B 组 FAIL（非 XFAIL） | ⛔ 阻塞 — 回归测试失败，必须查清 |
| 新 runtime 基线 FAIL | ⛔ 阻塞 — 该 runtime 合约退出 |
| XFAIL（已知预期失败） | ✅ 允许 — 要有 Deferred 记录引用 |
| 新暴露 gap | 记 Deferred，该行改 XFAIL + 给出归属 Phase |

### 6.3 Deferred 记录规范

每条新 Deferred 必须含：
- D-编号（DEFERRED_LEDGER.md 顺延）
- 哪次 E2E 暴露的
- 期望归属 Phase
- 严重度（P1-P3）
- 触发条件（复现命令）

---

## 七、Phase 收尾历史

> **每个大阶段收尾时追加一节。不编辑其他阶段的节。**

### Phase 1 — Event Engine（2026-04-14）

- **A 组**: 全通过（A1-A7，当时还没 A8+ 特性）
- **B 组**: 无（所有能力都在 A 组）
- **结果**: 🟢 Completed
- **原 artifact**: `docs/main/PHASE1_E2E_VERIFICATION_GUIDE.md`

### Phase 2 — Memory & Evidence（2026-04-15）

- **A 组**: A1-A11 全通过（S3.T5 ScopedHookExecutor 加入 A 组）
- **B 组**: 全跑（当时脚本手动驱动，见 `scripts/s4t3-runtime-verification.sh`）
- **结果**: 🟢 Completed 23/23

### Phase 2.5 — L1 Runtime Ecosystem（2026-04-17 进行中）

- **本次引入能力**: A12 (D120 HookContext parity) + A13 (L1 生态扩展) + B10 (goose F1) + B11 (合约 v1) + nanobot/goose runtime
- **全矩阵回归**: 本次 sign-off 要求 A1-A13 + B1-B11 全跑（由 `scripts/eaasp-e2e.sh` 驱动）
- **新 runtime 能力**: nanobot（真实 LLM 可回复，无 MCP 工具注入）+ goose（Initialize/Terminate/Health，Send stub → Phase 3）
- **新 Deferred**: D144（nanobot/goose ConnectMCP 工具注入 → Phase 3）
- **结果**: 待 sign-off
- **本次 artifact**: `docs/main/PHASE2_5_E2E_VERIFICATION_GUIDE.md`（降级为历史归档）

### Phase 3 — [待定]

- 预期新 A 组行: goose/nanobot 的 ConnectMCP 工具注入
- 预期新 B 组行: pydantic-ai / claw-code / ccb runtime 评估
- 预期升级: goose Send 完整 ACP 接线

---

## 八、故障排查速查

| 症状 | 可能原因 | 处理 |
|---|---|---|
| `make dev-eaasp` 超时 | 某 runtime .venv/镜像缺失 | `--skip-nanobot` / `--skip-goose` |
| nanobot 404 `/v1/v1/chat/completions` | base_url 重复 `/v1` | provider.py 已做 normalize（2026-04-17） |
| PRE_TOOL_USE < 4 | D87 capability matrix 未启用 | 检查 runtime 日志 tool_choice 决策 |
| events 只有 SESSION_CREATED/USER_MESSAGE | 拦截器没触发 | 检查 runtime 运行中，L4 EventEngine 在 lifespan 启动 |
| cluster_id 全空 | pipeline worker 未启动 | L4 启动日志查 `pipeline_worker: running` |
| Stop hook 不 inject | ScopedHookExecutor 没 register_session_stop_hooks | Rust 日志查 `session_stop_hooks` 计数 |

---

## 九、参考

- **核心 ADR**: ADR-V2-001 / 006 / 015 / 016 / 017 / 019
- **演进路径**: `docs/design/EAASP/EAASP_v2_0_EVOLUTION_PATH.md`
- **Deferred Ledger**: `docs/design/EAASP/DEFERRED_LEDGER.md`
- **自动化门控**: `make v2-phase2_5-e2e` / `scripts/s4t3-runtime-verification.sh`
- **环境约定**: `MEMORY.md` → env var conventions / no fallback / integration before E2E

---

## 十、维护承诺

- 每次大阶段收尾必更新第 7 节
- 新增能力先在合约/单元测试 GREEN → 才进本文第 4 节矩阵
- runtime 能力矩阵第 3 节变更必须同步到 ADR-V2-017
- 本文作者放 `docs/design/EAASP/`（per CLAUDE.md File Organization），不与 `docs/main/` 混放
