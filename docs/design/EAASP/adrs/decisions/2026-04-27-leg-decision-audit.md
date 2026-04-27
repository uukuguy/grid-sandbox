# Phase 4 Leg 决策审计 — 2026-04-27

Phase 4.1 audit, 系统化对照 ADR-V2-023 §P5 4 条 Leg B 激活触发条件 + Phase 4.1 baseline §F audit agenda Q1-Q4 4 个具体决策门, 产出每条 audit point 的 yes/no/partial/unknown verdict + evidence, 喂 Phase 4.2 ADR-V2-024 Decision 段。

**输入**:
- `.planning/phases/4.1-PRE-AUDIT-NOTES.md` §A-§F (baseline + audit agenda)
- `docs/external-review/2026-04-26-eaasp-skill-spec-coverage-internal.md` (30 项 MANDATORY 4/7/13/6 项 🟢/🟡/🔴/⚫ 分布)
- `docs/design/EAASP/adrs/ADR-V2-023-grid-two-leg-product-strategy.md` §P5 (L155-162)

**输出**: 8 audit-point verdict + 三选一推荐 (Leg A 硬化 / Leg B 激活 / 两腿都推) + 框架修订建议附加段 + Open Items 待 user 补段。

---

## §1. Summary

(verdict 计数等到 §2 §3 完成后回填; T2 末尾 commit 时已有 2/4 §P5 verdict, T4 末尾 commit 时 8/8 完整)

| 维度 | 计数 |
|------|------|
| §P5 trigger yes | (待回填) |
| §P5 trigger no | (待回填) |
| §P5 trigger partial | (待回填) |
| §P5 trigger unknown | (待回填) |
| §F Q yes | (待回填) |
| §F Q no | (待回填) |
| §F Q partial | (待回填) |
| §F Q unknown | (待回填) |

三选一推荐结论: (待 §4 完成回填)

---

## §2. §P5 Trigger 审计

### §2.0 Verdict 总表

| Trigger | 原文摘要 | Verdict | Evidence 类型 | Cross-ref §F |
|---------|---------|---------|--------------|--------------|
| §P5.1 | EAASP 项目延期/停滞/定价不合理 | partial (语义弱化) | ADR cross-ref + user 陈述 (PRE-AUDIT-NOTES §C.2) | §F.Q1 (EAASP 调度 vertical 决策同源) |
| §P5.2 | ≥2 客户要求 Grid 独立买 | unknown (待 user 补) | user 陈述待补 | §F.Q4 (Grid vs EAASP 优先级) |
| §P5.3 | 行业 agent runtime 标准出现 | (T2 填) | (T2 填) | (T2 填) |
| §P5.4 | 团队富余 ≥30% | (T2 填) | (T2 填) | (T2 填) |

### §2.1 Trigger 1 — EAASP 项目延迟/停滞/定价不合理

**Verdict**: partial (语义弱化)

**Confidence**: medium  <!-- per Fix #2: user 陈述 + ADR cross-ref 有, 但无独立 corroboration -->
**Corroboration**: yes  <!-- per Fix #2: PRE-AUDIT-NOTES §A.1 + §C.2 双源 + ADR-V2-023 L156 cross-ref 三向一致 -->
**Criticality**: influences  <!-- per Fix #2: 措辞调整影响 §4 推荐 framing, 不直接 block -->
**ADR-block**: no  <!-- per Fix #2: partial verdict, 不是 unknown, ADR-V2-024 Accepted 不被此条阻塞 -->

**Evidence**:
- ADR cross-ref: ADR-V2-023 §P5 第 1 条 (L156) 假设 EAASP 是外部上游 — verbatim "EAASP 项目成熟度变化（延迟、停滞、定价不合理等），导致 Grid 需要独立交付能力作为风险对冲"
- User 陈述 (PRE-AUDIT-NOTES §A.1): EAASP 当前**同仓孵化**, 是自家研究版, 未来分仓时点未定 — 此 trigger 措辞前提("外部上游 EAASP")与现状("自评内部 EAASP")不符
- User 陈述 (PRE-AUDIT-NOTES §C.2): 显式说明 "第 1 条变成自评 (自己控制的 EAASP 节奏), 语义弱化"

**Cross-ref §F**: 此 trigger 在 §F 框架下对应 §F.Q1 (EAASP 是否做调度行业 vertical) 的"EAASP 节奏指标"那一面 — 弱化的不是"是否激活 Leg B", 而是"是否要 EAASP engine 大幅扩展"。同时部分对应 §F.Q4 (Grid vs EAASP 优先级) — 当 EAASP 节奏由 user 自控时, "项目延迟"作为外部风险信号弱化。

**Recommendation note**: 此 trigger **不**简单标 N/A; **作为外部风险评估已弱化, 但作为自评内部节奏指标可保留** (per CONTEXT.md D-B-03 锁定措辞)。Phase 4.2 ADR-V2-024 重新框定时建议把此条改写为"EAASP 内部节奏门"(自评工时 / 进度延期超阈触发反思)+"Grid 独立路径备份门"(外部风险触发激活备份方案)两层表达。

### §2.2 Trigger 2 — ≥2 客户要求 Grid 独立买

**Verdict**: unknown (待 user 补)

**Confidence**: low  <!-- per Fix #2: 业务事实 audit 不可推断 (D-E-02), executor 无 evidence base -->
**Corroboration**: N-A  <!-- per Fix #2: 待 user 补真实业务事实, corroboration 无意义 -->
**Criticality**: blocks-decision  <!-- per Fix #2: 客户信号是否存在直接决定 §P5.2 是否激活 Leg B, 影响 §4 推荐 -->
**ADR-block**: yes  <!-- per Fix #2: unknown + blocks-decision → ADR-V2-024 Accepted 必须先 user 补真实数字 (即便答 0) -->

**Evidence**:
- ADR cross-ref: ADR-V2-023 §P5 第 2 条 (L157) verbatim "获得明确客户信号：≥2 个企业客户要求'不走 EAASP，直接买 Grid'"
- 业务事实 audit 不可推断 (per CONTEXT.md D-E-02 — 是否已有 ≥2 个企业客户表达"不走 EAASP 直接买 Grid"的明确意向, 只有 user 知道)
- User 陈述 (PRE-AUDIT-NOTES §C.2): 此条 "仍有效, 但前提是 Grid 已商品化到能直接卖" — 暗示当前 Grid 商品化度不足, trigger 即便满足也激活实力欠缺

**Cross-ref §F**: 此 trigger 在 §F 框架下对应 §F.Q4 (Grid vs EAASP 优先级真实工时分配) 的"客户信号是否扭转工时分配"那一面 — Q4 答 "Grid 优先" 时, ≥2 客户信号会加强;答"EAASP 优先" 时, ≥2 客户信号是 Q4 立场重估的强 trigger。

**Recommendation note**: 该 trigger 进入 §6 Open Items, Phase 4.2 启动 ADR-V2-024 Accepted 前必须 user 给真实数字(≥2 客户的具体名单 / 渠道 / 表达时点)。即便答数 0, 也要显式写 0, 不留空 — verdict=unknown 是诚实, verdict 一直空着是**未完成 audit**。

<!-- T1 stop point — §2.0 总表 §2.1 §2.2 已落盘, §2.3 §2.4 由 T2 接手, §3 / §4 / §5 / §6 / §7 由 T4 接手。
     T3 checkpoint(`/gsd-resume-work` 测试)在 T2 之后触发。 -->
