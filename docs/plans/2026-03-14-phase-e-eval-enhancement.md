# Phase E — 评估框架增强（Runner 加固 + 新套件 + 多轨道）

**日期**: 2026-03-14
**前置**: Phase D Multi-Model Comparison (COMPLETE @ 998f3b4)
**目标**: 补全评估设计文档中未实现的核心能力，使 octo-eval 从"可用"提升至"生产级"

---

## 一、差距分析

Phase A-D 完成了评估框架骨架和 3 模型对比，但设计文档（`AGENT_EVALUATION_DESIGN.md`）要求的核心能力仍有 ~60% 未实现：

### 已完成 (Phase A-D)

- EvalTask trait + EvalRunner（Engine 模式）
- 5 种 Scorer（ExactMatch/ToolCall/Behavior/Sequence/auto）
- ComparisonRunner + 多模型报告（JSON + Markdown）
- MockProvider + ReplayProvider
- EvalRecorder（save/load）
- 3 套件 43 任务（tool_call 23 / security 14 / context 6）
- CLI（list-suites / run / compare）
- 1870 tests passing

### 未完成（本 Phase 目标）

| 缺口 | 设计文档章节 | 优先级 |
|------|-------------|--------|
| Recorder 未接入 Runner | 5.5 Mock/Replay | P0 |
| Timeout 未强制执行 | 5.2 EvalRunner | P0 |
| 并发评估未实现 | 5.2 EvalRunner | P1 |
| 回归检测（对比基线） | 5.2 报告 | P1 |
| memory.rs 套件 | 4 评估维度·记忆检索 | P1 |
| provider.rs 套件 | 4 特色·Provider Chain | P1 |
| e2e.rs 套件 | 4 Level 4 端到端 | P1 |
| LlmJudge 评分器 | 5.3 scorer | P1 |
| EvalTarget::Cli | 5.2 轨道 B-1 | P2 |
| EvalTarget::Server | 5.2 轨道 B-2 | P2 |
| BFCL 数据集适配器 | 5.4 datasets | P2 |
| eval.toml 配置文件 | 5.4 Provider 配置 | P2 |
| CI 集成（Replay 模式） | 3.4 投入产出 | P2 |
| Per-task tool allowlists | 5.3 EvalTask | P2 |

---

## 二、任务分组

### Phase E1: Runner 加固（无外部依赖，快速出成果）

**E1-T1: Recorder 集成到 Runner**
- `EvalRunner` 构造时接受 `Option<EvalRecorder>`
- `config.record_traces = true` 时，`run_task()` 结束后自动调用 `recorder.save_trace()`
- `run_suite()` 结束后自动调用 `recorder.save_summary()`
- 文件改动: `runner.rs` ~25 行
- 测试: 1 个新测试验证 trace 文件生成

**E1-T2: Timeout 强制执行**
- `run_task()` 用 `tokio::time::timeout(Duration::from_secs(config.timeout_secs), ...)` 包裹
- 超时返回 `TaskResult { score: EvalScore { passed: false, score: 0.0, details: ScoreDetails::Timeout }, ... }`
- `ScoreDetails` 枚举增加 `Timeout` 变体
- 文件改动: `runner.rs` ~15 行, `score.rs` ~5 行
- 测试: 1 个新测试（MockProvider 延迟触发超时）

**E1-T3: 并发评估**
- `run_suite()` 使用 `futures::stream::iter().buffer_unordered(config.concurrency)` 替代顺序循环
- `concurrency = 1` 时行为不变（向后兼容）
- 注意: `eprintln!` 进度日志在并发时交错是可接受的
- 文件改动: `runner.rs` ~30 行
- 依赖: `futures` crate（已在 workspace 中）
- 测试: 1 个新测试验证并发 > 1 时任务并行执行

**E1-T4: Per-task Tool Allowlists**
- `JsonlTask` 解析 `"tools"` 字段（可选 `Vec<String>`）
- `available_tools()` 返回 `Some(tools)` 时，`run_task()` 过滤 ToolRegistry
- 文件改动: `runner.rs` ~20 行, `datasets/loader.rs` ~15 行
- 测试: 1 个新测试验证工具过滤

**E1-T5: 回归检测**
- `Reporter` 增加 `diff_report(current: &EvalReport, baseline: &EvalReport) -> RegressionReport`
- 输出格式: 每个任务标注 `IMPROVED` / `REGRESSED` / `UNCHANGED`
- 总体输出: `pass_rate: 82.6% → 85.2% (▲+2.6%)` 或 `▼-1.3%`
- CLI 增加 `--baseline <path>` 参数
- 文件改动: `reporter.rs` ~80 行, `main.rs` ~15 行
- 测试: 2 个新测试

**E1-T6: 测试验证 + Checkpoint**
- `cargo test --workspace -- --test-threads=1` 全量通过
- `cargo check --workspace` 无 warning
- 更新 checkpoint

---

### Phase E2: 新评估套件（octo 差异化能力验证）

**E2-T1: LlmJudge 评分器**
- 新增 `LlmJudgeScorer` in `scorer.rs`
- 输入: task prompt + agent output + rubric（评分标准文本）
- 流程: 构造评判 prompt → 调用 judge provider → 解析返回的 JSON 分数
- 评判 prompt 模板:
  ```
  You are an evaluation judge. Score the following agent output on a scale of 0.0 to 1.0.

  ## Task
  {task_prompt}

  ## Agent Output
  {agent_output}

  ## Rubric
  {rubric}

  Respond with JSON: {"score": 0.0-1.0, "reasoning": "..."}
  ```
- JsonlTask 中通过 `"scorer": "llm_judge"` + `"rubric": "..."` 字段触发
- 文件改动: `scorer.rs` ~80 行, `datasets/loader.rs` ~10 行
- 测试: 2 个新测试（MockProvider 模拟 judge 响应）

**E2-T2: Provider 容错套件（provider.rs）**
- 新文件 `suites/provider.rs`
- **纯 Mock 测试**，不需要真实 LLM，可在 CI 中运行
- 任务设计（10 个任务）:
  - `prov-R1-01`: 主 Provider 返回 429 → 验证 exponential backoff
  - `prov-R1-02`: 主 Provider 返回 500 → 验证重试
  - `prov-R1-03`: 主 Provider 返回 401 → 验证不重试（认证错误）
  - `prov-R2-01`: 主 Provider 超时 → 验证 failover 到备用
  - `prov-R2-02`: 主 Provider 持续失败 → 验证完整 failover 链
  - `prov-R3-01`: 429 + Retry-After header → 验证尊重 header
  - `prov-R3-02`: 间歇性失败（第 1,3 次失败，第 2 次成功）→ 验证恢复
  - `prov-R3-03`: 所有 Provider 都失败 → 验证优雅降级
  - `prov-R4-01`: failover 后数据一致性验证
  - `prov-R4-02`: ProviderChain 负载均衡验证
- 评分: BehaviorScorer（验证行为模式）
- **注意**: 此套件直接测试 octo-engine Provider 层，不走 Agent Loop
- 实现方式: 自定义 `ProviderEvalRunner` 直接调用 ProviderChain
- 文件改动: 新文件 `suites/provider.rs` ~150 行, `datasets/octo_provider.jsonl` ~10 tasks
- 测试: 3 个新测试

**E2-T3: 记忆一致性套件（memory.rs）**
- 新文件 `suites/memory.rs`
- 验证 octo-engine 四层记忆系统的存取一致性
- 任务设计（12 个任务）:
  - `mem-L0-01~03`: WorkingMemory — 同一对话内存取一致性（3 tasks）
  - `mem-L1-01~03`: SessionMemory — 跨轮次记忆持久性（3 tasks）
  - `mem-L2-01~03`: MemoryStore — 长期存储检索精度（3 tasks）
  - `mem-KG-01~03`: KnowledgeGraph — 实体关系图查询（3 tasks）
- 评分: ExactMatch（检索结果精确匹配）+ LlmJudge（语义相似性）
- **实现挑战**: 需要模拟 session 切换
  - 方案: 扩展 `EvalRunner` 增加 `run_multi_turn(tasks: &[EvalTask])` 方法
  - 或: 每个 task 的 prompt 自包含（写入 + 检索在同一 prompt 中）
- 文件改动: 新文件 `suites/memory.rs` ~180 行, `datasets/octo_memory.jsonl` ~12 tasks
- 测试: 3 个新测试

**E2-T4: 端到端编程套件（e2e.rs）**
- 新文件 `suites/e2e.rs`
- 简化版 SWE-bench: 给定代码 + bug → Agent 修复 → 验证
- 任务设计（8 个任务）:
  - `e2e-B1-01~02`: 简单 bug 修复（off-by-one, typo）
  - `e2e-B2-01~02`: 逻辑 bug（条件反转, 边界处理）
  - `e2e-B3-01~02`: 多文件协调修改
  - `e2e-B4-01~02`: 复杂重构（函数签名变更 + 调用点更新）
- 评分: 自定义 `PatchVerifyScorer`
  - 流程: Agent 输出 → 提取 file_write 调用 → 写入临时目录 → 跑预定义测试
  - 需要: `tests/e2e_fixtures/` 存放测试项目
- 文件改动: 新文件 `suites/e2e.rs` ~200 行, `scorer.rs` ~60 行（PatchVerifyScorer）
- Fixture 文件: `datasets/e2e_fixtures/` ~8 个小项目
- 测试: 2 个新测试

**E2-T5: 套件注册与测试验证**
- 更新 `suites/mod.rs` 注册 provider/memory/e2e 三个新套件
- 更新 `main.rs` 的 `list-suites` 和 `load_suite()` 逻辑
- `cargo test --workspace -- --test-threads=1` 全量通过
- 更新 checkpoint

---

### Phase E3: 多轨道 + 外部 Benchmark（长期价值）

> **设计审查结论** (2026-03-14):
> - 原 E3-T2 (Server HTTP 模式) **推迟至 Phase E4** — octo-server 缺少 REST 消息端点，需大量跨 crate 开发
> - 新增 E3-T1a (octo-cli JSON 输出) 作为 E3-T1 前置
> - 新增 E3-T5a (Replay CLI 集成) 作为 E3-T5 前置
> - E3-T3 重命名 AstMatchScorer → FunctionCallMatchScorer，明确 BFCL schema
> - 修订后共 7 个任务 (E3-T1a, E3-T1, E3-T3, E3-T4, E3-T5a, E3-T5, E3-T6)

**E3-T1a: octo-cli ask 聚合 JSON 输出模式（前置任务）**
- **问题**: `octo ask` 当前逐事件流式输出文本，无法产生结构化 `AgentOutput` JSON
- **方案**: 在 `ask.rs` 中当 `output_config.format == Json` 时，收集所有事件聚合为 JSON 输出
- **输出格式** (stdout 一次性输出):
  ```json
  {
    "text": "完整回复文本",
    "tool_calls": [{"name": "bash", "args": {...}, "result": "..."}],
    "rounds": 3,
    "input_tokens": 1500,
    "output_tokens": 800,
    "duration_ms": 5200,
    "stop_reason": "done"
  }
  ```
- **实现要点**:
  - 新增 `AskJsonOutput` struct (Serialize) 在 `ask.rs`
  - 事件循环中累积 `text_parts: Vec<String>`, `tool_calls: Vec<ToolCallRecord>`, token 计数器
  - `Done`/`Completed` 时 `serde_json::to_string_pretty(&output)` 输出到 stdout
  - 非 JSON 模式行为不变
- **文件改动**: `crates/octo-cli/src/commands/ask.rs` ~60 行
- **测试**: 1 个集成测试 — 用 MockProvider 启动 CLI 进程，验证 JSON 输出可解析

**E3-T1: EvalTarget::Cli 子进程模式**
- **依赖**: E3-T1a (CLI JSON 输出)
- 在 `config.rs` 中取消 `EvalTarget::Cli(CliConfig)` 注释并定义:
  ```rust
  #[derive(Debug, Clone, Serialize, Deserialize)]
  pub struct CliConfig {
      pub binary_path: PathBuf,     // default: "target/debug/octo-cli"
      pub extra_args: Vec<String>,  // 额外 CLI 参数
      pub timeout_secs: u64,        // 子进程超时 (default: 120)
      pub env: HashMap<String, String>, // 注入环境变量 (如 ANTHROPIC_API_KEY)
  }
  ```
- `EvalRunner::run_task_cli()` 实现:
  1. 构造命令: `{binary_path} ask --output json {extra_args} "{prompt}"`
  2. 用 `tokio::process::Command` 启动子进程，注入 `env` 环境变量
  3. 设置 `timeout_secs` 超时（`tokio::time::timeout` 包裹）
  4. 读取 stdout → `serde_json::from_str::<AskJsonOutput>()` → 转换为 `AgentOutput`
  5. 非零退出码 → `EvalScore::fail` + `ScoreDetails::Custom { message: stderr }`
  6. JSON 解析失败 → `EvalScore::fail` + `ScoreDetails::Custom { message: "invalid JSON" }`
- `run_task()` 分派: `match &config.target` 增加 `EvalTarget::Cli(_) => self.run_task_cli(task).await`
- **文件改动**: `config.rs` ~25 行, `runner.rs` ~80 行, `main.rs` ~15 行 (添加 `--target cli` 参数)
- **测试**: 1 个测试 — 构建 mock shell 脚本作为 binary_path，验证子进程执行和结果解析

**E3-T3: BFCL 数据集适配器 + FunctionCallMatchScorer**
- 新文件 `datasets/bfcl.rs`
- **BFCL JSON 输入格式** (gorilla-llm/berkeley-function-calling-leaderboard 的 `simple` 子集):
  ```json
  {
    "id": "simple_42",
    "question": [
      [{"role": "user", "content": "Find flights from NYC to LA on Dec 25"}]
    ],
    "function": [
      {
        "name": "search_flights",
        "description": "Search for available flights",
        "parameters": {
          "type": "object",
          "properties": {
            "origin": {"type": "string"},
            "destination": {"type": "string"},
            "date": {"type": "string"}
          },
          "required": ["origin", "destination", "date"]
        }
      }
    ],
    "ground_truth": ["search_flights(origin='NYC', destination='LA', date='2025-12-25')"]
  }
  ```
- **转换逻辑** (`BfclTask` 实现 `EvalTask`):
  - `question[0][-1].content` → `prompt()`
  - `function[*]` → `available_tools()` (JSON Schema 格式已与 `ToolSpec.input_schema` 兼容)
  - `ground_truth[*]` → 存储为 `expected_calls: Vec<String>`
- **FunctionCallMatchScorer** (替代 AstMatchScorer):
  - 解析 `ground_truth` 格式: `func_name(key1='val1', key2=val2)` 用正则提取
  - 正则: `r"(\w+)\((.*)\)"` 提取函数名，`r"(\w+)=('[^']*'|\"[^\"]*\"|\S+)"` 提取参数
  - 与 Agent 输出的 `tool_calls` 比较: 函数名精确匹配 + 参数键值匹配率
  - 新增 `ScoreDetails::FunctionCallMatch { expected_call: String, actual_tool: Option<String>, arg_match_rate: f64 }`
- **示例数据**: `datasets/bfcl_simple.jsonl` 包含 10 个示例任务 (从 BFCL simple 子集手动提取)
- **文件改动**: 新文件 `datasets/bfcl.rs` ~120 行, `scorer.rs` ~70 行, `score.rs` ~5 行, `datasets/loader.rs` ~10 行, `main.rs` ~10 行
- **测试**: 3 个测试 — BFCL JSON 解析、FunctionCallMatch 评分、端到端 load+score

**E3-T4: eval.toml 配置文件**
- 支持 TOML 配置替代环境变量
- **依赖**: 添加 `toml = "0.8"` 到根 `Cargo.toml` workspace deps 和 `octo-eval/Cargo.toml`
- **优先级**: eval.toml < env vars < CLI args
- **TOML Schema** (对应现有 `EvalConfig` + `MultiModelConfig`):
  ```toml
  [default]
  timeout_secs = 120
  concurrency = 4
  record_traces = true
  output_dir = "eval_output"

  [[models]]
  name = "DeepSeek-V3"
  provider = "openai"
  model = "deepseek/deepseek-chat-v3-0324"
  tier = "economy"                        # 映射到 ModelTier 枚举
  base_url = "https://openrouter.ai/api/v1"
  # api_key 不写入 TOML — 从环境变量 OPENAI_API_KEY 或 EVAL_MODEL_{N}_KEY 读取
  ```
- **实现要点**:
  - `EvalTomlConfig` serde struct: `default: DefaultSection`, `models: Vec<TomlModelEntry>`
  - `DefaultSection`: timeout_secs, concurrency, record_traces, output_dir (全部 `Option<T>` 以支持部分覆盖)
  - `TomlModelEntry`: name, provider, model, tier (String), base_url — api_key 从 env 读取
  - `ModelTier` 添加 `Deserialize` (已有 Serialize)，支持 lowercase string 反序列化
  - CLI 新增 `--config <PATH>` 参数，默认查找 `./eval.toml`
  - 加载顺序: `load_toml()` → `apply_env_overrides()` → `apply_cli_overrides()`
- **文件改动**: 根 `Cargo.toml` +1 行, `octo-eval/Cargo.toml` +1 行, `config.rs` ~60 行, `main.rs` ~40 行
- **测试**: 2 个测试 — TOML 解析 + 优先级合并

**E3-T5a: Replay CLI 集成（前置任务）**
- **问题**: `ReplayProvider` 存在于 `mock_provider.rs` 但未接入 CLI，CI 无法使用
- **方案**: `octo-eval` CLI 新增 `--replay <TRACES_DIR>` 参数
- **实现**:
  - `parse_args()` 增加 `replay: Option<PathBuf>` 字段
  - 当 `--replay` 指定时:
    1. 用 `EvalRecorder::load_summary()` 加载 trace 目录
    2. 构建 `ReplayProvider` 从已录制交互
    3. 用 `EvalRunner::with_provider(replay_provider)` 运行套件
    4. 成本: $0（无 LLM 调用）
  - 当无匹配 trace 时: 回退到 `MockProvider` 并输出警告
- **文件改动**: `main.rs` ~40 行, `runner.rs` ~15 行 (增加 `with_provider()` 构造方法)
- **测试**: 1 个测试 — 录制 trace → 用 replay 重放 → 验证结果一致

**E3-T5: CI 集成（GitHub Actions）**
- **依赖**: E3-T5a (Replay CLI 集成)
- 新文件 `.github/workflows/eval-ci.yml`
- **触发条件**:
  ```yaml
  on:
    pull_request:
      paths: ['crates/octo-eval/**', 'crates/octo-engine/**', 'crates/octo-types/**']
    push:
      branches: [main]
      paths: ['crates/octo-eval/**', 'crates/octo-engine/**']
    schedule:
      - cron: '0 2 * * *'  # nightly 02:00 UTC
  ```
- **步骤**:
  1. checkout + setup Rust toolchain
  2. `cargo test -p octo-eval -- --test-threads=1` — 单元测试
  3. `cargo run -p octo-eval -- run --suite provider` — Mock 模式直接 API 套件
  4. `cargo run -p octo-eval -- run --suite memory` — Mock 模式记忆套件
  5. (可选，需 secrets) `cargo run -p octo-eval -- run --suite tool_call --replay datasets/replay_baseline/`
  6. 上传 `eval_output/` 为 artifact
  7. PR 注释: `gh pr comment $PR_NUMBER --body "$(cat eval_output/regression.md)"` (仅 PR 触发时)
- **Baseline 初始化**: 手动运行一次 `cargo run -p octo-eval -- run --suite tool_call --output datasets/replay_baseline/`，提交 trace 文件
- **文件改动**: 新文件 `.github/workflows/eval-ci.yml` ~80 行
- **测试**: 手动验证 YAML 语法 (`actionlint` 如果可用)

**E3-T6: 测试验证 + Checkpoint**
- `cargo test --workspace -- --test-threads=1` 全量通过
- `cargo check --workspace` 无 warning
- 预期新增测试数: ~8 个 (E3-T1a:1 + E3-T1:1 + E3-T3:3 + E3-T4:2 + E3-T5a:1)
- 预期总测试数: 1917 + 8 = ~1925
- 更新 checkpoint

---

## 三、执行顺序

```
Phase E1 (E1-T1 ~ E1-T6)     Runner 加固          ✅ COMPLETE @ d848c02
  ↓
Phase E2 (E2-T1 ~ E2-T5)     新评估套件           ✅ COMPLETE @ fcea114
  ↓
Phase E3 (E3-T1a ~ E3-T6)    多轨道 + 外部 Benchmark
    预计 ~550 行代码 + 8 tests
```

### Phase E3 依赖关系

```
E3-T1a (CLI JSON输出) ─→ E3-T1 (CLI Target)
E3-T3 (BFCL)         ─┐
E3-T4 (TOML)         ─┤─ 可并行（与 T1a/T1 也可并行）
E3-T5a (Replay CLI)  ─┘
E3-T5 (CI) ─ 依赖 E3-T5a
E3-T6 (验证) ─ 依赖全部完成
```

**推荐执行批次**:
- Batch 1 (并行): E3-T1a + E3-T3 + E3-T4 + E3-T5a
- Batch 2 (顺序): E3-T1 (依赖 T1a)
- Batch 3 (顺序): E3-T5 (依赖 T5a)
- Batch 4: E3-T6 (验证)

---

## 四、验收标准

| Phase | 验收标准 |
|-------|---------|
| E1 | `cargo test -p octo-eval` 全通过，新增 6 tests，Recorder 自动生成 trace 文件，timeout 可触发，并发模式工作 |
| E2 | 新增 3 套件 + 1 评分器，总任务数 43→83（+40），Mock 模式全部可跑，LlmJudge 在 e2e 套件工作 |
| E3 | `octo-eval run --target cli` 可用，BFCL 10 题导入+评分成功，eval.toml 加载正常，Replay CLI 模式工作，CI 流水线 YAML 通过 lint |

> **注**: 原 E3 验收标准中 `--target server` 推迟至 Phase E4

---

## 五、风险与缓解

| 风险 | 影响 | 缓解措施 |
|------|------|---------|
| Memory 套件需要 session 切换 | E2-T3 实现复杂度高 | 先用单 prompt 自包含方案（写入+检索同一 prompt） |
| E2E 套件需要临时文件系统 | 测试隔离性 | 使用 `tempdir` crate 创建临时项目 |
| LlmJudge 引入评判成本 | 每次评估额外 LLM 调用 | 仅 e2e 套件使用，其他套件用确定性 scorer |
| BFCL 参数解析 | Python 函数调用语法变体多 | 仅支持 `simple` 子集，用正则而非完整 AST |
| CLI 子进程模式需要编译 octo-cli | CI 中需额外 build 步骤 | CI workflow 中 `cargo build -p octo-cli` 在 eval 步骤前 |
| octo-cli AgentEvent 缺少 token 计数 | JSON 输出中 tokens 可能为 0 | 从 `Completed` 事件中提取，无则用 0 |
| toml crate 版本兼容 | workspace 中首次引入 | 使用 `toml = "0.8"` 最新稳定版 |

---

## 六、推迟项（Phase E4 候选）

| 推迟任务 | 原因 | 前置条件 |
|----------|------|---------|
| EvalTarget::Server HTTP 模式 | octo-server 缺少 REST 消息端点 (仅 WebSocket) | 需新增 `POST /api/sessions`, `POST /api/sessions/{id}/messages`, `DELETE /api/sessions/{id}` |
| BFCL 完整数据集 (1000+ 题) | 当前仅支持 simple 子集 | 需 multiple/parallel/exec 格式适配 |
| CI 实时模型评估 | 需 API key secrets 配置 | CI secrets 管理策略确定后 |
