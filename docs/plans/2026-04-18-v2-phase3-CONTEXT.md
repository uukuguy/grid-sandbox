# EAASP v2.0 Phase 3 — L1 Runtime Functional Completeness — Context

> 下游 skill (writing-plans / executing-plans) 读取此文件作为契约。本文件里的决策不应在后续 skill 中被重新询问。如需变更，先更新本文件。

**Created:** 2026-04-18 06:35 GMT+8
**Topic:** EAASP v2.0 Phase 3 — L1 Runtime Functional Completeness
**Related prior CONTEXT:** none（首次使用 discuss skill；Phase 2/2.5 走 brainstorming）
**Upstream design docs:**
- `docs/plans/2026-04-16-v2-phase2_5-design.md`（Phase 2.5 设计锚点）
- `docs/design/EAASP/adrs/ADR-V2-017-l1-runtime-ecosystem-strategy.md`（L1 生态三轨）
- `docs/dev/NEXT_SESSION_GUIDE.md` §Phase 3 规划

---

## Scope

**In:**

### S1 — 工具命名空间治理（Phase 3 核心价值点）
- 新 ADR：分层命名空间契约（L0 runtime-core / L1 engine-builtin / L2 MCP-provided）
- `skill.workflow.required_tools` schema 升级为带命名空间前缀（`l1:memory_recall` / `l2:memory_search`）
- `grid-engine` 工具注册表按层打标，filter 路径由 `EAASP_TOOL_FILTER` env 升级为 declarative skill 声明
- skill 声明优先于 runtime 内置列表（冲突时 skill 赢）
- `examples/skills/*` 所有样例重写为命名空间声明形式
- 合约测试 v1.1 增量升级覆盖 namespace 断言

### S2 — Phase 2 历史遗留技术债（7 项全纳）
- **D130** session-lifetime vs per-turn token consolidation（S4.T4 遗留）
- **D78** event payload embedding（与 memory semantic 共 HNSW 架构）
- **D94** MemoryStore 单例 refactor（收尾 D12）
- **D98** HybridIndex HNSW 持久化（当前每 search 重建）
- **D117** Prompt executor（原 D50，LLM-driven yes/no）
- **D108** hook script bats/shellcheck 自动回归
- **D125** events/stream burst cap（>1k/sec 时）

### S3 — L1 Runtime 功能完整性（D144）+ 对比 runtime 全矩阵
- **D144-A** goose-runtime Send 完整 ACP 接线
  - 路径：GooseAdapter.stream() 驱动 goose CLI subprocess
  - 事件映射 ACP → AgentEvent（CHUNK / TOOL_CALL / TOOL_RESULT / STOP）
  - F1 Dockerfile 已 bake v1.30.0 可复用
- **D144-B** nanobot-runtime ConnectMcp + 工具注入
  - 真实 stdio MCP client 实装
  - 工具注册到 AgentSession（当前 tools=[]）
  - Stop Hook dispatch 补齐（当前只有 PostToolUse）
- **对比 runtime 三条轨**（全部进契约，不做 scope 裁剪）：
  - `pydantic-ai-runtime`（Python，类型化样板）
  - `claw-code-runtime`（UltraWorkers 对比轨）
  - `ccb-runtime`（TypeScript/Bun — 新语言栈，需 TS proto stubs）
- E2E harness 补齐 B1-B8 八项自动化触发（错误注入、retry 日志、memory_confirm、聚合溢出、PreCompact、HNSW、混合检索样本集）
- **sign-off 标准**：5 个新 runtime × 契约 v1.1 全 PASS + skill-extraction E2E 跑通

**Out:**

- 性能门槛（留 Phase 4+）
- error taxonomy 跨 runtime 强制一致（COMPARISON_MATRIX 观察维度即可）
- 合约 v2 breaking changes（v1.1 增量；v2 留未来）
- 新增 L1 runtime 样本（不纳 opencode / agno / hexagent / agt 等）

**Deferred（trigger condition 明确）:**

- **FE + Observability Dashboard**（trigger：Phase 3 sign-off 后，如果用户场景催）
- **ACP/MCP 协议版本兼容矩阵**（trigger：goose 出 ACP v2 或 MCP 出 breaking 时）
- **Docker image 瘦身 / multi-stage**（trigger：CI 构建时长 >10min 或镜像 >500MB）
- **Cross-runtime cache coherence（memory shared HNSW）**（trigger：多 runtime 同时写 memory 出现一致性问题）

---

## Decisions

### 1. Phase 3 主线范围 — DECIDED
**Choice:** Namespace 治理先行 + D144 接线（+ 全量 P1-defer + 全量对比 runtime）
**Why:** 命名空间是 Phase 2.5 sign-off 7 类结构债的根源，治理是 root cause；D144 + 对比 runtime 是治理方案的验证载体。
**Source:** user

### 2. 命名空间架构方向 — DECIDED
**Choice:** 分层命名（L0/L1/L2）+ skill 显式声明 + skill 声明优先
**Why:** 对 Phase 2.5 EAASP_TOOL_FILTER + executor.rs gate 的打补丁做正名；declarative 前缀给 skill 作者系统性控制力；与 ADR-V2-017 L1 生态策略对齐。
**Source:** user

### 3. D144 goose Send 技术路径 — DECIDED
**Choice:** GooseAdapter 驱动 goose CLI subprocess（W1.T0 spike 的 Outcome B）
**Why:** block/goose 未发 crate（workspace publish=false），goose-sdk 是 ACP 客户端非 embeddable；F1 Dockerfile 已预装 v1.30.0；subprocess 保留 ACP session 语义。
**Source:** user + W1.T0 spike report（memory: project_s1_w1_t0_goose_spike.md）

### 4. Phase 2 P1-defer 7 项处理 — DECIDED
**Choice:** 全部纳入 Phase 3（D130/D78/D94/D98/D117/D108/D125 全做）
**Why:** 用户要把 Phase 3 做成 consolidation phase，一次清所有结构债，避免债继续滚到 Phase 4+。
**Source:** user

### 5. Stage 编排 — DECIDED
**Choice:** 三轮 Stage — S1 治理 → S2 技术债 → S3 接线 + 对比
**Why:** 每轮边界清楚可单独 sign-off；三轮之间有自然依赖（命名空间治理先定架构，技术债消化环境整洁后接线才稳）。
**Source:** user

### 6. D144 验证标准 — DECIDED
**Choice:** 契约测试全绿 + skill-extraction E2E + 对比 runtime 同样进契约
**Why:** 不留 XFAIL，避免「局部绿色、整体褴褛」；对比 runtime 进契约可一次性验证 EAASP 跨语言跨生态的抽象正确性。
**Source:** user

### 7. 对比 runtime 必项清单 — DECIDED
**Choice:** pydantic-ai + claw-code + ccb 三条全做
**Why:** ADR-V2-017 三轨规划一次到位；ccb 还可以验证 EAASP 契约的语言中立性（Rust/Python/TS 三语言全覆盖）。
**Source:** user

### 8. 工作量 & 周期 — DECIDED
**Choice:** 维持现有范围，接受 3-5 周较长周期（估 30-40 tasks）
**Why:** 用户明确偏好「一次清债」而非小步迭代；Phase 2 (23)、Phase 2.5 (25) 的体量曲线可支撑 30-40 tasks 的可执行性。
**Source:** user

### 9. 测试框架沿用 — DECIDED (inferred)
**Choice:** 沿用 Phase 2.5 contract suite 基础设施（pytest + grpcio-tools + FastAPI mock_openai_server）
**Why:** 已稳态（35 cases × 4 runtimes），v1.1 增量升级而非新起；MEMORY.md 已记载运行经验。
**Source:** inferred

### 10. goose nanobot 冻结 vs 继续改 — DECIDED (inferred)
**Choice:** 继续改（goose Send 接 ACP、nanobot ConnectMcp 实装）
**Why:** Phase 2.5 sign-off 已明确把 goose Send stub + nanobot 空 tools 列为 Phase 3 backlog；ADR-V2-017 两样板属 in-scope。
**Source:** inferred

---

## Reusable Assets Found

| Path | What it is | How we use it |
|---|---|---|
| `crates/eaasp-goose-runtime/src/{service,goose_adapter,main,lib}.rs` | Phase 2.5 W1 产物：Initialize/Terminate 完整，其他方法 stub | S3 D144-A 扩展 send + connect_mcp + on_tool_call 等 |
| `crates/eaasp-goose-runtime/src/goose_adapter.rs` | GooseAdapter subprocess 骨架 | 基础上补 .stream() ACP → AgentEvent 映射 |
| `lang/nanobot-runtime-python/src/nanobot_runtime/{service,session,provider}.py` | Phase 2.5 W2 产物：16 gRPC 齐、AgentSession 多轮 loop、OAI provider | S3 D144-B 扩展 session.tools 实装 + stdio MCP client + Stop Hook |
| `tests/contract/` | Phase 2.5 S0 冻结的合约 v1（contract-v1.0.0 tag） | S1.T? 增量升级到 v1.1 + namespace assertions；S3 扩展到 5 新 runtime matrix |
| `tests/contract/harness/{runtime_launcher,mock_openai_server,assertions}.py` | RuntimeLauncher + FastAPI mock + assertions | pydantic-ai / claw-code / ccb 全部复用此 launcher |
| `examples/skills/skill-extraction/{SKILL.md,hooks/*,check_final_output.sh}` | Phase 2 S3.T2+T3 遗产 — skill-extraction 真实样板 | Phase 3 namespace 声明示范 + D144 E2E 冠军测试 |
| `crates/grid-engine/src/agent/executor.rs` | EAASP_TOOL_FILTER + GUARD snapshot 当前补丁代码（L378-400+） | S1 重构为 declarative 命名空间 filter 体（保留行为，换实现） |
| `crates/eaasp-scoped-hook-mcp/` | Method A stdio proxy MCP | S1 namespace 治理可能需要 hook envelope 同步升级；nanobot Stop Hook dispatch 可能复用 |
| `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` + `L1_RUNTIME_COMPARISON_MATRIX.md` | Phase 2.5 S2 文档 | 新增 pydantic-ai / claw-code / ccb 行；namespace 治理章节扩写 |
| `proto/eaasp/runtime/v2/*.proto` | Phase 2 S2 重命名后的单 package v2 proto | 若 namespace 治理需加字段走 v2.1 minor bump，不破坏 v2 兼容 |
| `docs/plans/2026-04-14-v2-phase2-plan.md` + `2026-04-16-v2-phase2_5-plan.md` | Phase 2 (23 tasks) + Phase 2.5 (25 tasks) 参考模板 | writing-plans 阶段模仿其 Stage 分解粒度 |
| `scripts/eaasp-e2e.sh` + `scripts/dev-eaasp.sh` | E2E 唯一入口 + 起全 4 runtime | Phase 3 扩展到 5 runtime + 新命名空间断言 |
| `docs/design/EAASP/DEFERRED_LEDGER.md` | D 编号 single source of truth | P1-defer 7 项的出处 + 关闭归档入口 |

---

## Constraints (from CLAUDE.md / conventions / prior CONTEXT)

- **Language:** Rust 1.75+ / Python 3.12+ / TypeScript 5.7+（ccb 新引入）
- **Rust 构建:** cargo workspace，publish=false
- **Python 构建:** uv + hatchling backend
- **Test framework:** pytest（Python）+ cargo test（Rust）+ vitest（TS，待 ccb 引入时敲定）
- **Proto:** eaasp.runtime.v2 单 package；minor bump 为 v2.1，不破坏 v2
- **Commit format:** Conventional commits + `Generated-By: Claude (claude-opus-4-7) via Claude Code CLI` + `Co-Authored-By: claude-flow <ruv@ruv.net>`
- **LLM provider policy:**（MEMORY: D20）grid-runtime / 新 runtime 默认 `OPENAI_*`，claude-code-runtime 专属 `ANTHROPIC_*`
- **No fallback:**（MEMORY: feedback_no_fallback）配置缺失直接 fail，不 fallback
- **Integration-before-E2E:**（MEMORY: feedback_integration_test_before_e2e）stub 必须匹配真实 runtime 输出，测试必须验证落盘
- **Stay in project root:**（MEMORY: feedback_stay_in_project_root）所有命令用绝对路径，不 cd 子目录
- **File placement:** 设计文档 `docs/design/EAASP/`（中文），ADR `docs/design/EAASP/adrs/`，plans `docs/plans/YYYY-MM-DD-*`，测试 `tests/` 或 crate 内 `tests/`
- **RuFlo 铁律:** 复杂任务必须 `ruflo swarm init --topology hierarchical --max-agents 8 --strategy specialized`
- **E2E 唯一入口:** `bash scripts/eaasp-e2e.sh`，由 `docs/design/EAASP/E2E_VERIFICATION_GUIDE.md` 规范

---

## Open Questions (for writing-plans to resolve in detail)

1. **Namespace ADR 编号**：下一个连续号 ADR-V2-020 还是跳号？（writing-plans 起 ADR 时决）
2. **Skill schema v1.1 vs v2**：required_tools 加命名空间前缀是 additive（向后兼容）还是 breaking？影响 pre-Phase 3 的 skill 存量（skill-extraction 等）。
3. **合约 v1.1 vs v2 命名**：只加断言是 v1.1，改 proto 才 v2。待 writing-plans 阶段读完 namespace ADR 初稿后决。
4. **对比 runtime 实施顺序**：pydantic-ai（接近 nanobot，最易）→ claw-code → ccb（TS 新栈）还是并行？三条线 staffing 是否支持并行？
5. **D130 token consolidation 具体实施点**：`AgentExecutor.cancel_token` 与 `SessionInterruptRegistry` 如何合并为单一 token tree？需读 S4.T4 实装细节再定。
6. **D78 event embedding 与 memory HNSW 复用**：共用 index vs 独立 index 各自索引？性能 vs 一致性的 trade-off。
7. **D94 MemoryStore 单例 scope**：进程级单例 vs tenant/session scope 单例？影响多租户隔离（Phase 2.5 未验证多租户场景）。
8. **Stop Hook dispatch 在 nanobot 实施方式**：复用 Python ScopedCommandExecutor（S3.T5 产物）还是重新实现？
9. **ccb 的 proto stubs 语言选型**：`@grpc/grpc-js` + ts-proto 还是 @bufbuild/protobuf？
10. **E2E B1-B8 的触发形态**：Bash harness 扩展 vs pytest fixture 注入？与现有 scripts/eaasp-e2e.sh 的集成方式。

---

## Exit

Ready for: `/superpowers:writing-plans` 或 `/gsd-plan-phase`
Input to writing-plans: this file + topic summary「EAASP v2.0 Phase 3 — namespace 治理先行 + Phase 2 P1-defer 7 项清债 + D144 goose/nanobot 接线 + pydantic-ai/claw-code/ccb 对比 runtime 进契约。三轮 Stage S1 治理 → S2 技术债 → S3 接线+对比。估 30-40 tasks，3-5 周。」
