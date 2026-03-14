# octo-sandbox 下一会话指南

**最后更新**: 2026-03-14 19:15 GMT+8
**当前分支**: `main`
**当前状态**: Phase A-H COMPLETE, Phase I IN PROGRESS (外部 Benchmark 适配层)

---

## 项目状态：外部 Benchmark 适配层设计完成，进入实现阶段

评估框架 Phase A-H 全部完成。1979 tests passing @ `37680ec`。
octo-eval 已具备 10 个 Suite、11 种 Scorer、~248 评估任务、3 种运行轨道、11 种行为类型。

Phase I 已从"只做 SWE-bench"扩展为**外部 Benchmark 适配层**，一次性适配 GAIA、SWE-bench、τ-bench 三大核心 benchmark。

### 完成清单

| 阶段 | Tasks | 状态 | Commit |
|------|-------|------|--------|
| Wave 1-10: v1.0-v1.1 | 全部 | COMPLETE | `675155d` |
| Phase A: 轨道 A 特色评估 | 8/8 | COMPLETE | `e490da3` |
| Phase B: 评估手册 | 全部 | COMPLETE | `90017dc` |
| Phase C: octo-eval crate | 全部 | COMPLETE | `24b02d4` |
| Phase D: 多模型对比 | 10/10 | COMPLETE | `998f3b4` |
| Phase E: 评估增强 | 18/18 | COMPLETE | `3e11905` |
| Phase F: 评估任务集 | 20/23 | COMPLETE | `b4d1cd2` |
| Phase G: Deferred 补齐 | 9/9 | COMPLETE | `ca5c898` |
| Phase H: 评估收官 | 10/10 | COMPLETE | `37680ec` |
| **Phase I: 外部 Benchmark** | **0/13** | **IN PROGRESS** | — |
| Phase J: Docker 修复 | 0/8 | PLANNED | — |
| Phase K: 模型报告 | 0/10 | PLANNED | — |

---

## 当前工作：Phase I — 外部 Benchmark 适配层

### 设计变更说明

原 Phase I 仅适配 SWE-bench (12 tasks)。经 brainstorming 分析后扩展为：

1. **ExternalBenchmark 抽象层** — 可插拔的外部 benchmark 适配架构
2. **GAIA** — Level 3 多步推理+多工具编排 (50 tasks, 无需 Docker)
3. **SWE-bench** — Level 4 端到端代码修复 (50 tasks, 需 Docker)
4. **τ-bench** — Level 3 多轮工具一致性 + pass^k (30 tasks, 无需 Docker)

### 任务分组

```
I1: ExternalBenchmark trait + registry (架构基础)
  │
  ├── I2: GAIA 适配 (最简单, 价值最大) ──┐
  ├── I3: SWE-bench 适配 (行业金标准)  ──┼── 可并行
  └── I4: τ-bench 适配 (一致性度量)    ──┘
          │
          ▼
        I5: 验证 + CI + ScoreDetails
```

### 计划文档

| Phase | 文件 | 内容 |
|-------|------|------|
| **I** | `docs/plans/2026-03-14-phase-i-swebench.md` | 外部 Benchmark 适配层 (已修订) |
| J | `docs/plans/2026-03-14-phase-j-docker-tests.md` | Docker 测试修复 |
| K | `docs/plans/2026-03-14-phase-k-model-benchmark.md` | 多模型对比报告 |

### 关键代码路径

| 组件 | 文件 | 说明 |
|------|------|------|
| 抽象层 | `src/benchmarks/mod.rs` | ExternalBenchmark + Registry |
| GAIA | `src/benchmarks/gaia.rs` | GaiaRecord + GaiaTask + GaiaBenchmark |
| SWE-bench | `src/benchmarks/swe_bench.rs` | SweBenchRecord + SweBenchTask |
| SWE 验证 | `src/swe_verifier.rs` | Docker 沙箱验证 |
| τ-bench | `src/benchmarks/tau_bench.rs` | TauBenchRecord + TauBenchTask |
| τ 验证 | `src/tau_verifier.rs` | pass^k 计算 |
| 分数 | `src/score.rs` | 新增 SweVerify/PassK/GaiaMatch |

---

## 基线

- **Tests**: 1979 passing @ `37680ec`
- **评估任务**: ~248 个 (10 Suite, 11 Scorer, 11 Behavior)
- **运行轨道**: Engine / CLI / Server
- **测试命令**: `cargo test --workspace -- --test-threads=1`
- **LLM 配置**: `.env` 中 OpenRouter 端点，不需要额外配置

## 启动命令

```bash
# 继续 Phase I 实现
/dev-phase-manager:resume-plan
```
