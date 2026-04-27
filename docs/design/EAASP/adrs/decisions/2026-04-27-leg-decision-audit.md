# Phase 4 Leg 决策审计 — 2026-04-27

Phase 4.1 audit, 系统化对照 ADR-V2-023 §P5 4 条 Leg B 激活触发条件 + Phase 4.1 baseline §F audit agenda Q1-Q4 4 个具体决策门, 产出每条 audit point 的 yes/no/partial/unknown verdict + evidence, 喂 Phase 4.2 ADR-V2-024 Decision 段。

**输入**:
- `.planning/phases/4.1-PRE-AUDIT-NOTES.md` §A-§F (baseline + audit agenda)
- `docs/external-review/2026-04-26-eaasp-skill-spec-coverage-internal.md` (30 项 MANDATORY 4/7/13/6 项 🟢/🟡/🔴/⚫ 分布)
- `docs/design/EAASP/adrs/ADR-V2-023-grid-two-leg-product-strategy.md` §P5 (L155-162)

**输出**: 8 audit-point verdict + 三选一推荐 (Leg A 硬化 / Leg B 激活 / 两腿都推) + 框架修订建议附加段 + Open Items 待 user 补段。

---

## §0. Framework Validity Gate (NEW per cross-AI review Fix #1)

> 此段在 §1 Summary 回填之前落盘. §0 verdict 决定 §4 三选一是否标 conditional/provisional + §5 elevation level. 见 PATTERNS.md interfaces 段 elevation 表.

**Question**: ADR-V2-023 §P5 (Leg B 激活 4 触发条件) 作为 framing Phase 4 product scope 决策的框架, 是否仍然有效?

**Verdict**: partial-needs-revision

**Confidence**: medium
**Corroboration**: yes
**Criticality**: blocks-decision
**ADR-block**: no

**Evidence** (per D-E-01 三档证据):
- ADR cross-ref: ADR-V2-023 §P5 (L155-162) verbatim 4 trigger — Leg A 集成 / Leg B 激活 二元产品形态切角仍然 dominant 在当前 codebase 与 ADR-V2-023 字面框架中
- User 陈述 (PRE-AUDIT-NOTES §C.1): user 真实心智模型是 "engine vs data/integration" 双轴 而非 Leg A/B 二元 — verbatim "User 实际心智模型用 'engine vs data/integration' 切职责: engine 层(全栈基础组件): 你专心做 …… data + integration 层: 他人主要做"
- User 陈述 (PRE-AUDIT-NOTES §C.1 末): "这与 ADR-V2-023 字面 'Leg A active / Leg B dormant' 措辞**正交**(不是反向, 是切角不同 —— 一个按'产品形态'切, 一个按'engine vs data/integration'切)"
- 代码/文件级 evidence: CONTEXT.md D-B-05 锁 "audit 不否决 §P5 4 条原文", 但 D-C-02 显式允许 §5 框架修订建议作为附加观察 — planner 已为框架 partial 弱化 reserve 了 §5 出口
- §P5 verdict 分布 (audit §2): 1 partial 语义弱化 (§P5.1) + 1 unknown (§P5.2) + 1 no (§P5.3) + 1 partial baseline 默认成立 (§P5.4) — 4 条中 2 条 partial 覆盖率 50%, 框架 trigger 措辞与现状部分不符
- §F verdict 分布 (audit §3): 2 partial (§F.Q1 vertical 并行倾向 / §F.Q2 governance 引擎部分) + 2 yes (§F.Q3 接入位 / §F.Q4 Grid 优先 + 并行) — 双轴模型 (engine vs data/integration) 正好把这 4 个 verdict 自然归约, 而 Leg A/B 二元只能勉强 framing

**Reasoning**:
ADR-V2-023 §P5 框架仍然是 Phase 4 决策的合法 framing 起点 — Leg A vs Leg B 是产品形态切角的真实 axis, 不能完全作废, ADR-V2-023 字面 §决策结尾 "腿 A 与腿 B 可以并存" 仍然成立。但 PRE-AUDIT-NOTES §C.1 user 心智模型与 ADR-V2-023 字面 framing **正交** (不是反向, 是切角不同) — user 实际工时切分按 "engine vs data/integration" 走, 而 §P5 4 条 trigger 是从 "Leg B 激活" 这个 dormant-then-activate 视角写的, 现状下 (EAASP 同仓孵化 + Grid 全栈也是 user 主战场) 这 4 条 trigger 措辞前提 "腿 A 已稳定, 腿 B dormant 待激活" 已经与现状不符 (Grid 已经 active 而非 dormant)。CONTEXT.md D-B-05 + D-C-02 已为这一 partial-revision 情形 reserve 了 §5 框架修订建议出口 — §0 verdict 选 partial-needs-revision 即承认 §P5 4 条 trigger 仍可作为外部风险评估的 watch list (按 D-B-03 措辞 "作为外部风险评估已弱化, 但作为自评内部节奏指标可保留"), 同时承认 user 心智模型双轴是更结构化的 framing, §5 应当与 §4 三选一 co-equal 输出, Phase 4.2 ADR-V2-024 必须读 §5 substance 而非仅引用 §4。本 audit §4 三选一推荐仍是 primary, 但不能脱离 §5 单独被 Phase 4.2 引用 — co-equal status 是 Fix #1 elevation logic 在 partial-needs-revision verdict 下的明确要求。

**Elevation 决定 (per Fix #1 elevation table)**:
- 若 §0 = still-valid: §4 三选一 primary; §5 附加观察 (D-C-04 honored)
- 若 §0 = partial-needs-revision: §4 三选一 primary, 但 §5 must be read alongside (co-equal)  ← **本 audit 落实**
- 若 §0 = obsolete-needs-replacement: §5 双轴模型 PRIMARY; §4 三选一 demoted to provisional/conditional fallback

**本 audit 的 §0 verdict 落实**: `partial-needs-revision` — §P5 4 条 trigger 中 2 条 partial (§P5.1 语义弱化 + §P5.4 baseline 默认成立) 覆盖率 50%, 框架措辞与现状部分不符, 但 Leg A/B 仍是 ADR-V2-023 + 当前 codebase 中 dominant framing。§4 三选一推荐保留 primary 地位, §5 双轴模型与 §4 co-equal — Phase 4.2 ADR-V2-024 §Decision 段必须同时引用 §4 推荐 phrase + §5 双轴模型 substance, 不能仅取其一。

---

## §1. Summary

| 维度 | 计数 |
|------|------|
| §P5 trigger yes | 0 |
| §P5 trigger no | 1 (§P5.3) |
| §P5 trigger partial | 2 (§P5.1, §P5.4) |
| §P5 trigger unknown | 1 (§P5.2) |
| §F Q yes | 2 (§F.Q3, §F.Q4) |
| §F Q no | 0 |
| §F Q partial | 2 (§F.Q1, §F.Q2) |
| §F Q unknown | 0 |

三选一推荐结论: **两腿都推进** (详见 §4); 框架修订建议: 双轴模型 (详见 §5; 与 §4 co-equal status per §0 partial-needs-revision elevation)

§0 Framework Validity Gate verdict: **partial-needs-revision** — §P5 4 条 trigger 中 2 条 partial 覆盖率 50%, 框架措辞与现状部分不符; §5 双轴模型与 §4 co-equal output 喂 Phase 4.2 ADR-V2-024 §Decision。

**Evidence schema 字段统计** (per cross-AI review Fix #2):
- 9 个 audit point (§0 + §2.1-§2.4 + §3.1-§3.4) 各自含 Confidence + Corroboration + Criticality + ADR-block 4 个扩展字段
- ADR-block:yes 计数 = 1 (§P5.2 unknown blocks-decision)
- blocks-decision criticality 计数 = 4 (§0 + §P5.2 + §F.Q1 + §F.Q4)
- 覆盖度: §6 hard-stop preamble 引用 ADR-block:yes 项, §6.1 优先级矩阵显式 P0/P1/P2 区分 unknown vs partial Open Items 阻塞力度
- Cross-ref total ≥ 8 (per D-B-02 平行交叉 §P5 4 条 + §F 4 条 = 8)

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

---

## §3. §F Audit Agenda Q1-Q4 审计

> §F 来源: `.planning/phases/4.1-PRE-AUDIT-NOTES.md` §F (commit `e1f27df`) — 调度 skill 规范 v0.1 评估浮现的 4 个 audit agenda 必答题。**§F 是 audit agenda(问题), 不是 backlog(动作)** — 回答这 4 个 Q 不预设要做哪些事, 但 audit 必须显式给立场, 否则 13 项 🔴 / 7 项 🟡 永远卡在"该谁做"悬置态。
>
> **§3 与 §2 的平行交叉关系 (per D-B-02)**: §2 §P5 4 条 trigger 是 ADR-V2-023 写入的"激活 Leg B"条件, §3 §F 4 条 Q 是从 EAASP 平台承接评估浮现的"engine vs data/integration 切分具体化"问题。两者**不是平行清单, 而是同一组事实在两种 framing 下的镜像**: §P5.1 (EAASP 节奏) ↔ §F.Q1 (vertical 决策) 同源; §P5.2 (≥2 客户) ↔ §F.Q4 (Grid 优先级) 同源; §P5.4 (团队富余) ↔ §F.Q4 工时分配 直接对应。本 §3 每个 Q 末尾的 Cross-ref §P5 段显式记录此对应关系, 便于 Phase 4.2 ADR-V2-024 Decision 段在两个 framing 间自由切换引用。

### §3.0 Verdict 总表

| Q | 题目摘要 | Verdict | Evidence 类型 | Cross-ref §P5 |
|---|---------|---------|--------------|--------------|
| §F.Q1 | EAASP 是否做调度行业 vertical | partial (并行倾向) | user 陈述 + 规范评估 | §P5.1 (节奏指标) + §P5.3 (vertical 决策连带) |
| §F.Q2 | Governance workflow 是否做 | partial (做基础引擎部分) | user 陈述 + 规范评估 §8 | §P5.2 (差异化能力) |
| §F.Q3 | 6 项 ⚫ 是否留接入位 | yes | 代码/文件级(已支持 ~80%) | (无直接对应) |
| §F.Q4 | Grid vs EAASP 优先级 | yes (Grid 优先 + 并行可行) | user 陈述(PRE-AUDIT-NOTES §B.2) | §P5.2 + §P5.4 |

### §3.1 Q1 — EAASP 是否做调度行业 vertical

**Verdict**: partial (并行倾向 — 部分 vertical 字段 engine 直接吸收, 部分留扩展点)

**Confidence**: medium
**Corroboration**: yes
**Criticality**: blocks-decision
**ADR-block**: no

**Evidence**:
- 规范评估 (`docs/external-review/2026-04-26-eaasp-skill-spec-coverage-internal.md` §"13 项 🔴"): 13 项 🔴 中 ~8 项(M1 机理 / M3 闭锁 / M7 dispatch_level / M8 latency_class / M9 灵活字段 / 6.6 多 skill 优先级 / Gotchas 段语义 / 风险等级)归属取决于 Q1 答案
- User 陈述 (PRE-AUDIT-NOTES §F.Q1): 三选项("是" / "否" / "两者并行")各自代价已列, "两者并行: 哪些字段 / hook MUST 在 engine, 哪些在 vertical? **这是真正难的事**"
- User 陈述 (PRE-AUDIT-NOTES §C.1): "engine vs data/integration" 切分模型与 vertical 决策同源 — vertical 是"他人接手 data/integration"的具体形式之一(此处"他人"是行业厂商而非泛指数据接入方)
- 工时事实 (PRE-AUDIT-NOTES §B.2): user 自陈 EAASP 引擎层是 user 工时(L2/L3/L4 引擎组件), data/integration 是他人 — 暗示 vertical 字段的 engine 部分由 user 做, vertical 业务规则由厂商写

**Cross-ref §P5**: 此 Q 在 §P5 框架下与 §P5.1 (EAASP 节奏指标) 同源 — Q1 答"是"(EAASP 直接吸收 vertical) 时, §P5.1 自评节奏门压力增大(EAASP engine 工时倍增风险);Q1 答"否"(扩展点) 时, §P5.1 节奏门压力减小但 §P5.3 (行业标准) 警觉度提升。同时与 §P5.3 (行业标准 / vertical 决策连带) 间接关联。

**Reasoning**:
"是" 与 "否" 二元在 audit 视角下都不是稳态最优 — "是" 让 EAASP engine 工时倍增风险 (M1 机理 / M3 闭锁等 8 项 vertical 语义直接吸收会让 L2/L3/L4 引擎组件在调度领域承担行业知识负担, 未来要做金融 / 医疗 vertical 时同样的扩展点要再 abstract 一次, 与 §B.2 user 工时倾向 Grid 主战场冲突), "否" 又让 EAASP 在调度市场失去差异化竞争力 (规范 §8 governance 论述显示纯通用引擎在企业市场卖不动)。"并行" 是 §F.Q1 三选项中 user 自陈 "真正难的事", 但也是与 §C.1 双轴模型最 self-consistent 的选项 — engine 吸收 4-5 项核心 vertical 字段 (dispatch_level / risk_class / latency_class 等行业通用 metadata, 这些字段在金融 / 医疗 vertical 也大概率出现, 复用性高), 业务规则 (机理表达 / 闭锁逻辑 / 6.6 多 skill 优先级) 留给厂商写, 即把 "engine vs data/integration" 切分映射到 "vertical metadata vs vertical business rules"。partial verdict 是 audit 现状的诚实描述 (具体字段切分待 Phase 4.2 子任务级 backfill), 不是回避答案。

**Recommendation note**: 当前 verdict partial(并行倾向)。Phase 4.2 ADR-V2-024 Decision 段如选"Grid 全栈 + EAASP 引擎层" + 三选一选择 Leg-A 或两腿都推时, 建议 Q1 落"并行" — 即 engine 吸收 4-5 项核心 vertical 字段(dispatch_level / risk_class / latency_class)做扩展点, 留 ~3-4 项业务规则给厂商写。具体哪些字段在 engine 哪些在 vertical 由 Phase 4.2 子任务列。

### §3.2 Q2 — Governance workflow 是否做

**Verdict**: partial (做基础引擎部分, governance 业务规则数据由企业接入)

**Confidence**: medium
**Corroboration**: yes
**Criticality**: influences
**ADR-block**: no

**Evidence**:
- ADR cross-ref: 当前 EAASP 已含 `tools/eaasp-l3-governance/` (policy DSL + risk classification 引擎)
- 规范评估 (`docs/external-review/2026-04-26-eaasp-skill-spec-coverage-internal.md` §"governance"): R1-R9 lint / HITL workflow / lifecycle 5 阶段 / 反措 → skill 升级流水 / 多维分类索引 — 5 项不绑定调度行业, 是任何企业 agent 平台都需要的
- User 陈述 (PRE-AUDIT-NOTES §F.Q2): "答'做': EAASP 从'通用 agent 平台基础引擎'升级为'企业级 agent governance 平台'。这是产品定位升级, 会拉高研发成本 + 拉长 time-to-market" / "答'不做': 没有 governance 的 agent 平台在企业市场卖不动(规范 §8 论述)"
- User 陈述 (PRE-AUDIT-NOTES §B.2): L3 governance 引擎是 user 工时主战场之一(基础组件), 企业 policy 数据 / 第三方治理系统集成是他人 — 此切分自然指向 partial(基础引擎做, 业务数据/集成不做)

**Cross-ref §P5**: 此 Q 在 §P5 框架下对应 §P5.2 (≥2 客户 Grid-only 信号) 的"差异化能力"那一面 — Q2 答"做基础引擎" 时, EAASP 在企业市场的差异化竞争力锁定 governance engine, 削弱"客户绕过 EAASP 直接买 Grid"的动机, 间接弱化 §P5.2 的 trigger 紧迫度。

**Reasoning**:
governance workflow 的 partial verdict 与 §C.1 双轴模型 self-consistent — R1-R9 lint engine / HITL workflow state machine / lifecycle 5 阶段 contract / multi-dim index 引擎 是**通用 engine** 层 (与调度无关, 任何企业 agent 平台都需要), 自然落 user 工时主战场; 而 policy 数据 / 业务规则 / 第三方治理系统适配 (e.g. 企业 SSO / 工单系统 / 审批流) 是**场景特定 data/integration**, 由客户 / 厂商接入。partial verdict 不是 "做一半" 的妥协, 而是 "做该做的部分, 不做不该做的部分" 的精准切分。"做" vs "不做" 二选一在 audit 视角下都偏极端 — "做" 让 EAASP 升级为完整企业级 governance 平台, 拉高研发成本 + 拉长 time-to-market (PRE-AUDIT-NOTES §F.Q2 user 自陈); "不做" 让 EAASP 在企业市场失去 governance 差异化, 卖不动。partial 是与 §B.2 user 工时事实 + §C.1 双轴模型 + 规范评估 §8 论述三向 self-consistent 的回答。Criticality=influences (而非 blocks-decision) 因为此 Q 影响 §P5.2 间接弱化但不直接 block ADR Accepted。

**Recommendation note**: 当前 verdict partial。建议措辞: governance workflow 的 **engine 层**(R1-R9 lint engine / HITL workflow state machine / lifecycle 5 阶段 contract / multi-dim index 引擎)由 user 做, **policy 数据 / 业务规则 / 第三方治理系统适配**由企业 / 厂商接入 — 与 §C.1 "engine vs data/integration" 切分一致。Phase 4.2 ADR-V2-024 Consequences 段引用此 verdict。

### §3.3 Q3 — 6 项 ⚫ 是否留接入位

**Verdict**: yes

**Confidence**: high
**Corroboration**: yes
**Criticality**: informational
**ADR-block**: no

**Evidence**:
- 代码/文件级 evidence: `tools/eaasp-mcp-orchestrator` 已支持任意第三方 MCP server 装载(per MEMORY.md "EAASP v2.0 Phase 2.5" 系列 commits)
- 代码/文件级 evidence: anchors schema 在 ADR-V2-006 §2.3 已含 evidence_anchor_id 字段层面对位外部签名扩展(国密签名 / WORM 存储等)
- 代码/文件级 evidence: `OpenAICompatProvider` (`lang/nanobot-runtime-python` + 工具支持的 LLM 兼容接口) 已让信创 LLM 可接入(Phase 2.5 W2.T2 落地)
- User 陈述 (PRE-AUDIT-NOTES §F.Q3): "**预期答案 = 是**, 因为 EAASP 已经做了 ~80%, 文档补一下即可。**audit 顺手就能输出**, 不是阻塞项"
- 业务事实推断豁免: "EAASP 是否要为 ⚫ 6 项留接入位"是产品意愿决策, 已由 user 在 §F.Q3 直接陈述 "yes 预期"

**Cross-ref §P5**: 此 Q 与 §P5 4 条 trigger 无强直接对应(§P5 关注 Leg B 激活, Q3 关注 EAASP engine 接入面是否完备)。但**间接关系**: Q3 答 yes 减少"客户因接入受阻而要求 Grid-only"的概率, 弱化 §P5.2 trigger 触发面之一。

**Reasoning**:
Q3 是 PRE-AUDIT-NOTES §F.2 第 4 点显式标记的 "audit 顺手便宜事" — 不是"是否要做接入位"的问题 (这是产品意愿决策, 已由 user 表态 yes), 而是 "EAASP 现状是否已经支持" 的事实核对。代码事实显示 ~80% 已支持: mcp-orchestrator 任意第三方 MCP server 装载 (Phase 2.5 W2.T2 落地)、anchors schema evidence_anchor_id 字段为外部签名 (国密 / WORM) 留位、OpenAICompatProvider 让信创 LLM 通过 OpenAI-compatible 接口接入 — 三处代码事实集体证明接入面已开。剩余 ~20% 主要是文档化补全 (把 "已经能装" 显式文档化为 EAASP extension surface contract), 不是新工程量。Criticality=informational 因为此 Q yes verdict 是稳态结论, 不影响 §4 三选一推荐 + 不进 Open Items + 不阻塞 ADR Accepted。Phase 4.3 / 后续 milestone 可单独立 ADR 把 "⚫ 6 项接入位" 显式文档化为客户/厂商集成参考, 但本 audit 不要求 Phase 4.2 处理。

**Recommendation note**: 当前 verdict yes(已支持 ~80%, 文档补一下即可)。Phase 4.2 ADR-V2-024 不需要为此 Q 单独添加 Decision 子段;但建议 Phase 4.3 / 后续 milestone 把"⚫ 6 项接入位"显式文档化为 EAASP extension surface contract(单独 ADR 候选), 作为客户/厂商集成参考。

### §3.4 Q4 — Grid vs EAASP 优先级

**Verdict**: yes (Grid 优先 + 并行可行)

**Confidence**: high
**Corroboration**: yes
**Criticality**: blocks-decision
**ADR-block**: no

**Evidence**:
- User 陈述 (PRE-AUDIT-NOTES §B.2): "重心在 Grid — user 自己工时主要投 Grid" 直接指向"Grid 优先"
- User 陈述 (PRE-AUDIT-NOTES §F.Q4): "答'Grid 优先': 先做 Grid 5-7 项, EAASP 决策做完再启动" + "答'并行': Grid 5-7 项跟 EAASP 决策正交, 可以同时推。**大概率是真实路径**"
- User 陈述 (PRE-AUDIT-NOTES §F.Q4 + §B.3): "Grid 侧的 5-7 项(M5 溯源链 + M9 model_lock + M6 evidence chain 字段)**不是规范驱动, 是 Grid 自身企业级品质必需**" — 暗示 Grid 优先级独立于 EAASP 决策
- 业务事实(精确工时切分百分比)属 D-E-02 不可推断范围, 由 §6 Open Items 待 user 补

**Cross-ref §P5**: 此 Q 在 §P5 框架下对应 **§P5.2** (≥2 客户 Grid-only 信号 触发激活备份) — Q4 答"Grid 优先 + 并行" 时, Grid 自身产品化路径独立推进, ≥2 客户信号即便满足也不再是仅有的激活动机(Grid 已 active 而非 dormant); **§P5.4** (团队富余 ≥30%) — Q4 直接回答 §P5.4 的工时分配前提, "Grid 优先" 即定义上 §P5.4 ≥30% 工时早已超阈。

**Reasoning**:
此 Q verdict yes (Grid 优先 + 并行) 是本 audit 中 evidence 最强的回答之一 — PRE-AUDIT-NOTES §B.2 + §F.Q4 + §B.3 三处 user 直接陈述同向, 加上 §C.1 双轴模型推论同向, 共 4 处 corroborating evidence。"Grid 优先" 意味 user 工时主要投 Grid (cli/server/desktop/platform/web*), "并行" 意味 EAASP 引擎层 (L2/L3/L4 基础组件) 与 Grid 工作正交可同时推 — 这与 §P5 4 条 trigger 框架的 "Leg A active / Leg B dormant" 措辞**直接矛盾** (Grid 在 user 心智中从未 dormant), 是 §0 verdict partial-needs-revision 的最强 evidence 之一。Criticality=blocks-decision 因为 Phase 4.2 grid-cli/server/desktop/platform/web* 五者发力顺序直接由此 verdict 决定 — Q4 答 "Grid 优先 + 并行" 即定义上 EAASP 不 dormant + Grid 不 dormant + 两者并行, Phase 4.2 三选一只能落 "两腿都推进" (audit §4.1 推荐的 evidence chain 直接来源)。Open Item: 五者中先发力哪 2 个? PRE-AUDIT-NOTES §E.3 列了此问题, 由 user 在 Phase 4.2 启动前补。

**Recommendation note**: 当前 verdict yes(Grid 优先 + 并行可行)。Phase 4.2 ADR-V2-024 Decision 段三选一应当落"两腿都推进 + 框架修订(Q4 倾向)"或"Leg A 硬化但 Grid 引擎品质独立投入"两类 — 不是简单"Leg A vs Leg B"二选一(本 Q4 verdict 即是"两腿都推, 但内部按 engine vs data/integration 切")的具体化。

---

## §4. 三选一推荐 (D-C-01)

> ROADMAP §Phase 4.1 SC#2 要求 audit doc 给出 "走 Leg A 硬化 / 走 Leg B 激活 / 两腿都推进" 三选一推荐, 措辞明确不含糊(per CONTEXT.md D-C-01)。本 §4 严格满足 SC#2, 框架修订建议另放 §5(per D-C-04 SC#2 严格满足三选一即可)。
>
> **Elevation 状态 (per Fix #1)**: 此 §4 推荐的 status 取决于 §0 Framework Validity Gate verdict:
> - 若 §0 = `still-valid`: 此 §4 推荐 = primary recommendation
> - 若 §0 = `partial-needs-revision`: 此 §4 推荐 = primary, 但 must be read alongside §5 (co-equal) ← **本 audit 落实 (§0 verdict = partial-needs-revision)**
> - 若 §0 = `obsolete-needs-replacement`: 此 §4 推荐 = conditional / provisional fallback; §5 双轴模型 = PRIMARY output

### §4.1 推荐: **两腿都推进**

理由(executor 据 §0 + §2 + §3 verdict 综合产出, 非抄 PLAN):

1. **§3.4 Q4 verdict yes (Grid 优先 + 并行可行) 直接否决"集中精力做 EAASP, Grid dormant"** — Grid 5-7 项企业级品质必需 (M5 溯源链 / M9 model_lock / M6 evidence chain) 独立于规范驱动, 已经在推进; PRE-AUDIT-NOTES §F.Q4 user 直接陈述"并行: 大概率是真实路径"。Grid 不是 Leg B dormant 待激活, Grid 已经 active。
2. **§2.4 §P5.4 verdict partial (baseline 默认成立) + §B.2 工时事实 同时支持 EAASP 引擎层投入** — user 工时主战场 包含 EAASP L2/L3/L4 引擎组件, 不是只投 Grid; §3.1 §F.Q1 partial (并行倾向) + §3.2 §F.Q2 partial (做基础引擎部分) 共同要求 EAASP engine 持续扩展 — 不是 "走 Leg B 激活" 之后让 EAASP 萎缩。
3. **§0 verdict partial-needs-revision + §2.1 §P5.1 partial 语义弱化** 表示 ADR-V2-023 字面 "Leg A active / Leg B dormant" 措辞与现状不符 (Grid 已 active), 但 ADR-V2-023 字面 §决策结尾 "腿 A 与腿 B 可以并存" 仍然成立 — 即 §P5 4 条 trigger 即便 partial, 也不否决"两腿同时推"作为产品形态选项。三选一中 "两腿都推进" 既不需要 §P5 trigger 全部 fully met (partial 即可), 也不需要 supersede ADR-V2-023 (D-F-05 决策权在 Phase 4.2)。
4. **§3.4 Q4 verdict 显式陈述 "两腿都推, 但内部按 engine vs data/integration 切"** — 此 §4 三选一推荐与 §5 双轴模型 co-equal output 不冲突: §4 答"两腿都推" 是产品形态切角的回答, §5 双轴模型是职责切角的回答, 两者正交。Phase 4.2 ADR-V2-024 §Decision 段两者必须同时引用。

### §4.2 推荐措辞(Phase 4.2 ADR-V2-024 Decision 段建议 verbatim)

> "Phase 4 product scope 决定 = **两腿都推进**. 据 audit §3.4 Q4 verdict yes (Grid 优先 + 并行可行) + §2.4 §P5.4 partial baseline 默认成立 + §B.2 user 工时事实, Grid 全栈与 EAASP 引擎层均为 user 工时主战场, 不存在二选一前提 (Grid 已 active, EAASP 同仓孵化期间 user 自做引擎); Implication: Phase 4.2+ 子任务必须按 §5 双轴模型 (engine vs data/integration) 切, 而非按 Leg A/B 切, 见 §5 + Open Items §6 待 user 补 5 条精确工时分配。" (per audit §4.2; §0 elevation status: partial-needs-revision — 需与 §5 双轴模型 co-equal 引用)

### §4.3 否决的两个候选

- **走 Leg A 硬化**: §3.4 Q4 yes (Grid 优先) + PRE-AUDIT-NOTES §B.2 + §B.3 (若被迫二选一选 Grid) 直接否决"集中精力做 EAASP, Grid dormant" — 与 baseline §D Sanity Guard 第 1 条冲突, audit 无硬证据推翻 baseline。Grid 5-7 项企业级品质必需独立于 EAASP 决策, 不能 dormant。
- **走 Leg B 激活 (Grid 独立 + EAASP dormant)**: 与 §B.2 user 自陈 EAASP 引擎层是 user 工时主战场之一相悖, audit 无硬证据推翻 baseline §D 第 4-5 条 Sanity Guard。EAASP engine (L2/L3/L4 基础组件) 由 user 自做是 baseline 锁定, 不能 "EAASP 引擎层交给他人做"。

### §4.4 推荐与 §0 Framework Validity Gate 的 self-consistency 检查

**§0 verdict = partial-needs-revision** elevation 含义: §4 三选一保留 primary 地位, 但**必须与 §5 双轴模型 co-equal 引用**。本 §4.1 推荐 "两腿都推进" 在 ADR-V2-023 字面框架下是合法选项 (§决策结尾 "腿 A 与腿 B 可以并存"), 在 §5 双轴模型下也清晰落地 ("user 投 engine + 他人投 data/integration, Grid 全栈作为产品 leg 自然 active 但内部职责切清晰") — 双框架同向支持本推荐, self-consistent。Phase 4.2 ADR-V2-024 §Decision 段必须**同时**引用 §4.2 推荐措辞 (产品形态切角) 与 §5.5 双轴模型 substance (职责切角), 不能仅取其一; 仅引用 §4 会丢失 partial-needs-revision elevation 要求的 §5 substance, 仅引用 §5 会丢失 SC#2 三选一硬要求。本审计的 evidence chain 在两种 framing 下都收敛到同一答案是 evidence triangulation 的副产物, 不是 framing 巧合 — Q4 Grid 优先 + 并行 + Q1/Q2 EAASP engine partial 同时支持 + §B.2 工时事实多源 corroborate。

---

## §5. 框架修订建议 (D-C-02)

> §5 elevation 状态 (per cross-AI review Fix #1) 由 §0 Framework Validity Gate verdict 决定:
> - 若 §0 = `still-valid`: §5 = **附加观察** (D-C-04 honored)
> - 若 §0 = `partial-needs-revision`: §5 = **co-equal with §4** (Phase 4.2 ADR-V2-024 必须读 §5 substance, 不能仅引用 §4) ← **本 audit 落实**
> - 若 §0 = `obsolete-needs-replacement`: §5 = **PRIMARY output** (§4 demoted to provisional)
>
> §5 与 §4 在本 audit 落实下 co-equal — 即 ADR-V2-023 字面框架 partial 保留, §4 三选一推荐"两腿都推"与 §5 双轴模型必须同时被 Phase 4.2 ADR-V2-024 §Decision 引用, 二者正交不冲突。

### §5.1 双轴模型提案

替代 ADR-V2-023 的 Leg A(EAASP 集成)/ Leg B(Grid 独立)二元切换, 提议两轴:

**轴 1: engine vs data/integration**
- engine = 可复用的核心组件, user 主战场(per PRE-AUDIT-NOTES §B.2 + §C.1)
- data/integration = 场景特定的横切关注, 客户/厂商/他人接手

**轴 2: Grid 全栈 vs EAASP 引擎层 vs 数据/集成横切**
- Grid 全栈 = `grid-cli` / `grid-server` / `grid-desktop` / `grid-platform` / `web*` 全部 — 都是 user 工时主战场
- EAASP 引擎层 = `tools/eaasp-l2-memory-engine/` + `eaasp-skill-registry/` + `eaasp-mcp-orchestrator/` + `eaasp-l3-governance/` + `eaasp-l4-orchestration/` 各自 engine 组件 — user 主战场之一
- 数据/集成横切 = 客户语料 / vector 库 / 企业 policy 数据 / SSO / 第三方治理 / 工作流 / SaaS 集成 / signature backend / WORM 存储 / 信创 LLM 适配 — 他人接手

### §5.2 与 ADR-V2-023 Leg A/B 的关系

双轴模型不**否决** ADR-V2-023 的精神(per CONTEXT.md D-B-05 audit 不否决 §P5 4 条原文, 但允许部分弱化措辞 + 提议重新框定):
- ADR-V2-023 Leg A(EAASP 集成) ≈ 双轴中 "Grid 全栈作为 EAASP L1" + "EAASP 引擎层 user 自做" 组合
- ADR-V2-023 Leg B(Grid 独立) ≈ 双轴中 "Grid 全栈 + 数据/集成横切由他人接入" 组合 — Grid 独立产品本质是"engine 完备 + data/integration 接入面定义清晰"

**关键差异**: ADR-V2-023 Leg A/B 是"产品形态切" — 切的是"打包成 EAASP L1 还是打包成独立产品"; 双轴是"职责切" — 切的是"哪些工时归 user(engine), 哪些归他人(data/integration)"。两者**正交**, 不是反向: 同一仓库可以按双轴划分职责, 同时打包成 Leg A 和 Leg B 两种产品形态(per ADR-V2-023 §决策 §决策结尾 "腿 A 与腿 B 可以并存")。

### §5.3 双轴模型的优势

1. **解释力增强**: §F.Q1-Q4 的所有 verdict 都自然归约为双轴中的一个面 — 不需要硬塞进 Leg A/B 二元框架
2. **工时分配明确**: user 工时投 engine(全部上游引擎组件), 他人工时投 data/integration(全部下游横切场景), 不再纠结"Grid 是不是该 dormant"
3. **§P5 4 条 trigger 自然重新框定**:
   - §P5.1(EAASP 节奏)→ engine 节奏自评(自家工时投入是否充分)
   - §P5.2(客户 Grid-only)→ data/integration 接入面是否完备(客户绕过 EAASP 是接入痛点信号)
   - §P5.3(行业标准)→ engine 跨厂商可移植性 watch list
   - §P5.4(团队富余)→ engine 工时充裕度自评
4. **Phase 4.2+ 子任务拆分清晰**: engine 子任务列(per 引擎组件)+ data/integration 接入面子任务列(per 接入场景), 不再纠结"哪个 leg 优先"

### §5.4 双轴模型的风险与代价

1. **改 ADR 成本**: ADR-V2-024 落 Proposed 时若直接采纳双轴, ADR-V2-023 实质上被 supersede(D-F-05 决策推到 Phase 4.2 — Phase 4.1 不决)
2. **CLAUDE.md / WORK_LOG / DEFERRED_LEDGER 的"Leg A / Leg B" 措辞需要 sweep**: 类似 Phase 4.0 CLEANUP-01 chunk_type sweep 的工程量, 评估 ≤1-2 hour
3. **未来沟通对外措辞**: 双轴模型对外解释门槛高于"Leg A vs Leg B 二元", 需要团队 / 客户文档同步更新
4. **决策权在 Phase 4.2 而非本 audit** — 本 §5 仅提议, 不强制采纳

### §5.5 audit 推荐: Phase 4.2 ADR-V2-024 Decision 段考虑采纳双轴

理由: §3 4 个 Q 的 verdict 在双轴下都自然归约且解释力强;§4 三选一推荐"两腿都推进"在 Leg A/B 框架下含糊("两腿都推进"是什么意思?), 在双轴下清晰("user 投 engine + 他人投 data/integration, Grid 全栈作为产品 leg 自然 active 但内部职责切清晰")。

但 audit 不强制 — Phase 4.2 决定是否采纳, audit 仅 surface 双轴模型作为 ADR-V2-024 §Alternatives Considered 的 Option D(per PATTERNS.md ADR-V2-024 body shape)。本 audit §0 verdict partial-needs-revision 下 §5 与 §4 co-equal, 即 Phase 4.2 ADR-V2-024 §Decision 段必须同时引用 §4.2 推荐措辞 + §5.5 双轴模型 substance, 不能仅引用 §4 而忽略 §5。

### §5.6 双轴模型与 §P5 4 条 trigger 的兼容性 case 检查

为证明 §5 双轴模型不**否决** ADR-V2-023 §P5 (per D-B-05), 列出每条 §P5 trigger 在双轴下的 self-consistent 重新框定:

- **§P5.1 (EAASP 项目延期/停滞/定价不合理)** → 在双轴下重新框定为 "engine 节奏自评门" — engine 工时投入是否充分维持节奏, partial 语义弱化 (因 EAASP 已是 user 自做 engine 而非外部上游) 在双轴下消解
- **§P5.2 (≥2 客户 Grid-only 信号)** → 在双轴下重新框定为 "data/integration 接入面完备性信号" — 客户绕过 EAASP 直接买 Grid 是 data/integration 接入痛点的逆向信号 (而非 Leg B 激活的二元开关)
- **§P5.3 (行业 agent runtime 标准出现)** → 在双轴下重新框定为 "engine 跨厂商可移植性 watch list" — 行业标准影响 engine 接口稳定性, 与 Grid 是否独立产品无关
- **§P5.4 (团队富余 ≥30%)** → 在双轴下重新框定为 "engine 工时充裕度自评" — 摆脱"释放给腿 B"的从属表述, 改为 engine 内部资源充裕性判定

四条 trigger 在双轴下都自然落点, 无一被否决 — §5 是 framing 升级, 不是 supersede。

---

## §6. Open Items — 待 user 补

> 下列 audit point verdict 标 unknown 或 partial, 需 user 在 Phase 4.2 启动 ADR-V2-024 Accepted 前补真实业务事实(per CONTEXT.md D-E-02 audit 不试图自己推断业务事实 + D-E-04 Open Items 段)。
>
> **Hard-stop rule (per cross-AI review Fix #2)**: 任何 Open Item 在 §0 / §2.X / §3.X 子段中标 `**ADR-block**: yes` 且 `**Verdict**: unknown` 时, **ADR-V2-024 不能从 Proposed → Accepted, 直到该 Open Item 被 user 补齐**。本 audit 已识别的 ADR-block:yes 项: §P5.2 (≥2 客户 Grid-only 信号 unknown). Phase 4.2 启动前必须补真实业务事实(即便答数 0, 也要显式落 0)。

> **设计意图**: §6 Open Items 是 audit 与 user 业务事实之间的 hand-off 清单。audit 不试图自己推断业务事实 (per D-E-02), 凡是涉及"客户信号"、"工时百分比"、"五者发力顺序" 这类只有 user 能答的问题, audit 显式标 unknown 或 partial 并落入 §6, 不在 audit 层面"猜"。partial verdict 的 Open Items (§P5.4 / §F.Q1 / §F.Q4) 表示 audit 已给方向 (baseline 默认成立 / 并行倾向 / Grid 优先), 但精确数值或字段切分待 user 补; unknown verdict 的 Open Items (仅 §P5.2) 表示 audit 完全无 evidence base, user 必须给 ground truth。这一区分通过 ADR-block 字段 (yes/no) 落实为是否阻塞 ADR-V2-024 Accepted。

- **§P5.2 (≥2 客户 Grid-only 信号)** — Verdict 当前 unknown; **待 user 补**: 是否已经收到 ≥2 个企业客户表达"不走 EAASP 直接买 Grid"的明确意向? 数字与渠道。即便答数 0, 也要显式写 0, 不留空。**ADR-block: yes** — Phase 4.2 ADR-V2-024 Accepted 前 hard-stop 必须补。
- **§P5.4 (团队工时分配精确百分比)** — Verdict 当前 partial(baseline 默认成立); **待 user 补**: 当前 user 自己工时切分 Grid vs EAASP 引擎层 vs 数据/集成各占百分比? 是否符合 PRE-AUDIT-NOTES §B.2 "Grid 是工时主战场"? 推荐格式: "Grid 全栈 X% / EAASP 引擎层 Y% / 元工作 Z%"。
- **§F.Q1 (vertical engine 字段 vs 厂商扩展点的精确边界)** — Verdict 当前 partial(并行倾向); **待 user 补**: 哪 4-5 项 vertical 字段 MUST 在 EAASP engine?(候选: dispatch_level / risk_class / latency_class / 哪两条)哪 ~3-4 项业务规则留厂商写? 此问题 Phase 4.2 ADR-V2-024 Decision 段需具体到字段级。
- **§F.Q4 (Grid 产品化优先排序)** — Verdict 当前 yes(Grid 优先 + 并行可行); **待 user 补**: `grid-cli` / `grid-server` / `grid-desktop` / `grid-platform` / `web*` 五者中先发力哪 2 个? PRE-AUDIT-NOTES §E.3 列了此问题, audit 不替 user 决, 但 Phase 4.2 PLAN.md 拆 task 时必须先回答。
- **(框架修订决策)** — Phase 4.2 决定是否采纳 §5 双轴模型(per D-F-05 supersede ADR-V2-023 决策权在 Phase 4.2)。**待 user 补**: 是否同意 ADR-V2-024 §Decision 引用双轴模型? 若同意, ADR-V2-024 是否同时 supersede ADR-V2-023?

### §6.1 Open Items 优先级与 ADR-block 矩阵

| Open Item | Priority | ADR-block | 阻塞 ADR-V2-024 Accepted? | 推荐 user 回答时机 |
|----------|----------|-----------|--------------------------|------------------|
| §P5.2 ≥2 客户 Grid-only 信号 | P0 | **yes** | **是** (Hard-stop per Fix #2) | Phase 4.2 启动前 |
| §F.Q1 vertical 字段精确边界 | P1 | no | 否 (但 Phase 4.2 子任务级需要) | Phase 4.2 PLAN.md 拆 task 前 |
| §F.Q4 grid-cli/server/desktop/platform/web* 五者发力顺序 | P1 | no | 否 (但 Phase 4.2 子任务级需要) | Phase 4.2 PLAN.md 拆 task 前 |
| §P5.4 工时分配精确百分比 | P2 | no | 否 (baseline 默认成立可作 placeholder) | Phase 4.2 ADR Accepted 后 backfill 也可 |
| 框架修订决策 (是否采纳双轴) | P0 | no | 否 (但 Phase 4.2 ADR Decision 段需要) | Phase 4.2 ADR-V2-024 Decision 段填实时 |

**优先级说明**: P0 = 必须在 Phase 4.2 启动前补; P1 = 必须在 Phase 4.2 PLAN.md 拆 task 前补; P2 = 可 backfill。本 audit 5 条 Open Items 中仅 §P5.2 是 ADR-block:yes 硬阻塞, 其余 4 条不阻塞 ADR Accepted 但影响 Phase 4.2 子任务粒度 — 这是 partial 与 unknown 在 Open Items 阻塞力度上的 schema 区分 (per Fix #2 ADR-block 字段)。

---

## §7. Audit Metadata

- Audit date: 2026-04-27
- Auditor: GSD-managed Phase 4.1 (gsd-execute-phase + superpowers two-stage 在 REVIEW_POLICY §2 high-risk triggers fire 时激活, audit doc T1/T2/T4 命中 §2.9 LOC > 200 + §2 战略级 design 改动 → 实证激活)
- Inputs scanned:
  - `.planning/phases/4.1-PRE-AUDIT-NOTES.md` (14.6KB §A-§F)
  - `docs/external-review/2026-04-26-eaasp-skill-spec-coverage-internal.md` (19.9KB, §F.Q1 / §F.Q2 evidence 主源)
  - `docs/design/EAASP/adrs/ADR-V2-023-grid-two-leg-product-strategy.md` (15.1KB, §P5 L155-162 verbatim)
- Audit scope (NOT-in-scope):
  - 不**否决** ADR-V2-023 字面 (D-B-05);允许"部分弱化"措辞 + 提议双轴模型替代框架(§5)
  - 不修改 ADR-V2-023 原文 (canonical_refs §"不要修改的 SSOT 例外")
  - 不动 DEFERRED_LEDGER (canonical_refs §"不要修改的 SSOT 例外")
  - supersede ADR-V2-023 决策推到 Phase 4.2 (D-F-05)
  - ADR-V2-024 §Enforcement 段填写推到 Phase 4.2 (D-F-03)
- Soft length cap: ~600 LOC (D-A-04 + Fix #1 + Fix #2 fields ~750 LOC margin); 当前实际 LOC: 见 `wc -l` 输出
- 三选一推荐(per ROADMAP §Phase 4.1 SC#2): **两腿都推进**(详见 §4)
- 框架修订建议(D-C-02 附加观察, 本 audit §0 verdict partial-needs-revision 下与 §4 co-equal): 双轴模型(engine vs data/integration)+ (Grid 全栈 vs EAASP 引擎层 vs 数据/集成横切)— 提议 Phase 4.2 ADR-V2-024 §Decision 采纳, 由 user 决定

### §7.1 Cross-AI Review 五 Fix 落实证据

| Fix # | 描述 | 落实位置 | 验证方法 |
|-------|------|---------|---------|
| Fix #1 | §0 Framework Validity Gate + elevation logic | §0 全段 + §4 §4.4 + §5 elevation 注 | grep "## §0. Framework Validity Gate" + Verdict 三态 regex |
| Fix #2 | Per-audit-point evidence schema extension (Confidence + Corroboration + Criticality + ADR-block) + §6 hard-stop preamble | 9 audit point 各自含 4 字段 + §6 preamble + §6.1 优先级矩阵 | `grep -c '\*\*Confidence\*\*' ≥ 9` + `grep -F 'Hard-stop rule'` |
| Fix #3 | T5 ADR Decision 段 runtime substitution (no PLAN-level hardcoded recommendation) | T5 task body (PLAN) + ADR §Decision 段 (Phase 4.2 走 substitution) | T5 acceptance criterion 6: ADR Decision verbatim 含 audit §4.1 phrase |
| Fix #4 | T6 GSD adoption notes demoted template (≥3 Scenario lines if claiming "no friction") | T6 task body (PLAN) + WORK_LOG.md GSD Adoption Notes 段 | T6 acceptance criterion 9: Friction:/No friction found: + ≥3 Scenario: |
| Fix #5 | T3 alternate trigger point selection (Path A default per D-D-04, Path B post-audit-pre-ADR with override reasoning) | T3 task body (PLAN) + 04.1-OBSERVATIONS-WIP.md Trigger point selected: A | T3 SKIPPED per user 决定 GOVERNANCE-03 deferred 至下一 phase 独立场景 |
