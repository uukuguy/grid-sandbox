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
| §P5.3 | 行业 agent runtime 标准出现 | no | 代码/文件级 (proto v2 仍 EAASP 内部) | §F.Q1 (vertical 决策连带) |
| §P5.4 | 团队富余 ≥30% | partial (baseline 默认成立) | user 陈述 (PRE-AUDIT-NOTES §C.2 第 3 点) | §F.Q4 |

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

### §2.3 Trigger 3 — 行业 agent runtime 标准出现

**Verdict**: no

**Confidence**: high  <!-- per Fix #2: 公开可观察事实 (proto 状态 + 行业 RFC 进度), 高可信 -->
**Corroboration**: yes  <!-- per Fix #2: proto v2 文件 evidence + 公开 RFC 状态双源 -->
**Criticality**: informational  <!-- per Fix #2: no verdict 是稳态, 不进 Open Items, 不影响 §4 推荐 -->
**ADR-block**: no  <!-- per Fix #2: no verdict 不是 unknown, 不阻塞 -->

**Evidence**:
- ADR cross-ref: ADR-V2-023 §P5 第 3 条 (L158) verbatim "行业标准演化：出现广泛采用的 agent runtime 标准（非 EAASP），Grid Platform 可以作为该标准的参考实现"
- 代码/文件级 evidence: `proto/eaasp/runtime/v2/runtime.proto` 仍是 EAASP 内部 16 method gRPC 契约 — 业内尚无广泛采用的等效协议(Anthropic ACP, OpenAI Assistants API 等都未达"广泛采用 agent runtime 标准"门槛, 各家自定义 protocol 互不兼容)
- 业务事实推断豁免(per D-E-02 边界): "行业是否存在标准" 是公开可观察事实, 不属于"audit 不可推断的业务事实", 当前结论 = 否(2026-04 行业仍处早期 RFC 阶段, MCP 是工具协议非 runtime 协议, A2A 等仍 RFC)
- User 陈述未直接覆盖此条, PRE-AUDIT-NOTES §C.2 也未提及, 默认沿用 ADR-V2-023 字面假设(无标准则不激活)

**Cross-ref §F**: 此 trigger 在 §F 框架下与 §F.Q1 (EAASP 是否做调度行业 vertical) 部分对应 — Q1 答"否"(EAASP 提供扩展点交厂商写 vertical) 时, 行业标准与 Grid Platform 参考实现的语义增强;Q1 答"是"时, EAASP 直接吸收 vertical 字段, 行业标准对 Grid 形态影响弱化。

**Recommendation note**: 当前 verdict no 是稳态结论, **不需要进入 Open Items**(D-E-04 Open Items 仅收 unknown verdict)。Phase 4.2 ADR-V2-024 重新框定时此 trigger 可保留原措辞, 但建议添加"行业标准 watch list"注脚(MCP 演化 / A2A RFC 状态 / Anthropic ACP 走向 — phase 末或 milestone 末扫一次)避免长期 dormant 失明。

### §2.4 Trigger 4 — 团队富余 ≥30%

**Verdict**: partial (baseline 默认成立)

**Confidence**: medium  <!-- per Fix #2: user 陈述 baseline 默认成立有, 精确百分比无 -->
**Corroboration**: yes  <!-- per Fix #2: PRE-AUDIT-NOTES §B.2 + §C.2 第 3 点双源一致 (Grid 是工时主战场) -->
**Criticality**: influences  <!-- per Fix #2: 精确百分比影响 Phase 4.2 工时分配 task 拆分粒度 -->
**ADR-block**: no  <!-- per Fix #2: partial verdict + baseline 默认成立, ADR Accepted 可推进; 精确百分比 Open Item 是 Phase 4.2 优化项不阻塞 -->

**Evidence**:
- ADR cross-ref: ADR-V2-023 §P5 第 4 条 (L159) verbatim "团队能力富余：核心腿 A 已经稳定到可以释放出 ≥30% 工程投入给腿 B"
- User 陈述 (PRE-AUDIT-NOTES §B.2): "重心在 Grid — user 自己工时主要投 Grid" + L43 "Grid 是工时主战场"
- User 陈述 (PRE-AUDIT-NOTES §C.2 第 3 点): 此 trigger "在 user 心智中**默认成立**(Grid 一直就是工时重心), 不需要等"
- 但精确百分比(Grid vs EAASP 实际工时切分)属业务事实(per D-E-02), audit 不可推断 — 进入 §6 Open Items 由 user 在 Phase 4.2 启动 ADR-V2-024 Accepted 前补真实工时切分百分比

**Cross-ref §F**: 此 trigger 在 §F 框架下直接对应 §F.Q4 (Grid vs EAASP 优先级真实工时分配) — Q4 答"Grid 优先"或"并行" 时, 此 trigger 的 ≥30% 阈值在 baseline 已超(user 自陈 Grid 是主战场即 ≥50% 工时); Q4 答"EAASP 优先" 时, 此 trigger 的"团队能力富余"语义需重考(若 EAASP 优先则定义上 Grid 工时不富余反成限制)。

**Recommendation note**: 此 trigger 与 baseline §B.2 切分表 + §C.1 "Grid 全栈是工时主战场" 同源, 当前 verdict partial 表示"baseline 默认成立 + 精确百分比待补"。Phase 4.2 ADR-V2-024 重新框定时建议把此条措辞从"团队能力富余 ≥30%"改写为"Grid 自评工时占比 ≥某阈值且 baseline 主战场地位稳定" — 摆脱"释放给腿 B"的从属表述, 改为 Grid 自身可独立产品化的资源充裕性判定。
