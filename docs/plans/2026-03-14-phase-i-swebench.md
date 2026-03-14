# Phase I — 外部 Benchmark 适配层

**日期**: 2026-03-14
**前置**: Phase H COMPLETE（resilience suite + AstMatch + context 扩充）
**目标**: 建立可插拔的外部 Benchmark 适配架构，适配 GAIA、SWE-bench、τ-bench 三大核心 benchmark，补全评估层次模型中 Level 3（多步推理+工具编排）和 Level 4（端到端代码修复）的空白

---

## 背景

### 为什么不只做 SWE-bench？

octo 的定位是**企业内网自主智能体平台**，支持长时自主决策、多工具编排、企业系统集成等场景。SWE-bench 只覆盖代码修复一个维度。评估体系需要覆盖完整的能力谱系：

```
Level 4: 端到端任务成功率 (SWE-bench)     → ❌ 未实现
Level 3: 多轮对话+工具链协调 (GAIA/τ-bench) → ❌ 完全缺失
Level 2: 单次工具调用精确度 (BFCL)          → ✅ 已实现 (50 tasks)
Level 1: 引擎基础能力 (单元测试)             → ✅ 1979 tests
```

### 当前 Benchmark 格局分析（2026-03）

| Benchmark | 评估维度 | SOTA | 对 octo 适配度 | 本次适配 |
|-----------|---------|------|---------------|---------|
| **BFCL** | 函数调用准确率 | ~77.5% | 核心 | ✅ 已有 |
| **GAIA** | 多步推理+多工具 | ~61% L3 | **高** | **本次** |
| **SWE-bench** | 代码修复 E2E | ~80.9% | **核心** | **本次** |
| **τ-bench** | 多轮工具一致性 | pass^8 <25% | **高** | **本次** |
| AgentBench | 综合 8 环境 | 多维 | 中 | 未来 |
| OSWorld | GUI 自动化 | ~38% | 低→未来 | 未来 |
| LiveCodeBench | 无污染代码生成 | ~86% | 中 | 未来 |
| Terminal-Bench | 终端任务 | ~77% | 中 | 未来 |
| CUB | 企业工作流 | ~10.4% | 未来 | 未来 |

### 关键洞察

1. **Scaffold > Model**: 同一模型在不同 agent 框架下 SWE-bench 差 22 个百分点。octo 的核心竞争力在框架/引擎层面。
2. **SWE-bench 正在饱和**: Verified 子集头部 80%+。SWE-bench Pro (~45%) 才有信号。
3. **τ-bench 暴露一致性**: pass^1 80% 但 pass^8 <25%。企业最在意行为可靠性。
4. **GAIA 是通用 agent 试金石**: 多步推理+多工具协同，最能体现 octo 的 Context Engineering 差异化。

---

## 一、架构设计

### ExternalBenchmark 抽象层

```rust
/// 外部 Benchmark 适配器 trait — 所有外部 benchmark 共享的接口
pub trait ExternalBenchmark: Send + Sync {
    /// Benchmark 名称 (e.g., "swe_bench", "gaia", "tau_bench")
    fn name(&self) -> &str;

    /// 人类可读描述
    fn description(&self) -> &str;

    /// 加载任务集
    fn load_tasks(&self) -> Result<Vec<Box<dyn EvalTask>>>;

    /// 是否需要特殊沙箱环境 (Docker, VM, etc.)
    fn requires_sandbox(&self) -> bool { false }

    /// 沙箱是否可用（运行时检查）
    fn sandbox_available(&self) -> bool { true }

    /// 自定义验证器（覆盖默认 task.score()）
    fn custom_verifier(&self) -> Option<Box<dyn BenchmarkVerifier>> { None }

    /// 自定义评估指标（如 τ-bench 的 pass^k）
    fn custom_metrics(&self) -> Vec<MetricDefinition> { vec![] }
}

/// 验证器 trait — 用于需要外部验证的 benchmark (如 SWE-bench Docker)
pub trait BenchmarkVerifier: Send + Sync {
    fn verify(&self, task: &dyn EvalTask, output: &AgentOutput)
        -> Pin<Box<dyn Future<Output = EvalScore> + Send + '_>>;
}

/// Benchmark 注册表
pub struct BenchmarkRegistry {
    benchmarks: HashMap<String, Box<dyn ExternalBenchmark>>,
}

impl BenchmarkRegistry {
    pub fn with_defaults() -> Self {
        let mut reg = Self::new();
        reg.register(Box::new(GaiaBenchmark::new()));
        reg.register(Box::new(SweBenchmark::new()));
        reg.register(Box::new(TauBenchmark::new()));
        reg
    }
}
```

### 整体流程

```
外部 JSONL 数据集
       │
       ▼
 ExternalBenchmark::load_tasks()    ── 数据格式适配
       │
       ▼
 Vec<Box<dyn EvalTask>>            ── 统一 EvalTask 接口
       │
       ▼
 EvalRunner::run_suite()           ── 标准评估管线
       │
       ├── BenchmarkVerifier (if custom)  ── SWE-bench Docker 验证
       └── task.score() (default)         ── GAIA ExactMatch
       │
       ▼
 EvalReport + 自定义 Metrics       ── 含 pass^k 等特殊指标
```

---

## 二、任务分组

### I1: ExternalBenchmark 抽象层 (架构基础)

**I1-T1: 定义 ExternalBenchmark trait + BenchmarkVerifier trait**

新文件: `crates/octo-eval/src/benchmarks/mod.rs` (~80 行)

内容:
- `ExternalBenchmark` trait 定义
- `BenchmarkVerifier` trait 定义
- `BenchmarkRegistry` 注册表
- `MetricDefinition` 自定义指标定义

**I1-T2: 集成到 CLI 和 Runner**

修改: `crates/octo-eval/src/main.rs` (~30 行)

内容:
- `load_suite()` 中添加 benchmark registry 查找
- `cmd_list_suites()` 中分区显示内部/外部 suites
- `cmd_run_direct_suite()` 中添加 benchmark 直跑支持

### I2: GAIA 适配 (Level 3 — 多步推理+多工具)

> **优先级最高**: 实施最简单、价值最大。不需要 Docker，直接验证 Agent Loop 多工具编排能力。

**I2-T1: GAIA 数据加载器**

新文件: `crates/octo-eval/src/benchmarks/gaia.rs` (~150 行)

GAIA 任务格式:
```json
{
    "task_id": "gaia-L1-001",
    "question": "How many studios were involved in making the film...",
    "final_answer": "3",
    "level": 1,
    "annotator_metadata": {
        "steps": "1. Search for the film... 2. Count studios...",
        "tools": ["web_search"],
        "num_steps": 2
    }
}
```

内容:
- `GaiaRecord` 结构体 — 解析 GAIA JSONL 格式
- `GaiaTask` — 实现 `EvalTask` trait
- `GaiaBenchmark` — 实现 `ExternalBenchmark` trait
- 评分: ExactMatch (已有) + `GaiaExactMatch` ScoreDetails 变体 (含 level)
- 难度映射: GAIA Level 1→Easy, Level 2→Medium, Level 3→Hard

**I2-T2: GAIA 数据集**

新文件: `crates/octo-eval/datasets/gaia_sample.jsonl` (~50 tasks)

分布:
| Level | 任务数 | 说明 |
|-------|--------|------|
| L1 | 20 | 单步/少工具，简单推理 |
| L2 | 20 | 多步+多工具协作 |
| L3 | 10 | 复杂长链推理 |

测试: 2 个测试 (JSONL 加载 + 难度分类)

### I3: SWE-bench 适配 (Level 4 — 端到端代码修复)

> 行业金标准。需要 Docker 沙箱验证，无 Docker 时 mock 降级。

**I3-T1: SWE-bench 数据加载器**

新文件: `crates/octo-eval/src/benchmarks/swe_bench.rs` (~200 行)

SWE-bench 任务格式:
```json
{
    "instance_id": "django__django-16527",
    "repo": "django/django",
    "base_commit": "abc123...",
    "patch": "diff --git a/...",
    "test_patch": "diff --git a/...",
    "problem_statement": "Issue 描述...",
    "hints_text": "...",
    "fail_to_pass": "[\"test_xxx\"]",
    "pass_to_pass": "[\"test_yyy\"]"
}
```

内容:
- `SweBenchRecord` 结构体
- `SweBenchTask` — 实现 `EvalTask` trait
- `SweBenchmark` — 实现 `ExternalBenchmark` trait
- `classify_swe_difficulty()` — 按 patch 大小和测试数量分类难度
- `swe_bench_tools()` — SWE-bench 评估可用工具集 (bash, file_read, file_write)

测试: 2 个测试 (JSONL 加载 + 难度分类)

**I3-T2: SWE-bench 验证器**

新文件: `crates/octo-eval/src/swe_verifier.rs` (~200 行)

内容:
- `SweVerifier` — 实现 `BenchmarkVerifier` trait
- `SweVerifier::verify(record, agent_patch)` — Docker 沙箱内验证:
  1. Clone repo @ base_commit
  2. Apply test_patch
  3. Apply agent patch
  4. Run FAIL_TO_PASS tests → must pass
  5. Run PASS_TO_PASS tests → must still pass
- `SweVerifier::verify_with_gold(record)` — 用 gold patch 验证管线 (mock 模式)
- `extract_patch_from_output(output)` — 从 agent 输出中提取 diff

降级策略:
- Docker 不可用: 仅验证 JSONL 加载和 patch 格式，跳过容器化验证
- CI: 通过环境变量 `DOCKER_AVAILABLE` 条件执行

测试:
- 1 个单测: patch 提取逻辑
- 1 个集成测试 (需 Docker): gold patch 验证

**I3-T3: SWE-bench 数据集**

新文件: `crates/octo-eval/datasets/swe_bench_lite.jsonl` (~50 tasks)

选择标准:
- 仅 Python 项目
- 单文件修改优先
- patch < 100 行
- 覆盖 5+ 不同仓库

分布:
| 仓库 | easy | medium | hard | 合计 |
|------|------|--------|------|------|
| django | 3 | 5 | 2 | 10 |
| flask | 3 | 3 | 1 | 7 |
| sympy | 2 | 4 | 2 | 8 |
| requests | 3 | 3 | 1 | 7 |
| pytest | 2 | 3 | 2 | 7 |
| 其他 | 3 | 5 | 3 | 11 |
| **合计** | **16** | **23** | **11** | **50** |

### I4: τ-bench 适配 (Level 3 — 多轮工具一致性)

> 验证 Agent 行为可靠性。pass^k 指标暴露智能体行为不一致性 — 企业最在意的维度。

**I4-T1: τ-bench 数据加载器和验证器**

新文件: `crates/octo-eval/src/benchmarks/tau_bench.rs` (~180 行)

τ-bench 任务格式:
```json
{
    "task_id": "tau-retail-001",
    "domain": "retail",
    "user_instruction": "I want to return my order #12345...",
    "policy_rules": ["Returns within 30 days...", "Must have receipt..."],
    "available_tools": ["lookup_order", "process_return", "send_email"],
    "expected_actions": [
        {"tool": "lookup_order", "args": {"order_id": "12345"}},
        {"tool": "process_return", "args": {"order_id": "12345", "reason": "..."}}
    ],
    "expected_db_state": {"order_12345_status": "returned"},
    "k": 8
}
```

内容:
- `TauBenchRecord` 结构体
- `TauBenchTask` — 实现 `EvalTask` trait
- `TauBenchmark` — 实现 `ExternalBenchmark` trait
- `TauVerifier` — 多轮对话模拟 + pass^k 计算
- `PassKCalculator` — 统计连续 k 次执行的一致通过率

新文件: `crates/octo-eval/src/tau_verifier.rs` (~150 行)

测试: 2 个测试 (JSONL 加载 + pass^k 计算逻辑)

**I4-T2: τ-bench 数据集**

新文件: `crates/octo-eval/datasets/tau_bench_retail.jsonl` (~30 tasks)

分布:
| 领域 | 任务数 | 说明 |
|------|--------|------|
| 零售退货 | 10 | 退货流程+政策遵守 |
| 零售查询 | 10 | 订单状态+库存查询 |
| 零售修改 | 10 | 订单修改+地址变更 |

### I5: 验证与收尾

**I5-T1: ScoreDetails 新增变体**

修改: `crates/octo-eval/src/score.rs` (~25 行)

```rust
// 新增 3 个 ScoreDetails 变体
SweVerify {
    instance_id: String,
    fail_to_pass_passed: bool,
    pass_to_pass_passed: bool,
    fail_to_pass_count: usize,
    pass_to_pass_count: usize,
    execution_time_ms: u64,
},
PassK {
    k: u32,
    passes: u32,
    pass_at_1: f64,
    pass_at_k: f64,
},
GaiaMatch {
    expected: String,
    actual: String,
    level: u32,
},
```

**I5-T2: CLI 更新**

修改: `crates/octo-eval/src/main.rs` (~40 行)

- 分区显示: 内部 suites vs 外部 benchmarks
- `load_suite()` 中添加 gaia / swe_bench / tau_bench
- `cmd_run_direct_suite()` 中添加 benchmark 直跑支持

**I5-T3: eval-ci.yml 更新**

修改: `.github/workflows/eval-ci.yml` (~15 行)

```yaml
- name: Run GAIA benchmark (mock mode)
  run: cargo run -p octo-eval -- run --suite gaia

- name: Run SWE-bench (requires Docker)
  if: env.DOCKER_AVAILABLE == 'true'
  run: cargo run -p octo-eval -- run --suite swe_bench

- name: Run τ-bench (mock mode)
  run: cargo run -p octo-eval -- run --suite tau_bench
```

**I5-T4: 全量测试**

```bash
cargo test --workspace -- --test-threads=1
```

---

## 三、文件改动矩阵

| 文件 | 操作 | 行数估计 |
|------|------|---------|
| `src/benchmarks/mod.rs` | **新建** | ~80 |
| `src/benchmarks/gaia.rs` | **新建** | ~150 |
| `src/benchmarks/swe_bench.rs` | **新建** | ~200 |
| `src/benchmarks/tau_bench.rs` | **新建** | ~180 |
| `src/swe_verifier.rs` | **新建** | ~200 |
| `src/tau_verifier.rs` | **新建** | ~150 |
| `src/score.rs` | 修改 | +25 |
| `src/lib.rs` | 修改 | +2 |
| `src/main.rs` | 修改 | +40 |
| `datasets/gaia_sample.jsonl` | **新建** | ~50 tasks |
| `datasets/swe_bench_lite.jsonl` | **新建** | ~50 tasks |
| `datasets/tau_bench_retail.jsonl` | **新建** | ~30 tasks |
| `.github/workflows/eval-ci.yml` | 修改 | +15 |
| **总计** | **9 新文件, 4 修改** | **~1,100 行** |

---

## 四、依赖

- **Docker daemon**: SWE-bench 验证必须有 Docker（Phase J 会修复）
- **网络访问**: SWE-bench 需要 clone GitHub 仓库到容器内
- **磁盘**: SWE-bench 每个仓库 clone ~100MB-1GB

### 降级策略

| Benchmark | Docker 不可用 | 网络不可用 |
|-----------|-------------|-----------|
| GAIA | 不影响 | 不影响（本地数据） |
| SWE-bench | mock 模式（仅验证 JSONL 加载） | mock 模式 |
| τ-bench | 不影响 | 不影响（本地数据） |

---

## 五、执行顺序

```
I1 (架构层)
  │
  ├── I2 (GAIA) ──┐
  ├── I3 (SWE)  ──┼── 可并行
  └── I4 (τ-bench)┘
          │
          ▼
        I5 (验证)
```

I2/I3/I4 之间无依赖，可并行开发。I1 必须先完成（定义共享 trait）。

---

## 六、验收标准

- [ ] `ExternalBenchmark` trait 和 `BenchmarkRegistry` 正确实现
- [ ] `gaia_sample.jsonl` 包含 50 个任务，覆盖 L1-L3
- [ ] `swe_bench_lite.jsonl` 包含 50 个任务，覆盖 5+ 仓库
- [ ] `tau_bench_retail.jsonl` 包含 30 个任务
- [ ] `cargo run -p octo-eval -- list-suites` 分区显示内部/外部 suites
- [ ] `cargo run -p octo-eval -- run --suite gaia` mock 模式可运行
- [ ] `cargo run -p octo-eval -- run --suite swe_bench` mock 模式可运行
- [ ] `cargo run -p octo-eval -- run --suite tau_bench` mock 模式可运行
- [ ] Docker 不可用时 SWE-bench 优雅降级
- [ ] ScoreDetails 新增 SweVerify / PassK / GaiaMatch 变体
- [ ] `cargo test --workspace -- --test-threads=1` 全部通过

---

## 七、与未来 Phase 的关系

```
Phase I (本次): ExternalBenchmark 架构 + GAIA + SWE-bench + τ-bench
     ↓
Phase J: Docker 测试修复 → SWE-bench 从 mock 升级为真实验证
     ↓
Phase K: 模型报告 → 跨 GAIA/SWE-bench/τ-bench 的多模型对比
     ↓
Phase L (未来):
  ├── LiveCodeBench 适配 (无污染代码生成)
  ├── AgentBench 适配 (多环境综合)
  ├── OSWorld 适配 (电脑操作) — 需要 VM 基础设施
  ├── CUB 适配 (企业工作流)
  └── Terminal-Bench 适配 (终端操作)
```
