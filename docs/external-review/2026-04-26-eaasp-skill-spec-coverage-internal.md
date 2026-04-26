# EAASP 平台对《电力调度 AI 智能体 Skill 规范 v0.1》承接评估（内部）

**日期**: 2026-04-26
**状态**: 内部分析,**不对外发**
**配套文件**: `docs/external-review/2026-04-26-skill-spec-v0.1-standard.md`(规范本体)+ `docs/external-review/2026-04-26-skill-spec-reading-guide.md`(导读)
**目的**: 把 v0.1 规范的 MANDATORY 清单(M1-M10 / R1-R9 / 4 创新点)逐条对照本仓库 EAASP 现有架构,识别**已支持** / **需平台改动** / **不在 EAASP 范围**(由 skill 编写者 / 上层应用承接)

> **重要边界**: 本文档**不**写"平台改动 backlog 草稿"。识别出的 gap 仅作为 Phase 4.1 audit 的输入素材,不预设 phase 拆分 / ADR 编号 / 排期。Phase 4.1 audit 的结论才是决策。

---

## §1. 摘要 — 整体覆盖度评级

EAASP v2.0 当前是**研究版**(Phase 0-3.6),`tools/eaasp-*/` 7 个组件覆盖了 L2/L3/L4 的**通用 agent 平台基础引擎**,但**电力调度行业语义层完全空白**。

| 维度 | EAASP v2.0 现状 | 规范 v0.1 要求 | 评级 |
|------|----------------|----------------|------|
| L0 wire 协议 | proto v2 16-method gRPC,稳态 | MCP / Anthropic Skills(规范 §3) | 🟢 **基本支持**(EAASP gRPC 是 L1 内部协议,与 MCP 互补;skill 包格式对齐 anthropic skills 已有 SkillStore) |
| L1 runtime 适配 | 7 个 runtime 通过 contract v1.1 | 跨厂商 skill 包格式 + L1.5 业务接口适配 | 🟡 **部分支持**(L1 已 substitutable;L1.5 完全空白) |
| L2 内存 / skill 包 | l2-memory-engine + skill-registry + mcp-orchestrator | progressive disclosure / Gotchas / Steps / References / Boundaries | 🟡 **基础引擎已有**(SkillStore 解析 v2 frontmatter 含 scoped_hooks),但**调度行业字段、Gotchas 段语义、5 阶段 lifecycle 全部缺位** |
| L3 治理 | l3-governance(policy DSL + ManagedHook + audit) | M1 机理胜出 / M3 闭锁优先 / M5 溯源链 / M6 国密签名 | 🔴 **方向对但语义不对位**(policy DSL 是通用 deny/allow,**没有"机理引擎 verdict 单调胜出""闭锁系统外部权威""国密 SM2 签名"** 这类调度专属义) |
| L4 编排 | l4-orchestration(SessionOrchestrator + L1Client + 4-card dispatch) | HITL / 双签 / 调度层级 / 极端 case 回放 | 🔴 **session lifecycle 引擎已有,但调度专属字段 + workflow 全空白** |
| 数据质量 / 时间戳 | 量测元数据 ingestion 不存在 | M4 数据质量与时间戳一致性 | ⚫ **根本不在范围**(属于 L1.5 SCADA-MCP server,需电网厂商提供) |
| 国密 / 等保 / 信创 | 当前用 jsonwebtoken / SHA-256 / AES | M6 + §6.5 信创合规 | ⚫ **根本不在范围**(属于客户侧 / 集成层,EAASP 引擎不锁加密栈) |
| 4 创新点 | 原子-SOP / Gotchas / L1.5 / 库治理 | 全部 MUST | 🔴 **全部缺位**(parser 没有 skill_type 字段;SKILL.md 段不强制 Gotchas;L1.5 是新概念;lifecycle 4→5 阶段需扩展) |

**整体结论**:

1. **EAASP 的 substitutable L1 / 通用 agent 引擎能力 ≠ 调度行业 skill 平台**。规范 v0.1 是 L2 调度行业约束层,**EAASP 当前就是 L1 通用层 + L2 通用基础**,**两者并不直接对位**。
2. EAASP 已有的"通用基础"对规范的支撑是**间接的**(skill registry / policy DSL / L4 orchestration 是底座),但所有**调度专属语义**(M1-M10 内涵 / Gotchas / 国密 / L1.5)都需要在 EAASP 之上**额外建一层**才能承接规范。
3. 这一层的归属**不是预设决策**:既可能是 EAASP 平台扩展(向调度行业垂直化),也可能是基于 EAASP 的**第三方调度行业 vertical**(由南瑞 / 国电南自等厂商在 EAASP 通用引擎上做行业化)。Phase 4.1 audit 应当把这个边界明确化。

---

## §2. 逐条 gap 表

### §2.1 §3 L1 通用层 — 业界标准引用(MUST)

| 标准 | EAASP 现状 | 评级 |
|------|-----------|------|
| MCP 2025-11-25 | mcp-orchestrator(rmcp 1)+ l2-memory-engine 暴露 6 MCP tools | 🟢 已支持 |
| Anthropic Agent Skills + agentskills.io 2025-12-18 | skill-registry parse v2 frontmatter(name/description/version/allowed-tools/scoped_hooks/workflow.required_tools)+ progressive disclosure 通过 SkillStore 写盘 | 🟢 已支持 |
| JSON Schema draft 2020-12 | mcp_tools.py 用 JSON Schema 描述每个 tool 入参 | 🟢 已支持 |
| OpenTelemetry GenAI + W3C Trace Context | event_handlers.py 有 EventHandler Protocol + DefaultIngestor + FTS5Indexer(自定义事件,**非 OTel**) | 🟡 间接支持(EAASP 自有事件流,**未对接 OTel**) |
| SemVer | skill-registry 用字符串 `version` 字段,**未做 SemVer 解析** | 🟡 字段在,语义不强制 |
| JSON-RPC 2.0 | rmcp 内嵌 | 🟢 已支持 |
| OAuth 2.1 + DCR | grid-server 有 jsonwebtoken;EAASP tools 默认无 auth(信任内网)| 🟡 grid-server 有,EAASP tools 默认开放 |

**Gap 总结(L1 通用层)**:
- **OpenTelemetry GenAI conventions** 对接缺失 — EAASP 自有 event 模型与 OTel 语义不对齐(规范要求 OTel 是为了"调度可追溯性合规硬要求")
- **SemVer 强制语义**缺失 — 字段存在但 lifecycle 转换 / dependency 范围检查未做

### §2.2 §4 L2 调度行业 MANDATORY 清单(M1-M10)

| ID | 内容 | EAASP 现状 | 归属 | 评级 |
|----|------|-----------|------|------|
| **M1** 机理引擎单调胜出 | LLM vs 机理引擎冲突时机理胜出 + 入审计 | EAASP 完全没有"机理引擎"概念。L3 policy DSL 是**通用 deny/allow**,不知道什么是"潮流计算 verdict" | 平台改动(L3 引入"verdict 来源优先级表";"机理引擎"接入是 L1.5 厂商职责)| 🔴 缺位 |
| **M2** 输入侧 + 输出侧两道闸 | 校核分两道,输出 MUST 调机理引擎,不可 LLM 自评 | EAASP 有 PreToolUse / PostToolUse hook(scoped_hooks),**机制对** —— 调度专属语义需要在 hook 内编排 | 平台改动小(hook 机制已有,语义靠 skill 写)+ skill 编写者 | 🟡 机制 OK,语义靠 skill |
| **M3** 闭锁系统作为外部权威(仅 A 类)| AI skill 仅闭锁 client,不可 issuer | EAASP 没有"闭锁系统"概念,但 L3 policy DSL 可表达"deny if 来源 != 闭锁判决" | L1.5 闭锁-MCP server(电网厂商职责)+ L3 配 deny rule | 🔴 调度专属,EAASP 通用层无此概念 |
| **M4** 数据质量 + 时间戳一致性 | A/B 类 skill 声明依赖量测点 + staleness 检测 + INPUT_QUALITY_FAILED 错误 | EAASP 完全无量测元数据 ingestion | 不在 EAASP 范围(L1.5 SCADA-MCP 必须提供 quality_flag / timestamp / staleness;由 skill 编写者在 SKILL.md 声明依赖量测点)| ⚫ 根本不在范围 |
| **M5** 决策溯源链 | evidence chain: input_snapshot_id + tool_call_sequence + mechanism_check_results + llm_generation_params | l2-memory-engine `anchors` 表已有 event_id / source_system / tool_version / model_version / rule_version / snapshot_hash —— **schema 维度已基本对位** | 🟢 已支持(基础)+ 平台小改动(`mechanism_check_results` / `llm_generation_params` 字段对齐) | 🟢 大半已支持 |
| **M6** 法定签名 + WORM 审计(仅 A 类)| `(agent_signature, model_version, prompt_hash, evidence_chain_hash, utc_timestamp)` 五元组 + 国密 SM2 + WORM 存储 | l3-governance audit.py 有 audit_log;l2 anchors 是 append-only;**完全没有签名 / WORM 存储 / 国密** | 不在 EAASP 范围(国密 + WORM 是客户侧/信创集成层,EAASP 引擎不锁加密栈)+ 平台小改动(签名字段挂入 anchor schema)| ⚫ 大半不在 EAASP 范围 |
| **M7** 跨调度层级权限边界 | `dispatch_level` + `controllable_assets_filter`,越权拒绝 | EAASP **没有 dispatch_level 概念**;L3 policy DSL 有 deny rule 但**字段不对**;skill-registry frontmatter parser 没有 dispatch_level 字段 | 平台改动(skill_parser.rs + L3 policy DSL 加 dispatch_level 字段)+ skill 编写者声明 + L1.5 网络层加密 | 🔴 缺位 |
| **M8** LLM 延迟硬上限 + 秒级闭环禁区 | 声明 latency_class,平台校核挂载位置 | EAASP 没有 latency_class 概念。**最关键的"AGC / 紧急控制 / 稳控不能跑 LLM"是产品架构决策,不是 EAASP 配置项** | 平台改动(skill_parser 加 latency_class)+ 不在 EAASP 范围(秒级闭环根本不通过 EAASP 部署)| 🔴 部分在 EAASP,部分根本不该在 |
| **M9** 模型版本锁定 + 漂移管控 | `model_lock: {provider, model_id, version, fingerprint}` + 每次调用校验 + 升级触发 skill 重评 | EAASP l1_client.py 没有 fingerprint 校验;l2 anchors 有 model_version 但**没有 fingerprint** | 平台改动(L4 dispatch 调用前校 fingerprint;skill_parser 加 model_lock;skill-registry lifecycle 触发 model 升级 → 重评流水)| 🔴 缺位 |
| **M10** 投运前历史极端工况回放 | 国调中心维护极端 case 库;A/B 类 skill 投运前回放通过 | EAASP 有 eaasp-certifier(L1 contract 认证),**机制对**,但是给 L1 contract 的不是给 skill 的;skill 级"极端 case 回放"完全空白 | 平台改动(certifier 扩展支持 skill-level case suite,或新增 skill-eval 引擎)+ 国调中心维护 case 库(不在 EAASP 范围)| 🔴 缺位但底座可复用 |

**Gap 热度**:
- **🟢 大半已支持**: M5 决策溯源链(anchors schema 已对位 ~70%)
- **🟡 机制 OK 语义靠 skill**: M2 校核闸(PreToolUse / PostToolUse hook 机制对位)
- **🔴 EAASP 缺核心概念**: M1 / M3 / M7 / M9 / M10(都需要 EAASP 引入"机理引擎" / "闭锁系统" / "dispatch_level" / "model_lock" / "case suite" 等调度专属抽象)
- **⚫ 根本不在 EAASP 范围**: M4(L1.5 SCADA-MCP 责任) / M6(信创集成层 / 客户侧)

### §2.3 §6 外部边界

| 边界 | EAASP 现状 | 评级 |
|------|-----------|------|
| 6.1 工具边界(三档 strict / degrade / advisory)| L3 hook mode 有 deny / allow / warn,**类比成立但不严格映射**;skill_parser 没有 `x-grid-tool-error-policy` 字段 | 🟡 机制类似,字段缺 |
| 6.2 人(HITL)边界 | EAASP **没有 HITL 概念**;L3 policy hook 模式可以做"need_approval" deny,但**没有"双签"/"事后复核"workflow 引擎** | 🔴 缺位(HITL workflow 引擎是平台空白) |
| 6.3 知识边界(规程 / RAG)| l2-memory-engine 是**通用 RAG**(FTS + HNSW + time-decay),**机制对**,但"规程引用必带 (regulation_id, version, clause_id, library_snapshot_hash)"四元组**字段对不上** | 🟡 引擎对位,schema 需扩展 |
| 6.4 **L1.5 业务接口适配层** | EAASP 有 mcp-orchestrator 管 MCP server 生命周期,**机制是对的**(orchestrator 可以装载任意 MCP server),但 D5000-MCP / EMS-MCP / SCADA-MCP / 闭锁-MCP / PMS-MCP / AGC-MCP **全部不存在,也不在 EAASP 写**(规范明确"由原厂商提供")| ⚫ EAASP 提供"装载位"(mcp-orchestrator),实现由厂商承担 |
| 6.5 信创合规(国密 / 等保 / 信创全栈)| EAASP 不锁加密栈,LLM provider 通过 OpenAICompatProvider 已支持国产模型(智谱 / 通义 等)走 OpenRouter 或自托管 | ⚫ EAASP 不限制,客户侧实现 |
| 6.6 多 skill 编排冲突 | EAASP **没有"skill 优先级" / "互斥矩阵"概念**;l4-orchestration session 内一次只跑一个 skill | 🔴 缺位 |

### §2.4 §7 反模式 lint(R1-R9)

| ID | 反模式 | EAASP 现状 | 评级 |
|----|------|-----------|------|
| R1-R9 全部 | LLM 自己写 SQL / 模拟潮流 / 自评 / import 别 skill / 编造规程 等 | **EAASP 完全没有 lint** —— skill-registry 的 SkillParser 只做 frontmatter / scoped_hooks 解析,不做反模式检测 | 🔴 缺位(平台需要新增 skill quality gate 引擎) |
| 额外质量门 | description 触发准确率 / prompt 注入扫描 / Gotchas ≥ 3 / A 类反例 / 规程引用一致性 | 全部空白 | 🔴 缺位 |

### §2.5 §8 技能库治理

| 项 | EAASP 现状 | 评级 |
|----|-----------|------|
| 8.1 Lifecycle 5 阶段 + governance hook | skill-registry SkillStatus enum **只有 4 阶段**: Draft / Tested / Reviewed / Production。规范要求 6 阶段: draft / review / shadow / canary / production / retired | 🔴 阶段名错位 + 数量少 2(shadow / canary / retired 完全缺) |
| 8.1 退出 KPI / 退回触发 | 完全没有 governance hook 引擎 | 🔴 缺位 |
| 8.2 多维分类索引(业务域 / 调度层级 / 风险等级 / skill 类型)| skill-registry 只支持 tag + status + scope 检索,**没有业务域 / 调度层级 / 风险等级 / skill 类型字段** | 🔴 缺位 |
| 8.3 跨厂商互通 + vendor 命名空间 | 无 `vendor.<name>.*` 命名空间约束 | 🟡 schema 约束缺,但语义可加 |
| 8.4 SemVer 版本治理 + 模型联动 | version 字段是字符串无解析;model_lock 缺(M9 联动)| 🔴 缺位 |
| **8.5 知识资产沉淀(反措 → skill 升级流水)**| EAASP 没有反措 / 安监通报 ingestion;l2 memory 可以**容纳**反措通报作为 evidence,但**没有"反措通报触发 Gotchas 段更新"流水** | 🔴 缺位(平台缺 governance workflow 引擎)|
| 8.6 集中 + 本地化(国调 / 省调 / 地调 三层)| skill-registry git_backend 模块有 git-based 分发(理论可支持上下游 fork),但脱敏机制缺 | 🟡 git 分发底座对,脱敏机制缺 |
| 8.7 Skill registry 模式(签名 + DAG + 审计)| 签名缺(回到 M6);依赖图缺;调用 trace 缺(回到 M5 OTel 对齐)| 🔴 缺位 |

### §2.6 4 大创新点(规范明示)

| 创新点 | EAASP 现状 | 评级 |
|--------|-----------|------|
| **原子 vs SOP skill 显式二分** | skill-registry parser **没有 `skill_type: atomic / sop / hybrid` 字段** | 🔴 缺位(parser 改动小,但下游 lint / lifecycle 依赖)|
| **Gotchas 段(反措沉淀)** | SKILL.md 段约束**只在 SkillStore `parse_skill_md` 拆头/正文**,不强制段名;Gotchas 概念完全空白 | 🔴 缺位(SkillStore 需要 SKILL.md 段语义解析)|
| **L1.5 业务接口适配层** | mcp-orchestrator 提供"装载位",但**没有 L1.5 概念命名 / contract test framework / CIM 语义中介** | 🟡 装载位对,概念缺位 |
| **技能库治理(企业 AI 资产 + lifecycle + 反措 → skill 升级流水)**| 见 §2.5 lifecycle / governance hook / 反措流水 全部缺位 | 🔴 缺位 |

---

## §3. 与 ADR-V2-023 / Phase 4.1 baseline 的耦合

### §3.1 这次 gap 分析对 Phase 4.1 audit 的输入

Phase 4.1 audit 的核心问题是 **"engine vs data/integration 切分"**(`.planning/phases/4.1-PRE-AUDIT-NOTES.md` §B.2 / §C.1)。本评估**正交印证**这个切分:

| Gap 类型 | 数量(M1-M10 + R1-R9 + 4 创新点 + §6 + §8 全集 ~30 项)| 归属 |
|---------|-------------|------|
| **🟢 已支持 / 大半已支持** | 4 项(L1 通用层基本对位 + M5 溯源 + l2-memory RAG 引擎 + git_backend 分发底座)| EAASP engine ✅ |
| **🟡 机制对位但 schema / 字段需扩展** | 7 项(M2 hook / 6.1 三档 / 6.3 RAG / 6.6 多 skill / SemVer / 8.6 脱敏 / OTel)| **EAASP engine 增量改动**(中等) |
| **🔴 EAASP 缺核心概念** | 13 项(M1 / M3 / M7 / M8 / M9 / M10 + R1-R9 lint + HITL + lifecycle 5 阶段 + 多维索引 + 反措流水 + 4 创新点中的 3 个)| **要么 EAASP engine 调度行业化扩展,要么基于 EAASP 的调度行业 vertical** —— **这是 Phase 4.1 audit 的真问题** |
| **⚫ 根本不在 EAASP 范围** | 6 项(M4 SCADA / M6 国密 + WORM / L1.5 厂商 MCP / 6.5 信创全栈 / M10 case 库内容 / 8.7 国调 registry)| **data / integration 层** —— 印证 Phase 4.1 baseline §B.2 的"他人主要做"这块 |

### §3.2 三个观察(给 Phase 4.1 audit 的素材,不是结论)

**观察 1**: 🟢/🟡 类(11 项)是 **EAASP engine 自然演进**的范围,不依赖"是否要做调度行业 vertical"这个决策 —— 这些是通用 agent 平台向**任何垂直行业**演进时都会遇到的 schema 完备性问题(SemVer / OTel / 字段约束 / 段语义)。

**观察 2**: 🔴 类(13 项)**集中在两个方向**:
- **调度行业语义引入**(M1 / M3 / M7 / M8 / M9 / M10 + 6.6 + 4 创新点)— 必须有人决定"这层语义放 EAASP engine 内置,还是放 EAASP 之上的 vertical 层"
- **平台 governance workflow 引擎**(R1-R9 lint / HITL / lifecycle 5 阶段 / 反措流水)— 可以是 EAASP engine 通用扩展,**不必绑定调度行业**

**观察 3**: ⚫ 类(6 项)**完全在 ADR-V2-023 / Phase 4.1 baseline 中已经预留的"data / integration / 客户侧"**:
- L1.5 SCADA / EMS / D5000 等 MCP — 厂商职责(规范 §6.4 已显式说明)
- 国密 / WORM — 信创集成层(EAASP 不锁加密栈)
- 极端 case 库内容 — 国调中心维护
- registry 集中 / 分布部署 — 政策决策

**这三类划分跟 Phase 4.1 baseline 的 "engine vs data/integration" 切分是一致的,不是冲突的。**

### §3.3 与 ADR-V2-023 §P5 触发条件的关系

ADR-V2-023 §P5 4 条 Leg B 激活触发条件,在本评估中显现的"调度行业 vertical 决策"是**比 Leg A/B 更细一层的决策**:

- 如果决定"EAASP engine 直接做调度行业化扩展"(13 项 🔴 全在 EAASP 内做)→ EAASP 团队工时大幅倾向调度
- 如果决定"调度行业 vertical 由厂商在 EAASP 之上做"(13 项 🔴 给厂商做)→ EAASP engine 提供 hook / 扩展点,厂商写垂直层
- 如果决定"两者并行"(部分 EAASP 内置,部分 vertical)→ 需要明确**哪些 MUST 在 engine,哪些在 vertical**

ADR-V2-023 §P5 假设的是"是否激活 Grid 独立产品",**没有覆盖"EAASP engine 是否做调度行业化扩展"** —— 这一层决策 Phase 4.1 audit / Phase 4.2 ADR-V2-024 应当显式处理。

---

## §4. Open Questions(给后续 phase 用,不在本评估解答)

1. **EAASP engine 的"行业中立"定位与"调度行业语义引入"的张力** — 加入 dispatch_level / risk_class / latency_class 等字段会不会让 EAASP 不再行业中立? 还是只把字段做成可选 / 命名空间隔离的扩展点?
2. **L1.5 概念命名** — 规范 §6.4 用 "L1.5",EAASP 现有 4 层(L0-L4)架构里没有 1.5;是否要在 EAASP 文档里引入这层命名,还是 mcp-orchestrator 装载的 MCP server 默认就是 L1.5(无需新概念)?
3. **lifecycle 5 阶段 vs EAASP 4 阶段** — Draft → Tested → Reviewed → Production 对位 draft → review → ? → ? → production; shadow / canary / retired 是行业通用还是调度专属?
4. **R1-R9 lint 引擎归属** — Quality gate 是 EAASP engine 内置(类似 cargo clippy),还是上层应用配置的 hook?
5. **反措 → skill 升级流水** — 规范 §8.5 是调度行业核心,但"通报触发 Gotchas 段更新 + MAJOR 升级 + lifecycle 重走"作为 workflow 是否可抽象为通用"外部信号驱动 skill 演化"机制?
6. **多 skill 编排冲突 / 优先级 / 互斥矩阵** — l4-orchestration 当前是单 skill,这是 EAASP 通用扩展还是调度专属?
7. **eaasp-certifier 是否扩展为 skill-level case 回放** — 还是新增 skill-eval 引擎(类似 grid-eval)? 跟 grid-eval 的分工?
8. **HITL workflow 引擎** — l4-orchestration 当前没有 HITL / 双签 / 事后复核;这层是 EAASP 通用,还是调度专属?

> 这 8 条 Open Questions 是 §3.2 观察 2 的具体展开。建议 Phase 4.1 audit 优先消化 1 / 2 / 4 / 5(架构方向)再做 3 / 6 / 7 / 8(具体引擎设计)。

---

## §5. 用本评估时的注意事项

1. **本文档不是 backlog,不是排期,不是 ADR**。识别出的 13 项 🔴 gap **不是"必须做的事"**,只是"如果要承接规范 v0.1,EAASP 平台层需要补的能力清单"。是否要承接 / 承接到什么程度 / 由 EAASP 还是 vertical 承接 = Phase 4.1 audit 决定。
2. **规范 v0.1 是评审稿,不是定稿**。等领导反馈 + v0.2 之后,本评估需要重新过 —— v0.2 的 MANDATORY 集合可能调整。
3. **"已支持" / "缺位"**指的是**当前 EAASP v2.0 研究版**的状态,**不**指 Grid 整体能力。Grid `crates/grid-engine` 有更丰富的 hook / security / audit 机制,部分 gap 可能已经在 grid-engine 内有对位实现 —— 但本评估**只**对照 `tools/eaasp-*/`,因为规范是给 L2-L4 平台的,不是给 L1 runtime 的。
4. **本文档措辞规则**: 用 EAASP 内部术语(crate / table / class)+ 规范 ID(M1-M10 / R1-R9 / §X.Y),不重复规范本体的领域论述 — 那些归导读 / 标准版 / 完整版。
5. **不引用本评估给国调 / 厂商**。这是给团队内部 Phase 4.1 audit 的素材,带有"EAASP 当前是研究版""ADR-V2-023 决策悬置"等内部上下文,对外引用会造成误导。
