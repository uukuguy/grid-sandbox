# EAASP v2.0 Phase 2.5 — Consolidation + goose-runtime 设计文档

**日期**: 2026-04-16
**阶段**: EAASP v2.0 Phase 2.5
**主题**: Runtime 生态首批落地（ADR-V2-017 W1/W2 并行 + 契约基础设施 v1 冻结）
**Status**: Design locked（via `/superpowers:brainstorming`）
**Author**: Jiangwen Su + Claude

---

## 1. Context / 背景

Phase 2 (Memory and Evidence) 23/23 已闭环于 2026-04-16（commit `f4d0edf`）。Phase 2 产出 47 个新 Deferred (D91-D130)，其中 ADR-V2-017 明确承诺 **Phase 2.5 五件交付**：

1. 共享契约测试套件 `tests/contract/`
2. `L1_RUNTIME_ADAPTATION_GUIDE.md` 通用接入指南
3. `L1_RUNTIME_COMPARISON_MATRIX.md` 对比能力矩阵
4. **goose-runtime** 完整 16 方法实装（Phase 2.5 W1）
5. **nanobot-runtime** T2 样板实装（Phase 2.5 W2）

Phase 2.5 以此 5 件 + 1 件**人工 E2E 验证 runbook**（中档断言清单）为交付边界，不纳入 D130 / D78 / D94 / D98 等其余 consolidation 事项（留 Phase 3 专项）。

---

## 2. Scope Boundaries / 范围边界

### 2.1 In Scope

- **S0 共享契约集 v1**（中契约 ~35 cases）
  - proto 16 方法 request/response shape
  - 7 event_type 枚举有效性
  - MCP bridge 调用路径
  - skill workflow.required_tools 执行
  - hook envelope §2/§3 字段存在性（ADR-V2-006）
  - 端到端 smoke：Initialize → Send → Events → Close
- **S0 D120 envelope parity**（作契约测试驱动实装）
  - Rust `HookContext::to_json/to_env_vars` 补 ADR-V2-006 §2/§3 字段：`event`、`skill_id`、`draft_memory_id`、`evidence_anchor_id`、`created_at`
  - 新增 env vars：`GRID_EVENT`、`GRID_SKILL_ID`
  - 与 Python runtime 实装对齐
- **W1 goose-runtime**（Rust，thin wrap）
  - `crates/eaasp-goose-runtime/` 新 crate
  - `goose_adapter.rs` 翻译层：EAASP 16 方法 → goose 原生 API
  - 16 方法完整实装 + 契约测试全过
  - goose 不发 crate 时 fallback subprocess-via-MCP（goose 是 MCP 原生）
- **W2 nanobot-runtime**（Python，OpenAI-compatible）
  - `lang/nanobot-runtime-python/` 新 package
  - provider 层：httpx + OpenAI Chat Completions 兼容接口
  - 三 env vars 可切任意 OpenAI-compat 端点：`OPENAI_BASE_URL` + `OPENAI_API_KEY` + `OPENAI_MODEL_NAME`
  - 无 OpenRouter-specific header/routing 字段
  - 16 方法完整实装 + 契约测试全过 + 能跑 skill-extraction 样板 skill
- **S2 文档**
  - `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md`
  - `docs/design/EAASP/L1_RUNTIME_COMPARISON_MATRIX.md`
- **S3 CI**
  - 4 runtime × 契约集 CI matrix
  - Makefile `v2-phase2_5-e2e` automated gate
- **S4 人工 E2E runbook**（中档）
  - `scripts/phase2_5-runtime-verification.sh`
  - 4 runtime × skill-extraction × 附加断言清单（"应看到 X/Y/Z"）
  - 人工签字 + 截图 + 异常记录
  - 至少 2 runtime 真实 OpenAI-compat 端点跑通

### 2.2 Out of Scope（留 Phase 3+）

| Deferred | 内容 | 归属 |
|----------|------|------|
| D130 | session-lifetime parent token consolidation | Phase 3 |
| D78 | event payload embedding | Phase 3（与 memory semantic 共 HNSW 架构） |
| D94 | MemoryStore 单例 refactor（闭 D12） | Phase 3 |
| D98 | HybridIndex HNSW 持久化 | Phase 3 |
| D108 | hook script bats/shellcheck 自动回归 | Phase 3 |
| D117 | Prompt executor (原 D50) | Phase 3 |
| D125 | events/stream burst cap | 按需 |
| T3 pydantic-ai-runtime | 类型化 Python 样板 | Phase 3 |
| claw-code-runtime | UltraWorkers 对比 | Phase 3 |
| ccb-runtime | TS/Bun 对比（仅内部） | Phase 3 |

### 2.3 关键非 goal

- 不重构 claude-code-runtime 的 Anthropic SDK 原生路径（保留为官方 SDK 样板）
- 不抽 `eaasp-provider-py` 共享包（等 Phase 3 pydantic-ai-runtime 引入后才有 rule-of-three）
- 不做契约集 v2 breaking changes（v1 冻结后走 v1.1 增量升级）
- 不做 error taxonomy 跨 runtime 一致性强制（放 COMPARISON_MATRIX 观察维度，不做准入项）
- 不做性能门槛（留 Phase 3）

---

## 3. Architecture / 架构

### 3.1 Phase 2.5 Runtime 生态目标状态

```
L1 Runtime 生态（Phase 2.5 结束）
├── grid-runtime (Rust)               主力 ✅（已存在，Phase 2.5 在此补 D120）
├── claude-code-runtime (Python)      样板：Anthropic SDK 原生 ✅（已存在，Phase 2.5 契约认证）
├── hermes-runtime (Python)           样板：外挂 agent framework ⏸️（冻结，Phase 2.5 不改）
├── goose-runtime (Rust)              对比：Block 官方 🆕 W1
└── nanobot-runtime (Python)          样板：OpenAI-compat 轻量 🆕 W2
```

### 3.2 共享契约集架构

```
tests/contract/
├── contract_v1/
│   ├── proto_shape/              # 16 方法 request/response shape
│   ├── event_type/               # 7 event_type 枚举
│   ├── mcp_bridge/               # MCP 工具调用路径
│   ├── skill_workflow/           # skill.workflow.required_tools
│   ├── hook_envelope/            # ADR-V2-006 §2/§3 字段
│   └── e2e_smoke/                # Initialize→Send→Events→Close
├── harness/
│   ├── runtime_launcher.py       # 启动被测 runtime (gRPC client)
│   ├── assertions.py             # 共享断言库
│   └── mock_openai_server.py     # Mock OpenAI-compat server (CI 用)
└── conftest.py                   # pytest fixtures (per-runtime config)
```

**契约测试执行模型**:
- 每个被测 runtime 暴露 `eaasp.runtime.v2` gRPC endpoint
- harness 以 pytest gRPC client 身份调用，断言 proto shape 和行为
- provider 侧用 `mock_openai_server.py` (FastAPI based) 零外部依赖
- CI 矩阵：`pytest tests/contract/ --runtime={grid,claude-code,goose,nanobot}`

### 3.3 goose-runtime thin wrap 架构

```
┌─────────────────────────────────────────┐
│ eaasp-goose-runtime (crate)             │
│                                          │
│ ┌─────────────┐    ┌─────────────────┐ │
│ │ gRPC server │───▶│ goose_adapter.rs│ │
│ │ (16 methods)│    │ (translation)   │ │
│ └─────────────┘    └────────┬────────┘ │
│                              │          │
│                              ▼          │
│                     ┌────────────────┐  │
│                     │ goose (cargo   │  │
│                     │  dep)          │  │
│                     └────────────────┘  │
└─────────────────────────────────────────┘
         │
         │ (fallback if goose 不发 crate)
         ▼
  subprocess-via-MCP (goose 本身是 MCP 原生)
```

**翻译语义表**（goose_adapter.rs 核心）:

| EAASP method | goose 内部调用 |
|--------------|---------------|
| `Initialize` | `goose::session::new()` + managed hooks 注入 |
| `Send` | `goose::session::send_message()` |
| `Events` | goose session event stream → `eaasp.runtime.v2.Event` |
| `Close` | `goose::session::close()` |
| ... (其余 12 方法) | ... |

### 3.4 nanobot-runtime OpenAI-compat 架构

```
┌─────────────────────────────────────────┐
│ lang/nanobot-runtime-python             │
│                                          │
│ ┌─────────────┐   ┌──────────────────┐ │
│ │ gRPC server │──▶│ session.py       │ │
│ │ (16 methods)│   │  (agent loop)    │ │
│ └─────────────┘   └────────┬─────────┘ │
│                             │            │
│                             ▼            │
│                    ┌────────────────┐   │
│                    │ provider.py    │   │
│                    │ (httpx + OAI)  │   │
│                    └────────┬───────┘   │
└─────────────────────────────┼───────────┘
                              │
                              ▼
          ┌──────────────────────────────────────┐
          │ OPENAI_BASE_URL (可切)                │
          │  ├── openrouter.ai/api/v1 (默认)      │
          │  ├── api.openai.com/v1                │
          │  ├── Azure OpenAI endpoint            │
          │  ├── vLLM / LM Studio / Ollama       │
          │  └── 任何 OpenAI-compat 端点          │
          └──────────────────────────────────────┘
```

**provider 约束**:
- 仅使用 OpenAI Chat Completions 标准 payload（`messages`、`model`、`tools`、`temperature`）
- 仅使用标准 header：`Authorization: Bearer ${OPENAI_API_KEY}`、`Content-Type: application/json`
- 禁止 OpenRouter-specific 字段：`HTTP-Referer`、`X-Title`、`provider` routing
- tool_calls 使用 OpenAI function calling 标准 schema

---

## 4. Data Flow / 数据流

### 4.1 契约测试 TDD 驱动 D120

```
1. S0 先写契约测试（assert envelope has event/skill_id/draft_memory_id/evidence_anchor_id/created_at）
   → grid-runtime 契约测试 🔴 RED
2. S0 实装 D120（Rust HookContext::to_json/to_env_vars 补字段）
   → grid-runtime 契约测试 🟢 GREEN
3. Python runtime 契约测试验证已对齐（应该本来就 GREEN，因 Phase 2 S3.T5 Python 侧已合规）
4. S1 W1/W2 各自跑契约测试 → 必须 GREEN 才能进 S2
```

### 4.2 Phase 2.5 整体交付序列

```
S0 (3-4d, 串行 prerequisite)
  ├── 契约集 v1 骨架
  ├── D120 Rust 实装
  └── grid-runtime + claude-code-runtime 契约测试全绿 ← 证明契约集合理

S1 W1 ∥ W2 (5-7d, 并行)
  ├── goose-runtime 起步 → 16 方法 → 契约测试全绿
  └── nanobot-runtime 起步 → 16 方法 → 契约测试全绿 → skill-extraction 实跑

S2 (2d, 文档)
  ├── ADAPTATION_GUIDE (基于 W1/W2 经验抽共性)
  └── COMPARISON_MATRIX (4 runtime × N 能力维度)

S3 (2d, CI)
  └── Makefile + GitHub Actions matrix

S4 (1-2d, 人工 gate)
  └── runbook + 真人眼过 + 签字
```

---

## 5. Testing Strategy / 测试策略

### 5.1 三层测试

| 层级 | 内容 | 位置 |
|------|------|------|
| **契约层** | S0 共享契约集 ~35 cases | `tests/contract/` |
| **runtime 内部** | 每 runtime 自身单元/集成测试 | `crates/*/tests`, `lang/*/tests` |
| **端到端** | CI matrix + 人工 E2E runbook | `scripts/phase2_5-runtime-verification.sh` |

### 5.2 契约测试 CI 矩阵

```yaml
# .github/workflows/contract.yml (Phase 2.5 新增)
strategy:
  matrix:
    runtime: [grid, claude-code, goose, nanobot]
steps:
  - name: Start ${{ matrix.runtime }} runtime
  - name: Run pytest tests/contract/ --runtime=${{ matrix.runtime }}
```

### 5.3 人工 E2E 断言清单示例

```
[Step 3] 在 grid-runtime 触发 skill-extraction skill
  ✅ 你应该看到：
    [ ] TOOL_CALL event (name=memory_search)
    [ ] TOOL_RESULT event (status=ok)
    [ ] 3 次 tool_use 循环 (memory_search → read → write_anchor → write_file)
    [ ] Hook PostToolUse 触发 1 次
    [ ] L2 memory store 出现 1 条 evidence_anchor + 1 条 memory_file
  ❌ 如果看到：
    [ ] 任何 error 级 log
    [ ] 事件序列中断或 timeout
  → 签字 / 截图 / 记录异常
```

---

## 6. Risks & Mitigations / 风险与对策

| 风险 | 影响 | 对策 |
|------|------|------|
| **goose 不发 crate** | W1 thin wrap 方案不成立 | Fallback subprocess-via-MCP；goose 是 MCP 原生，协议稳定 |
| **goose upstream 升级破坏翻译层** | W1 维护负担大 | Cargo.toml `goose = "=X.Y.Z"` 精确 pin 版本；翻译层独立模块 `goose_adapter.rs` 集中改 |
| **契约集 v1 被 Rust runtime 形塑，Python 揭露盲区** | 回填成本 | S0 末期 grid-runtime + claude-code-runtime **双通过** 才冻结 v1，验证至少一 Rust + 一 Python |
| **W1 W2 并行时契约集同时演化导致三方互相拉扯** | 进度不可控 | S0 冻结 v1 快照后才启 W1/W2；W1/W2 期间如需改契约，走 PR + 契约集升 v1.1 流程，不直接改 v1 |
| **OpenAI-compat 端点行为差异（Azure vs 原生 vs vLLM）** | W2 契约测试 flaky | 契约测试只用 `mock_openai_server.py`（零真实端点），真实端点仅在人工 E2E runbook 验证 |
| **人工 E2E runbook 签字流于形式** | 验证门无效 | 中档断言清单明示"应看到 X/Y/Z"，签字人必须对照打勾；异常必须记录到 DEFERRED_LEDGER |
| **nanobot provider 代码重复 (与未来 pydantic-ai-runtime)** | 技术债 | 接受短期重复；Phase 3 引入 T3 后再抽 `eaasp-provider-py` 共享包（rule-of-three 时机成熟） |
| **契约集覆盖不全遗漏语义** | 后续 runtime 踩坑 | 中契约 v1 明确承认不覆盖 error taxonomy/multi-turn/capability；留 COMPARISON_MATRIX 作观察维度 |

---

## 7. Exit Gate / 退出条件

- [ ] 6 件交付齐全：契约集 + goose-runtime + nanobot-runtime + 2 份文档 + 人工 runbook
- [ ] 4 runtime × 契约集 CI 全绿
- [ ] `make v2-phase2_5-e2e` automated gate 全绿
- [ ] 人工 E2E runbook 签字完成（至少 2 runtime 真实 OpenAI-compat 端点跑通 skill-extraction）
- [ ] ADR-V2-017 W1/W2 deliverables 勾选完成
- [ ] 所有 reviewer 发现 closed 或 routed 到 Phase 3 non-blocking Deferred
- [ ] `.phase_stack.json` + `MEMORY.md` + `DEFERRED_LEDGER.md` 同步
- [ ] WORK_LOG.md + NEXT_SESSION_GUIDE.md (Phase 3) 更新

---

## 8. Timeline / 时间估算

| Stage | 串行天数 | 并行后 |
|-------|---------|--------|
| S0 契约集 v1 + D120 | 3-4d | 3-4d |
| S1 W1 goose-runtime | 5-7d | (并行 W2) |
| S1 W2 nanobot-runtime | 5-7d | 5-7d |
| S2 文档 | 2d | 2d |
| S3 CI | 2d | 2d |
| S4 人工 E2E | 1-2d | 1-2d |
| **合计** | **18-24d** | **13-17d** |

---

## 9. Open Questions / 开放问题

（写 plan 时进一步解决）

1. **goose crate 发布状态** — ✅ CLOSED 2026-04-16 via W1.T0：**Block/goose 未发布到 crates.io**。crates.io 现有的 `goose` crate（v0.1.0 自 2017-10-23，最新依赖 `ctrlc`/`reqwest`/`structopt`）是 Tag1 的 **HTTP 负载测试工具**（jeremyandrews 作者），与 Block AI agent 同名但完全无关；`goose-ai` 不存在（crates.io API 404）；`block-goose` 无匹配。Block goose 在 GitHub (`block/goose`, 42.3k stars, `main` 分支, workspace `v1.31.0`) 以 workspace 源码形式发布，`crates/*` 包括 `goose`/`goose-sdk`/`goose-cli`/`goose-mcp`/`goose-server`/`goose-acp` 等 9 个内部 crate，均 **未设置 `publish = ["crates-io"]`** 且 workspace `[package]` 默认即 `publish = false`。`goose-sdk` 官方定位是 **"Rust SDK for talking to Goose over the Agent Client Protocol (ACP)"**——即 ACP/MCP 客户端，而非 in-process 嵌入库。**决定走 Outcome B（subprocess-via-ACP/MCP）为主**，原因：(a) 避免引入 workspace git 依赖带来的 supply-chain 风险与 OpenSSL/rustls 特性冲突（goose `default` features 包含 `local-inference`/`aws-providers`/`otel`/`candle-core`/`llama-cpp-2`，拉进来会让 `cargo build` 膨胀 10+ 分钟），(b) goose 原生即 ACP/MCP，subprocess 是 Block 官方推荐的嵌入方式（见 `goose-sdk` 的 `sacp` 依赖），(c) 保留 git dep 作为 P2 fallback：`goose = { git = "https://github.com/block/goose", tag = "v1.31.0", default-features = false, features = ["rustls-tls"] }`——仅在契约层出现必须 in-process 的语义差异时启用。**§3.3 架构图的 cargo dep 箭头按现有 line 143 callout 降级为 fallback 读法，主路径是图下方的 "subprocess-via-MCP"**。
2. **契约测试 harness 语言选择** — pytest 可同时跑 Rust/Python runtime（gRPC client），已确定用 pytest。
3. **mock_openai_server 是否覆盖 tool_calls** — W2 需要 tool_use 支持，mock server 必须支持 streaming + function calling，复杂度评估放到 plan 阶段。
4. **goose 能否接受 managed hooks 注入** — ✅ CLOSED 2026-04-16 via W1.T0：**程序化 Hook API 存在但仅限 git dep 路径**。`crates/goose/src/tool_inspection.rs` 定义 `#[async_trait] trait ToolInspector { async fn inspect(&self, session_id, tool_requests, messages, goose_mode) -> Result<Vec<InspectionResult>> }`，返回 `InspectionAction::{Allow, Deny, RequireApproval(Option<String>)}`——这正是 **PreToolUse hook** 语义。管理器 `ToolInspectionManager::add_inspector(Box<dyn ToolInspector>)` 允许外部宿主注入自定义 inspector（goose 默认即注入 `SecurityInspector`+`EgressInspector`+`AdversaryInspector`+`PermissionInspector`+`RepetitionInspector` 五个，见 `crates/goose/src/agents/agent.rs`）。但是：**此 trait 只在 Rust 源码层可达，ACP/MCP 协议表面没有等价接口**——subprocess 路径无法从外部 `add_inspector`。因此在 **Outcome B 主路径下 W1.T3 必须用 adapter 侧 emulation**：`goose_adapter.rs` 在 goose subprocess 前插入一个 **wrapping MCP middleware server**（`eaasp-scoped-hook-mcp`），拦截所有 `tools/call` 请求，对照 ADR-V2-006 §2/§3 envelope 把 PreToolUse 脚本分发出去，读取 stdout 判 Allow/Deny/InjectAndContinue，再把结果透传给 goose 后面的真实 MCP tool server。PostToolUse + Stop 同理 emulate 在 Tool response 和 session termination 边界。这种 "MCP middleware as hook proxy" 是 goose MCP 原生架构下最干净的注入点，不需要 fork。**Fallback（git dep 路径）**：若 emulation 出现语义漂移，激活 git dep 后直接 `agent.tool_inspection_manager_mut().add_inspector(Box::new(EaaspScopedHookInspector))`，零 emulation 代价。PostToolUse/Stop 对应 goose 侧的 `tool_execution.rs` + `session_context.rs` 扩展点（git dep 下可直接 wrap agent.reply() 循环）。

---

## 10. References

- ADR-V2-017 — L1 Runtime 生态策略（主力 + 样板 + 对比 三轨）
- ADR-V2-006 — Scoped-hook envelope 契约 §2-§10
- ADR-V2-016 — Agent loop 设计原则
- Phase 2 `2026-04-14-v2-phase2-plan.md` — Phase 2 plan（上一阶段）
- `docs/design/EAASP/DEFERRED_LEDGER.md` — D120 原始登记
- Phase 2 S3.T5 `project_s3_t5_scoped_hook_executor.md` — D120 发现根因（Rust HookContext 预先于 ADR-V2-006）

---

**Phase 2.5 设计冻结，下一步：`/superpowers:writing-plans` 产出任务拆解 plan。**
