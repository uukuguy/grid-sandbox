# EAASP v2.0 Phase 3 — L1 Runtime Functional Completeness 设计文档

**日期**: 2026-04-18
**阶段**: EAASP v2.0 Phase 3
**主题**: 工具命名空间治理 + Phase 2 P1-defer 技术债清理 + D144 L1 runtime 功能补全 + 对比 runtime 全矩阵（pydantic-ai / claw-code / ccb）
**Status**: Design locked（via `/discuss`，见 `docs/plans/2026-04-18-v2-phase3-CONTEXT.md`）
**Author**: Jiangwen Su + Claude
**预计周期**: 3-5 周
**预计任务量**: 30-40 tasks，三轮 Stage

---

## 1. Context / 背景

Phase 2.5（25/25）于 2026-04-18 sign-off 闭环，E2E PASS exit 0。过程中挖出并治本了 7 类结构债（BROADCAST_CAPACITY / EAASP_TOOL_FILTER env / KG-MCP-manage tool_filter / Stop ctx 注入 / SKILL_DIR/hooks materialize / L4 chunk 聚合 / Stop envelope 字段）。其中 **EAASP_TOOL_FILTER 这类 env-driven 打补丁**暴露了一个更深层的问题：

> **grid-engine 内置的 L0/L1 工具**（`memory_recall` / `timeline` / `graph_*` / `bash` / `file_read` / `agent` / `query_agent` / ...）与 **L2 MCP-provided 工具**（`memory_search` / `memory_read` / `memory_write_*` / `memory_confirm` / ...）命名空间混乱，skill 作者无法系统性控制 LLM 工具选择。

Phase 2.5 靠 env 变量和 `executor.rs:378-400+` 的 GUARD snapshot 临时兜底，这次 Phase 3 要系统重构。

同时，Phase 2 遗留 7 项 P1-defer 技术债（D130/D78/D94/D98/D117/D108/D125），Phase 2.5 sign-off 明确列为 Phase 3 backlog。用户决策：**全部纳入，Phase 3 作为一次性 consolidation phase**。

D144（goose Send ACP + nanobot ConnectMcp + 对比 runtime 进契约）既是独立的功能补全，也是 namespace 治理方案的验证载体。

---

## 2. Scope Boundaries / 范围边界

### 2.1 In Scope

| Stage | 主题 | 交付物 |
|-------|------|--------|
| **S1** | 工具命名空间治理 | 新 ADR + 分层注册表 + skill schema v1.1 + 合约 v1.1 |
| **S2** | Phase 2 P1-defer 7 项全清 | D130/D78/D94/D98/D117/D108/D125 closed |
| **S3** | L1 runtime 功能完整性 + 对比 runtime | D144-A/B 接线 + pydantic-ai/claw-code/ccb 进契约 + E2E B1-B8 |

### 2.2 Out of Scope（留 Phase 4+）

| Deferred | 内容 | 归属 |
|----------|------|------|
| FE + Observability Dashboard | 可视化 Dashboard | Phase 4，用户场景催时启 |
| 性能门槛 | latency/QPS/内存 目标值 | Phase 4 |
| error taxonomy 跨 runtime 强制一致 | 仅 COMPARISON_MATRIX 观察维度，不做准入项 | Phase 4 |
| 合约 v2 breaking changes | v1.1 增量；v2 留未来 | 未定 |
| 新增 L1 runtime 样本（opencode / agno / hexagent / agt） | 不在 Phase 3 收 | Phase 4+ |
| Cross-runtime memory HNSW 一致性 | 多 runtime 同时写 memory 的冲突解决 | Phase 4，触发条件明确 |
| ACP/MCP 协议版本兼容矩阵 | 版本 gate 系统 | Phase 4，触发条件明确 |
| Docker image 瘦身 / multi-stage | CI 镜像优化 | Phase 4，触发条件明确 |

### 2.3 关键非 goal

- 不重构 grid-engine 的工具层面 API（Tool trait 不变）—— S1 治理只改注册路径和声明机制
- 不提供跨 runtime 的共享 HNSW 索引——D78 走独立索引路径（见 §3.3）
- 不做 skill workflow 的图灵完备表达（if/else/loop），required_tools 保持 flat list + namespace 前缀
- 不引入 AI judgment 做 namespace 自动识别（显式优先）

---

## 3. Architecture / 架构

### 3.1 工具命名空间治理（S1 核心）

#### 3.1.1 分层定义

```
┌─────────────────────────────────────────────────────────────────┐
│  L0 — runtime-core（runtime 进程自身）                          │
│  ├─ l0:lifecycle.initialize / .terminate / .snapshot            │
│  ├─ l0:session.create / .destroy / .list                        │
│  └─ l0:telemetry.*                                              │
│  —— 对 LLM 不可见，由 gRPC 契约直接调用                          │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  L1 — engine-builtin（grid-engine / 其他 runtime 内置）         │
│  ├─ l1:memory.recall / .timeline / .graph_search                │
│  ├─ l1:filesystem.read / .write / .glob / .grep                 │
│  ├─ l1:bash.execute                                             │
│  ├─ l1:agent.spawn / .query                                     │
│  ├─ l1:web.search / .fetch                                      │
│  └─ l1:...                                                      │
│  —— LLM 可见，runtime-native，零外部依赖                         │
└─────────────────────────────────────────────────────────────────┘
┌─────────────────────────────────────────────────────────────────┐
│  L2 — MCP-provided（外部 MCP server 注册）                      │
│  ├─ l2:memory.search / .read / .write_file / .confirm           │
│  ├─ l2:skill-registry.submit / .list / .fetch                   │
│  ├─ l2:governance.check / .audit                                │
│  └─ l2:{任意 MCP server 定义}                                   │
│  —— LLM 可见，通过 MCP 协议路由                                  │
└─────────────────────────────────────────────────────────────────┘
```

**命名规则**：`{layer}:{domain}.{action}`。
- `layer` ∈ {`l0`, `l1`, `l2`}
- `domain` 可嵌套（e.g. `memory.graph_search` vs `memory.search`）
- `action` 单一动词

#### 3.1.2 冲突解决

skill 声明 `required_tools` 时同名冲突优先级：**skill 显式 > runtime 启动配置 > runtime 内置默认**。

例：
```yaml
# skill YAML
workflow:
  required_tools:
    - l2:memory.search       # 强制用 L2 MCP 版本
    - l1:filesystem.read     # 强制用 L1 内置（不接 MCP 文件服务器）
    - l2:skill-registry.submit  # 强制 L2
```

即使 runtime 也内置了 `memory.search`，skill 声明 `l2:` 前缀后，runtime 要 route 到 L2 MCP 层而不是内置。

#### 3.1.3 注册表升级

`crates/grid-engine/src/tools/traits.rs::Tool` trait 增 `fn layer() -> ToolLayer` 方法（或在 `ToolRegistry::register()` 签名上带 layer 参数），**不破坏**已有 `execute()` 契约。

`ToolRegistry` 内部以 `HashMap<(Layer, FullName), Arc<dyn Tool>>` 存储，对 LLM 暴露时根据 skill 的 `required_tools` 过滤并拼名。

**兼容性策略**：
- pre-Phase 3 skill 未声明 namespace 前缀 → 默认匹配 `l1:` 或 `l2:` 的 fallback 查找链（先 l2，后 l1，避免破坏 MCP 优先的 Phase 2.5 行为）。
- 样例 skill（`examples/skills/skill-extraction/SKILL.md` 等）全部重写为显式 `l2:` 前缀。

#### 3.1.4 EAASP_TOOL_FILTER env 退役路径

Phase 2.5 的 `EAASP_TOOL_FILTER` env + `executor.rs:378-400` GUARD snapshot：
- **保留行为**：session 级别 tool 允许列表，GUARD snapshot 为事实上的 namespace filter
- **替换实现**：由 skill YAML 声明驱动，从 `runtime.Initialize()` gRPC 的 skill metadata 里读
- **env 变量**：保留但标记 deprecated（writing-plans 阶段 Deprecation warning log）

#### 3.1.5 合约测试 v1.1 升级

Phase 2.5 冻结的 `contract-v1.0.0` tag 基础上：
- **新增**（不破坏 v1.0.0）：`test_tool_namespace_enforcement.py` — 验证 skill 声明 `l2:memory.search` 时 runtime 不 route 到 `l1:memory.recall`
- **新增**：`test_conflict_resolution.py` — 同名 L1 / L2 工具冲突时 skill 声明赢
- **新增**：`test_pre_phase3_skill_compat.py` — 未声明前缀的 skill 按 fallback 链查找
- **tag**：完成后打 `contract-v1.1.0` local-only

### 3.2 Phase 2 P1-defer 技术债（S2）

#### 3.2.1 D130 — session-lifetime vs per-turn token consolidation

**问题**：S4.T4 遗留。`SessionInterruptRegistry.cancel_session()` 走 2 条路径：
1. registry flag set
2. `AgentMessage::Cancel` channel send（后者才是 authoritative mid-turn interrupt）

`AgentExecutor` 持 session-lifetime parent token，`AgentLoop` 每轮创建 child token；2 组 token 未联动。

**方案**：统一成单一 `CancellationTokenTree`：
- `session_token: CancellationToken`（root，session-lifetime）
- `turn_token: CancellationToken`（child of session，per-turn）
- `cancel_session(sid)` 只需 `session_token.cancel()`，turn_token 自动级联 cancel
- 保留 `AgentMessage::Cancel` channel 作为 external observer 观测通道

**文件**：`crates/grid-engine/src/agent/{executor,loop,interrupt_registry}.rs`

#### 3.2.2 D78 — event payload embedding（独立索引路径）

**问题**：event.payload 当前未向量化，跨 session retrieval 受限。

**方案**：沿用 S2.T1 `EmbeddingProvider` Protocol，但**独立 HNSW index 文件**（`.grid/embeddings/events/`）与 memory HNSW（`.grid/embeddings/memory/`）分离：
- 好处：避免 memory vs event 的写冲突锁竞争；model_id 漂移独立处理
- 代价：index 重复建（接受）
- **未来**：如果 Phase 4 确认需要共用 index，做 migration（不在 Phase 3 scope）

**文件**：`tools/eaasp-l2-memory/src/event_index.py` 新建 + `tools/eaasp-l2-memory/src/api.py` + dual-write path 对称 files.py

#### 3.2.3 D94 — MemoryStore 单例 refactor

**问题**：`MemoryFileStore` / `AnchorStore` / `HybridIndex` 仍 per-call `connect()`，D12 未闭环。

**方案**：进程级单例 + 共享连接池（aiosqlite `connect()` 共享），tenant 隔离通过 schema 层（`scope_id` 字段）而非多连接实现。
- **Scope**：进程级，非 tenant/session 级。Phase 2.5 未验证多租户，留 Phase 4 再细分。
- **兼容**：保持 `MemoryFileStore()` 构造契约不变，内部走单例工厂。

**文件**：`tools/eaasp-l2-memory/src/{store,anchor_store,hybrid_index}.py`

#### 3.2.4 D98 — HybridIndex HNSW 持久化

**问题**：`HybridIndex.search()` 每次 `_try_load_sync()` 读磁盘 ~10ms。

**方案**：与 D94 合并实施。单例化后 HNSW 只在进程启动时 load 一次，运行态保持在内存；add/remove 更新双写磁盘 + 内存。

#### 3.2.5 D117 — Prompt executor（原 D50）

**问题**：S3.T5 blueprint §F 明确不收 Prompt 执行器，等真实 skill 使用再落地。Phase 3 等不到真实 skill 触发，先做 stub 把结构留好。

**方案**：定义 `PromptExecutor` trait（Rust） + `LlmDrivenPromptExecutor` 实现（Haiku fast model 做 yes/no classify），挂在 `StopHookDecision` 扩展点。测试覆盖但 runtime 默认不启用（env 开关）。
- **风险**：LLM classify 的 latency，留 Phase 4 再调优。

**文件**：`crates/grid-engine/src/agent/prompt_executor.rs` 新建

#### 3.2.6 D108 — hook script bats/shellcheck 自动回归

**问题**：`examples/skills/*/hooks/*.sh` 靠 orchestrator 手动回归；S3.T2 曾因此漏 C1-class bug。

**方案**：
- 每个 `*.sh` 对应一个 `*.bats` 或 `*.test.sh`（bats-core 框架）
- `scripts/test_hook_scripts.sh` 作 unified runner
- 集成到 `make verify` 和 CI matrix
- shellcheck 作 lint，-SC1091（include 不追）-SC2034（unused var）例外

**文件**：`examples/skills/*/hooks/*.bats` 新建 + `scripts/test_hook_scripts.sh` + Makefile 更新

#### 3.2.7 D125 — events/stream burst cap

**问题**：events poll 500 events/s 上限，L1 超限静默滞后。

**方案**：backpressure 策略——超限时 channel 侧 block（而非 drop），暴露 metric 计数 `events_stream_backpressure_total`。L1 >1k/sec 时 Phase 4 再上 sharded channel，Phase 3 仅做可观测性。

**文件**：`crates/grid-engine/src/event/bus.rs` + Prometheus metric 暴露

### 3.3 D144 L1 runtime 功能补全（S3）

#### 3.3.1 D144-A goose-runtime Send 完整 ACP 接线

**路径**（CONTEXT §Decision 3）：
```
client ──gRPC Send──▶ service.rs::send()
                        │
                        ▼
                   GooseAdapter.stream(session_id, messages)
                        │
                        ▼
                   tokio::process::Command (goose acp --stdio)
                        │ stdin/stdout JSON-RPC
                        ▼
                   ACP event parser
                        │
                        ▼
                   map ACP event → AgentEvent
                        │  (CHUNK / TOOL_CALL / TOOL_RESULT / STOP)
                        ▼
                   tokio::sync::mpsc::Sender<SendResponse>
                        │
                        ▼
                   client receives stream
```

**关键设计点**：
- 每 session 一个 subprocess，session_id 映射 process handle 到 `Arc<Mutex<HashMap<SessionId, GooseProcessHandle>>>`
- subprocess exit → emit STOP event + cleanup handle
- kill_on_drop + SIGTERM+5s+SIGKILL（Phase 2.5 F3 fix 复用）
- hook envelope 从 ACP event 抽字段后走 `eaasp-scoped-hook-mcp` 代理链（Phase 2.5 W1.T3 产物）

**文件**：`crates/eaasp-goose-runtime/src/{goose_adapter,service}.rs` 扩展

#### 3.3.2 D144-B nanobot-runtime ConnectMcp + 工具注入

**问题**：当前 `service.py::connect_mcp()` 空实现，`AgentSession` 持 `tools=[]`，Stop Hook dispatch 仅 PostToolUse。

**方案**：
1. **stdio MCP client**：接入 `mcp-python-sdk` 官方 stdio client（或最小 JSON-RPC 实现）
2. **工具注册**：ConnectMcp 请求收到后，建 `McpClient` subprocess，拉 `tools/list`，工具描述塞入 `AgentSession.tools` 列表
3. **OAI provider 投喂**：`provider.py::chat_completion()` 把 `AgentSession.tools` 翻成 OAI function schema 注入 `tools=[]` 字段
4. **Stop Hook dispatch**：复用 Phase 2 S3.T5 产物 `ScopedCommandExecutor` 做 Stop hook 调用（不重写）
5. **teardown**：session Terminate 时 kill 所有关联 MCP subprocess

**文件**：`lang/nanobot-runtime-python/src/nanobot_runtime/{session,service,mcp_client}.py` 扩展 + 新建

#### 3.3.3 对比 runtime 三条全做

| Runtime | 语言 | 样板路径 | 新建位置 |
|---------|------|---------|---------|
| pydantic-ai | Python 3.12+ | 类型化 Agent framework（与 nanobot 近似但 pydantic validator 重） | `lang/pydantic-ai-runtime-python/` |
| claw-code | Rust | UltraWorkers 风格（内部评估，不商用） | `crates/eaasp-claw-code-runtime/` |
| ccb | TypeScript/Bun | TS/Bun 新语言栈验证 | `lang/ccb-runtime-ts/` |

**共性实施**：
- 16 gRPC 方法完整实装（不留 stub）
- Initialize/Send/Terminate/ConnectMcp 跑通 contract v1.1
- 能跑 `examples/skills/skill-extraction/` 样板（已 namespace 治理后的版本）

**ccb 特有**：
- TS proto stubs：`@bufbuild/protobuf` + `@bufbuild/connect`（对 gRPC 更好的 TS 支持）
- Bun 作 runtime（比 Node.js 启动快），但允许 fallback Node.js
- vitest 作测试框架

#### 3.3.4 E2E B1-B8 八项自动化

Phase 2.5 S4 sign-off 遗留 TODO：

| ID | 内容 | 实现方式 |
|----|------|---------|
| B1 | ErrorClassifier E2E harness（错误注入） | pytest fixture：mock provider 返回可配置错误码/超时；`tests/e2e/test_error_classifier.py` |
| B2 | graduated retry 日志解析 | e2e 跑后 grep runtime log 按 retry 曲线断言（1s/2s/4s/8s backoff） |
| B3 | HNSW 样本集 | `tests/e2e/fixtures/memory_hnsw_samples.json` 大规模向量种子 |
| B4 | 混合检索样本集 | FTS+semantic+decay 三路断言，`tests/e2e/test_hybrid_retrieval.py` |
| B5 | memory_confirm 状态机定制 skill | `examples/skills/memory-confirm-test/` |
| B6 | 状态机 edge case skill | 同上扩展 |
| B7 | 聚合溢出 blob_ref | `tests/e2e/test_aggregate_spill.py` 用大 tool output 触发 turn_budget.rs |
| B8 | PreCompact 长对话模拟 | `tests/e2e/test_precompact_long_conversation.py` |

**集成点**：`scripts/eaasp-e2e.sh` 作 entry，各 test 通过 pytest marker 分组，CI 可按 tag 选跑。

### 3.4 Sign-off 标准

Phase 3 视为 "ready for archive" 需达到：

1. ✅ Namespace ADR accepted + 合约 v1.1 tag local-only
2. ✅ 7 项 P1-defer 全 closed（DEFERRED_LEDGER.md 全部挪到 closed 段）
3. ✅ goose + nanobot + pydantic-ai + claw-code + ccb = 5 runtimes × contract v1.1 全 PASS（不留 XFAIL）
4. ✅ `skill-extraction` E2E 在所有 5 runtime 上跑通
5. ✅ B1-B8 八项 E2E 自动化可 `make v2-phase3-e2e` 一键跑
6. ✅ `make verify` 全绿（cargo check + cargo test + pytest + shellcheck + bats）
7. ✅ 人工 sign-off runbook 跑通 + 签字

---

## 4. Stage Breakdown / 阶段划分

### S1 — 工具命名空间治理（6-8 tasks，1-1.5 周）

| Task | 产出 | 大致工作量 |
|------|------|-----------|
| S1.T1 | ADR-V2-020 namespace 契约（write + review + Accepted） | 1d |
| S1.T2 | `ToolLayer` enum + Tool trait 扩展 + ToolRegistry 双键 map 重构 | 1.5d |
| S1.T3 | skill YAML schema v1.1 + 解析器 + fallback 查找链 | 1d |
| S1.T4 | `examples/skills/*` 所有样例重写为 namespace 声明 | 0.5d |
| S1.T5 | `EAASP_TOOL_FILTER` env 退役 + executor.rs filter 路径重构 | 1d |
| S1.T6 | 合约 v1.1 新增 3 个 case（enforcement / conflict / pre-phase3-compat） + tag `contract-v1.1.0` | 1d |
| S1.T7 | `L1_RUNTIME_ADAPTATION_GUIDE.md` namespace 章节 + 文档 | 0.5d |
| S1.T8 | S1 stage sign-off（make verify + 回归 + commit） | 0.5d |

### S2 — Phase 2 P1-defer 清债（10-14 tasks，1.5-2 周）

| Task | 产出 | 对应 D | 工作量 |
|------|------|--------|--------|
| S2.T1 | CancellationTokenTree + executor/loop/registry 改造 + 单元测试 | D130 | 1.5d |
| S2.T2 | event embedding 独立 HNSW index + dual-write + 测试 | D78 | 1.5d |
| S2.T3 | MemoryStore 单例工厂 + 共享连接池 | D94 (D12 收尾) | 1d |
| S2.T4 | HybridIndex HNSW 进程级缓存 + add/remove 双写 | D98 | 1d |
| S2.T5 | PromptExecutor trait + LlmDriven 实现 + 测试 + env 默认关闭 | D117 | 1.5d |
| S2.T6 | bats 测试基建 + `*.bats` 全覆盖 + `scripts/test_hook_scripts.sh` | D108 | 1d |
| S2.T7 | Events backpressure + Prometheus metric 暴露 | D125 | 0.5d |
| S2.T8 | DEFERRED_LEDGER.md 7 项全标 closed + commit 归档 | 0.5d |
| S2.T9 | S2 stage sign-off（make verify + 回归） | 0.5d |

### S3 — D144 + 对比 runtime + E2E（14-18 tasks，1.5-2 周）

| Task | 产出 | 工作量 |
|------|------|--------|
| S3.T1 | goose-adapter ACP event parser + subprocess 管理 + 单元测试 | 1.5d |
| S3.T2 | goose Send 完整 ACP 接线 + 集成测试 + contract v1.1 PASS | 1.5d |
| S3.T3 | nanobot stdio MCP client + ConnectMcp 实装 + 工具注册 | 1.5d |
| S3.T4 | nanobot Send tools 投喂 OAI + Stop Hook dispatch 接 ScopedCommandExecutor | 1d |
| S3.T5 | nanobot contract v1.1 PASS（含 skill-extraction E2E） | 0.5d |
| S3.T6 | pydantic-ai-runtime scaffold + 16 gRPC + provider layer | 1.5d |
| S3.T7 | pydantic-ai contract v1.1 PASS | 0.5d |
| S3.T8 | claw-code-runtime scaffold + 16 gRPC（Rust） | 1.5d |
| S3.T9 | claw-code contract v1.1 PASS | 0.5d |
| S3.T10 | ccb-runtime scaffold + Bun + TS proto stubs + 16 gRPC | 2d |
| S3.T11 | ccb contract v1.1 PASS | 0.5d |
| S3.T12 | E2E B1 (错误注入) + B2 (retry 日志) | 1d |
| S3.T13 | E2E B3 (HNSW 样本) + B4 (混合检索) | 1d |
| S3.T14 | E2E B5+B6 (memory_confirm skill) | 0.5d |
| S3.T15 | E2E B7 (聚合溢出) + B8 (PreCompact) | 1d |
| S3.T16 | `make v2-phase3-e2e` target + `scripts/eaasp-e2e.sh` 扩展 | 0.5d |
| S3.T17 | `L1_RUNTIME_COMPARISON_MATRIX.md` 三新 runtime 行 + `L1_RUNTIME_ADAPTATION_GUIDE.md` ccb 章节 | 0.5d |
| S3.T18 | Phase 3 sign-off runbook + 人工 E2E PASS + 归档 | 1d |

**合计：S1 (8) + S2 (9) + S3 (18) = 35 tasks**

---

## 5. Open Questions / 遗留问题

| # | 问题 | 默认决策（待 writing-plans 敲定） |
|---|------|--------------------------------|
| 1 | ADR 编号 | **ADR-V2-020**（连续号） |
| 2 | skill schema v1.1 兼容策略 | **additive**（未声明前缀走 fallback 查找链，不破坏存量 skill） |
| 3 | 合约命名 | **contract-v1.1.0**（增量，不破坏 v1.0.0） |
| 4 | 对比 runtime 实施顺序 | **串行**：pydantic-ai → claw-code → ccb（易到难，每个 sign-off 后起下一个） |
| 5 | D130 token tree 实施 | **CancellationTokenTree**（parent/child 级联） |
| 6 | D78 event vs memory HNSW | **独立 index**（避免锁竞争） |
| 7 | D94 单例 scope | **进程级**（多租户留 Phase 4） |
| 8 | nanobot Stop Hook 复用 | **复用 ScopedCommandExecutor**（Phase 2 S3.T5 产物） |
| 9 | ccb proto stubs 选型 | **@bufbuild/protobuf + @bufbuild/connect** |
| 10 | E2E B1-B8 触发形态 | **pytest marker**（fixture 注入 + tag 分组） |

---

## 6. Risk / 风险分析

| # | 风险 | 等级 | 缓解 |
|---|------|-----|------|
| R1 | ccb TS 新栈引入成本（学习 + 新工具链） | 🟡 Med | 最后做，从 nanobot 和 pydantic-ai 复用经验；如 >3 天未跑通，退到 Phase 4 |
| R2 | D94 单例改造破坏 Phase 2 测试 | 🟡 Med | 先内部工厂重构保留外部构造契约；全 pytest 矩阵对比 |
| R3 | D117 Prompt executor 没有真实使用场景，可能做完闲置 | 🟢 Low | Phase 3 仅留结构 + 默认关闭；真实场景出现再激活 |
| R4 | D130 token tree 级联 cancel 漏 edge case | 🔴 High | Phase 2 S4.T4 已锁 7+5 测试，Phase 3 增量再补 5 dual-path 场景 |
| R5 | goose ACP 协议解析错误导致 runtime hang | 🟡 Med | subprocess 5s 超时+SIGKILL；单元测试覆盖 malformed event |
| R6 | pydantic-ai validator 重导致 provider 层耗时 >nanobot | 🟢 Low | COMPARISON_MATRIX 观察即可，不做准入 |
| R7 | 对比 runtime 契约差异导致 v1.1 需再 bump | 🔴 High | 每个 runtime 做完立即跑 v1.1，不 batch；发现 gap 立刻回 S1 改 ADR |
| R8 | Phase 3 体量（30-40 tasks）超出 3-5 周 | 🟡 Med | S3 有自然退点（ccb 推后）；每周 retrospective |

---

## 7. Success Criteria / 成功标准

Phase 3 sign-off 视为 "ready for archive" 需全绿：

1. ✅ ADR-V2-020 namespace 契约 Accepted
2. ✅ `contract-v1.1.0` tag local-only（35+ cases）
3. ✅ 7 项 P1-defer 在 `DEFERRED_LEDGER.md` 全部 closed
4. ✅ 5 runtimes（goose / nanobot / pydantic-ai / claw-code / ccb）× contract v1.1 全 PASS 无 XFAIL
5. ✅ `skill-extraction` E2E 在所有 5 runtime 跑通
6. ✅ `make v2-phase3-e2e` 一键跑 B1-B8
7. ✅ `make verify` 全绿（cargo check + cargo test + pytest + shellcheck + bats）
8. ✅ `L1_RUNTIME_COMPARISON_MATRIX.md` 更新到 5-runtime 全行
9. ✅ `make v2-phase3-runtime-verification` 人工 runbook 跑通 + sign-off

---

## 8. References / 参考

- `docs/plans/2026-04-18-v2-phase3-CONTEXT.md` — 本 Phase 决策契约
- `docs/design/EAASP/adrs/ADR-V2-017-l1-runtime-ecosystem-strategy.md` — L1 生态三轨
- `docs/design/EAASP/adrs/ADR-V2-006-hook-envelope-contract.md` — hook envelope（Phase 2.5 S3.T5）
- `docs/design/EAASP/adrs/ADR-V2-019-l1-runtime-deployment-model.md` — L1 部署模式
- `docs/design/EAASP/L1_RUNTIME_ADAPTATION_GUIDE.md` — L1 接入指南（Phase 2.5 S2）
- `docs/design/EAASP/L1_RUNTIME_COMPARISON_MATRIX.md` — runtime 对比矩阵（Phase 2.5 S2）
- `docs/design/EAASP/E2E_VERIFICATION_GUIDE.md` — E2E 规范
- `docs/design/EAASP/DEFERRED_LEDGER.md` — D 编号 single source of truth
- `docs/plans/2026-04-14-v2-phase2-plan.md` — Phase 2 参考模板
- `docs/plans/2026-04-16-v2-phase2_5-plan.md` — Phase 2.5 参考模板

---

*Design locked 2026-04-18 by /discuss skill session. Implementation plan: `docs/plans/2026-04-18-v2-phase3-plan.md`.*
