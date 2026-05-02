---
id: ADR-V2-025
title: "L1 Runtime 契约执行强度差异化策略（四档）"
type: strategy
status: Accepted
date: 2026-05-02
accepted_at: 2026-05-02
phase: "Phase 5.1 — Engine Hardening"
author: "Jiangwen Su"
supersedes: []
superseded_by: null
deprecated_at: null
deprecated_reason: null
enforcement:
  level: strategic
  trace:
    - "tests/contract/cases/test_chunk_type_contract.py"
    - ".github/workflows/phase3-contract.yml"
  review_checklist: "docs/design/EAASP/adrs/ADR-V2-025-l1-runtime-contract-tier-strategy.md"
affected_modules:
  - "crates/grid-runtime/"
  - "crates/eaasp-goose-runtime/"
  - "crates/eaasp-claw-code-runtime/"
  - "lang/claude-code-runtime-python/"
  - "lang/nanobot-runtime-python/"
  - "lang/pydantic-ai-runtime-python/"
  - "lang/ccb-runtime-ts/"
  - "lang/hermes-runtime-python/"
  - "tests/contract/"
  - ".github/workflows/phase3-contract.yml"
related:
  - ADR-V2-017
  - ADR-V2-021
  - ADR-V2-024
---

# ADR-V2-025 — L1 Runtime 契约执行强度差异化策略（四档）

**Status:** Accepted
**Date:** 2026-05-02
**Phase:** Phase 5.1 — Engine Hardening
**Author:** Jiangwen Su
**Related:** ADR-V2-017（L1 生态三轨：主力 / 样板 / 对比），ADR-V2-021（chunk_type 契约冻结），ADR-V2-024（Phase 4 双轴模型 — engine vs data/integration）

---

## Context / 背景

ADR-V2-017 把 L1 runtime 划成三轨（主力 grid-runtime + 样板 + 对比），ADR-V2-021 把 `chunk_type` enum 列入冻结契约，并在 Phase 3 通过 contract-v1.1 把 7 个 runtime（grid / claude-code / goose / nanobot / pydantic-ai / claw-code / ccb，hermes 已冻结）一次性拉齐到 42 PASS / 22 XFAIL。

但 Phase 3 的"7 runtime 平等通过 v1.1"是 baseline 投资 — 它没有回答**契约演进时**的问题：当 contract-v1.2 引入新行为（例如 hook event 扩展、新 chunk 类型、tool namespace 加严），是否每个 runtime 都必须立刻 PASS？

实际开发节奏（per Phase 4a + Phase 5.0 review）暴露三种张力：

1. **主力 vs 对比的投入差距**：grid-runtime + claude-code-runtime 是 user 工时主战场，每个 contract 改动会立刻被回归覆盖。但 nanobot / pydantic-ai / ccb / claw-code 的实装速度跟不上 — 强行要求"v1.2 同日 7/7 PASS"会把 contract 演进变成 7-runtime 同步代码动员，违背 ADR-V2-017 §对比"借鉴而非维护"的初衷。
2. **xfail 的语义模糊**：Phase 3 的 22 XFAIL 既包含"runtime 真不支持的功能"（D136 grid-runtime probe-turn hook miss）又包含"runtime 故意不实装的特性"（ccb 是 TS 反编译参考）。但 CI 把它们一视同仁标记为 xfail，无法表达"主力档 xfail = blocker"vs"参考档 xfail = expected"的差异。
3. **冻结档没有规范出口**：hermes 冻结 per ADR-V2-017，但 CI matrix 仍把它当成"可选 runtime"处理，每次 PR 触发 7 个 job 中包含一个永远不会被触碰的 hermes 路径，浪费 CI 时间也制造视觉噪音。

WATCH-05 / NEW-D2（Phase 4a project review 发现的 test parametrization gap — `test_chunk_type_contract.py` 当前只有 3 个 test，没有 7-runtime 参数化）放大了这三种张力 — 没有 ADR 定义"哪几个 runtime 必须 PASS"，参数化扩展时也无法明确每个分支的策略。

NEW-D2 的存在揭示了一个根本问题：**对比 runtime 是契约的活体测试**（ADR-V2-017 §对比理由），但**契约对各 runtime 的强度要求需要差异化**（实操理由）。两者必须明确分离。

---

## Decision / 决策

引入 **L1 Runtime 契约执行强度四档策略**，与 ADR-V2-017 三轨产品策略正交。Phase 5.3 contract-v1.2.0 起所有契约演进按本表 gating；hermes 由 ADR-V2-017 §冻结决议保留为档外的 frozen note。

### 四档定义

| Tier (执行强度) | Behavior | grid | claude-code | nanobot | pydantic-ai | goose | claw-code | ccb |
|---|---|---|---|---|---|---|---|---|
| **主力档 (Primary)** | MUST PASS; xfail = blocker | ✅ | ✅ | — | — | — | — | — |
| **样板档 (Sample)** | PASS-or-xfail; regressions must be investigated | — | — | ✅ | — | ✅ | — | — |
| **参考档 (Reference)** | v1.1 baseline; no new failures required | — | — | — | ✅ | — | ✅ | ✅ |
| **冻结档 (Frozen)** | skip; hermes frozen per ADR-V2-017 | — | — | — | — | — | — | — |

(hermes-runtime-python 单独占冻结档；CI 路径直接 skip — 见下表 ADR-V2-017 frozen note。)

| Frozen note | Disposition |
|---|---|
| **hermes-runtime-python** | ⏸️ skip (per ADR-V2-017 §冻结决策) — 不在主表参与 v1.1+ 任何契约审计 |

### 各 runtime 分档理由

**主力档 — MUST PASS**

- **grid-runtime** (Rust): 本团队"主力 + 唯一可替换 L1"的核心实现 (per ADR-V2-024 §1 双轴模型 engine 接入面 primary focus)。任何 contract-v1.x 改动都必须先在 grid 上 PASS — grid 是 substitutable L1 的 reference truth。
- **claude-code-runtime-python** (Python): Anthropic SDK baseline，是契约本身的"母语实现"参照 — 如果 chunk_type 含义在 Anthropic 一侧不一样，契约定义本身就有问题。Phase 4 已被纳入 grid-cli 优先发力组合 (ADR-V2-024 Open Item #3) 边界场景的 daily-driver runtime，每次契约改动会立即在两个 runtime 上跑通才合并。

**样板档 — PASS-or-xfail; regressions must be investigated**

- **nanobot-runtime-python** (Phase 3 certified): Phase 3 S3.T5 拿到 contract v1.1 cert (42 PASS / 22 XFAIL)。OpenAI-compat baseline，验证"轻量 Python L1 接入路径"。新契约要求"PASS or 显式 xfail with rationale"，xfail 增加要 reviewer 看根因。
- **goose-runtime** (Phase 2.5 W1 certified): Block goose 通过 ACP subprocess + stdio MCP proxy 接入。验证"Rust 子进程 wrap 第三方 agent runtime 接入路径"。新契约同样 PASS-or-explained-xfail。

**参考档 — v1.1 baseline; no new failures required**

- **pydantic-ai-runtime-python** (Phase 3 addition): pydantic-ai 框架的早期接入，无正式 cert。新契约不要求 PASS，但**v1.1 baseline 不能 regress** — i.e. 现有 PASS 不能掉回 xfail。
- **claw-code-runtime** (Phase 3 addition, emerging): UltraWorkers 模块化 Rust runtime，仍在演进期。Reference tier 给它呼吸空间。
- **ccb-runtime-ts** (TypeScript reference impl, internal): Bun + TS，仅供"TS 接入参考与反编译对照"，per ADR-V2-017 §对比 §3 "仅内部对比用，不商用"。新契约只盯 v1.1 baseline 不掉。

**冻结档 — skip**

- **hermes-runtime-python**: ADR-V2-017 §"hermes 冻结" 已说明：保留代码作历史样板，不再投入修复。本档为 hermes 提供规范出口 — CI 不再把它当成 active runtime 跑契约审计，明确节省 CI 时间且消除"xfail 等于 frozen"的语义混淆。

### CI 执行规则

`.github/workflows/phase3-contract.yml` matrix 按 tier 标 `xfail` 字段 + `continue-on-error` 控制 PR 阻塞性：

- 主力档 (`tier: primary`, `xfail: false`): 失败立即 block PR，无 `continue-on-error`。
- 样板档 (`tier: sample`, `xfail: true`): `continue-on-error: true`；reviewer 看 xfail 增量决定是否需要根因。
- 参考档 (`tier: reference`, `xfail: true`): `continue-on-error: true`；只对 v1.1 baseline regression 失败 block。
- 冻结档 (`tier: frozen`, `xfail: skip`): matrix 仍列出但 `if: matrix.runtime != 'hermes'` 守护跳过整个 build。

### Test 参数化要求

`tests/contract/cases/test_chunk_type_contract.py` 通过 `@pytest.mark.parametrize("runtime_name", [...7 runtimes...])` 把 live-runtime 测试展开为 7 个 parametrized case (NEW-D2 关闭见 Phase 5.1 T2)。两个 guard test (`test_whitelist_matches_adr` + `test_unspecified_is_zero`) 不参数化 — 它们是 proto schema invariant，与 runtime 无关。

`pytest --collect-only` 应展示 ≥7 个 parametrized item，CI 通过 `--runtime=<name>` 在每个 matrix job 里只执行匹配的那个 runtime 分支（其余 skip）。

---

## Consequences / 后果

### Positive

- **契约演进路径清晰**：Phase 5.3 contract-v1.2.0 起，一份 PR 不需要等 7 个 runtime 同步实装。主力档先合，样板档 follow-up，参考档自演进，冻结档免审。
- **xfail 语义分明**：xfail 的"权重"由 tier 而不是 runtime 决定 — 主力档 xfail 是 blocker，参考档 xfail 是 expected。reviewer 看 CI 输出可以直接判断严重性。
- **CI 时间节省**：hermes 跳过整段 build/setup，从 7 → 6 个真跑的 job。
- **NEW-D2 闭环**：test_chunk_type_contract.py 7-runtime 参数化 + tier-based gating 把 contract gate 真正落到每个 runtime 上 (从 3 → ≥21 collected items)。

### Negative

- **额外维护文档**：ADR-V2-025 + 每次 contract-v1.x 演进时 ADR-V2-025 表格的 tier 字段需要 review (是否 promote/demote)。
- **样板档 → 主力档晋升缺触发条件**：本 ADR 不规定 nanobot/goose 何时可以晋升到主力档 — 留给后续 Phase 5.3 / 5.5 评估 (ADR-V2-024 §1 双轴模型下双产品形态的策略评估)。
- **参考档"baseline 不掉"门槛仍需契约测试机制保障**：纯靠 reviewer 人眼看 PR diff 不可靠；需 CI 在 PR 报告中标出 v1.1 baseline regression。本 ADR 仅声明策略，具体 CI 报告增强留 Phase 5.5 INTERFACE 工作。

### Risks

- **Tier 漂移**：runtime 实际能力变化（例如 ccb 主动跟进 v1.2）但 ADR 表未更新，会让 reviewer 误判。缓解：每次 contract-v1.x bump 必须连同 ADR-V2-025 表格一起 review。
- **主力档过窄**：当前只有 grid + claude-code 在主力档，`/adr:trace` 受影响 modules 也以这两为主。如果未来 grid-cli 增加新依赖（比如 nanobot 作 fallback runtime），需要 ADR amendment 而不是默默把 nanobot 提到主力档。
- **冻结档误用**：hermes 之外不应轻易添加冻结档 runtime — 冻结意味着"不再演进且不计 CI 时间"，应只在 ADR-V2-017 §冻结决议这种正式动议下使用。

---

## Verification / 验证

### F1 — Frontmatter 合法

```bash
python3 -c 'import yaml; yaml.safe_load(open("docs/design/EAASP/adrs/ADR-V2-025-l1-runtime-contract-tier-strategy.md"))'
grep -E '^\-\-\-' docs/design/EAASP/adrs/ADR-V2-025*.md | wc -l   # → 2
```

### F2 — Status accepted

```bash
grep '^status: Accepted' docs/design/EAASP/adrs/ADR-V2-025*.md   # exit 0
```

### F3 — Tier table 完整 (4 档 + 7 active runtime + hermes frozen note)

```bash
grep -E '主力档|样板档|参考档|冻结档' docs/design/EAASP/adrs/ADR-V2-025*.md   # 4 hits
grep 'hermes' docs/design/EAASP/adrs/ADR-V2-025*.md                          # frozen note 行
grep -E 'grid|claude-code|nanobot|pydantic-ai|goose|claw-code|ccb' docs/design/EAASP/adrs/ADR-V2-025*.md  # 7 runtime
```

### F4 — References 引用正确

```bash
grep -E 'ADR-V2-017|ADR-V2-021|ADR-V2-024|NEW-D2' docs/design/EAASP/adrs/ADR-V2-025*.md   # 全部 hit
```

### Trace files 存在

- `tests/contract/cases/test_chunk_type_contract.py` — Phase 5.1 T2 后包含 `@pytest.mark.parametrize("runtime_name", [...])`
- `.github/workflows/phase3-contract.yml` — Phase 5.1 T3 后 matrix 含 `tier:` + `xfail:` 字段，hermes skip 守护

---

## References / 参考

- **ADR-V2-017** — L1 Runtime 生态策略（主力 + 样板 + 对比 三轨）— 本 ADR 的产品策略基础
- **ADR-V2-021** — SendResponse.chunk_type 契约冻结（统一枚举）— whitelist/UNSPECIFIED 禁止/DONE 强制；本 ADR 的契约 invariant 锚点
- **ADR-V2-024** — Phase 4 Product Scope Decision (双轴模型 — engine vs data/integration)；本 ADR 在双轴模型 engine 接入面 primary focus 下落地
- **NEW-D2** — Phase 4a project review 发现 test_chunk_type_contract.py 仅 3 tests, not 7-runtime parametric — 本 ADR 的直接触发点；Phase 5.1 T2 闭环
- **WATCH-05** — `.planning/REQUIREMENTS.md` Active 列表中 NEW-D2 的对应 REQ 项
- **CONTRACT-00** — `.planning/REQUIREMENTS.md` Active 列表中本 ADR 候选项
