# Phase R — 使用筛选策略的 GAIA 评估重测

> 创建日期: 2026-03-17
> 前置: Phase Q (GAIA/SWE-bench Standard Testing) 完成

## 目标

根据 Phase Q 深度分析中制定的数据筛选策略，过滤掉 Agent 能力盲区任务（图片/音频/视频），
使用筛选后的数据集重新运行 GAIA 评估，获取更准确的基线数据。

## 背景

Phase Q 的 GAIA R2 评估（30 任务采样）发现：
- 30 题中有 13 题（43%）全模型零通过，其中约 5 题因能力盲区（OCR/视频/音频）而非推理不足
- 全 165 题数据集中，**16 题**属于能力盲区（图片 10、音频 3、pptx 1、YouTube 视频 9，去重后 16+）
- 筛选后保留 **149 题**（L1:46, L2:78, L3:25），有效区分度更高

### 筛选规则

| 排除条件 | 排除数 | 原因 |
|----------|:------:|------|
| `.png` / `.jpg` / `.jpeg` / `.gif` 附件 | 10 | 无 OCR/多模态视觉 |
| `.mp3` 附件 | 3 | 无音频转写 |
| `.pptx` 附件 | 1 | 需要幻灯片解析 |
| 问题含 `youtube.com` URL | 9 | 无视频理解（部分与上面重叠） |
| **去重后总排除** | **16** | |

筛选后: 165 → **149 题** (L1:46, L2:78, L3:25)

---

## 任务分解

### G1: 实现数据集筛选 (2 tasks)

#### T1: GaiaBenchmark 添加筛选逻辑
- 在 `gaia.rs` 的 `load_tasks()` / `load_from_jsonl()` 中添加过滤
- 过滤条件: file_name 后缀 ∈ {png, jpg, jpeg, gif, mp3, pptx} 或 question 含 `youtube.com`
- 添加 `GaiaBenchmark::with_filter()` 构造方法
- 新增测试: 验证过滤逻辑
- **文件**: `crates/octo-eval/src/benchmarks/gaia.rs`

#### T2: benchmark.toml 更新筛选配置
- 在 `[gaia]` 部分添加 `exclude_file_extensions` 和 `exclude_question_patterns`
- 更新 `full = 149`（筛选后总数）
- 更新 `standard = 30` 采样数保持不变
- **文件**: `config/eval/benchmark.toml`

### G2: 生成筛选后数据集 (1 task)

#### T3: 创建筛选后 GAIA 数据集文件
- 使用 Python 脚本从 `gaia_sample.jsonl` 过滤生成 `gaia_filtered.jsonl`
- 验证: 149 条记录，Level 分布正确
- 两种方案择一:
  - A) 运行时过滤（在 Rust 代码中过滤） ← **优先**
  - B) 预生成新 JSONL 文件
- 如选 A，无需额外文件，T1 完成即可
- **文件**: `crates/octo-eval/datasets/gaia_filtered.jsonl`（仅方案 B）

### G3: 运行 GAIA 评估 (2 tasks)

#### T4: 创建评估配置文件
- 创建 `config/eval/gaia_r3.toml`
- 4 个模型 × 全量 149 题（或 standard 30 题采样）
- 使用 Tavily 搜索（复用 R2 配置）
- 配置 `max_iterations=30`, `timeout=300s`
- **文件**: `config/eval/gaia_r3.toml`

#### T5: 执行 GAIA R3 评估
- 运行: `cargo run -p octo-eval -- benchmark --config config/eval/gaia_r3.toml`
- 等待完成，收集结果
- 输出: `eval_output/runs/2026-03-17-0XX/`
- **运行时任务**: 需要用户确认后执行

### G4: 结果分析与报告 (2 tasks)

#### T6: 分析 R3 vs R2 对比
- 对比筛选后结果与 R2（未筛选 30 题采样）
- 按 Level 分级分析通过率变化
- 按模型分析: 筛选策略对哪些模型影响最大
- 生成对比表格

#### T7: 更新基线报告
- 更新 `docs/design/EVAL_STANDARD_BENCHMARK_REPORT.md`
- 添加 R3 结果段落
- 更新筛选策略章节（从提议 → 已实施）
- 更新改进路线图

---

## 依赖关系

```
T1 (filter logic) → T3 (dataset) → T4 (config) → T5 (run eval)
T2 (toml update) ────────────────→ T4
T5 (results) → T6 (analysis) → T7 (report)
```

## 风险

1. **API 额度**: 149×4=596 次 LLM 调用 + Tavily 搜索，需确认额度充足
2. **运行时间**: 全量 149 题约需 3-5 小时；standard 30 题约需 40-60 分钟
3. **筛选后通过率可能下降**: 排除了盲区任务后，剩余任务平均难度可能更高

## 成功标准

- [ ] GAIA 筛选逻辑正确实现并通过测试
- [ ] R3 评估完成，结果可与 R2 对比
- [ ] 基线报告更新，包含筛选策略实施记录
