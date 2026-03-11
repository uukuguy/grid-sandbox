# 暂缓项实施方案 — Wave 3 + Wave 4

> **目标**: 基于 4 个研究智能体的代码分析结果，规划 7 个暂缓项的实施优先级和详细方案。
>
> **基线**: 1408 tests passing @ commit `b5d7e61`
>
> **前置完成**: Wave 1 (T1-T5) + Wave 2 (T6-T10) 全部 COMPLETE

---

## 总览

| Wave | 主题 | Tasks | 可行性 | 预估工期 |
|------|------|-------|--------|---------|
| **Wave 3 (P0)** | 架构清理 + 跨 Provider 路由 + 多模态 | D2, D7, D3 | 中等 | 3-5 天 |
| **Wave 4 (P1)** | Byzantine 共识 + 桌面更新 | D1, D5 | 中-高 | 5-8 天 |
| **Wave 5 (P2)** | 离线同步 + 自动证书 | D6, D4 | 高复杂度 | 暂缓 |

---

## 优先级排序依据

| 排序因素 | D1 | D2 | D3 | D4 | D5 | D6 | D7 |
|---------|----|----|----|----|----|----|-----|
| 前置条件满足 | ✅ | ✅ | ⚠️ | ❌ | ⚠️ | ❌ | ✅ |
| 代码复杂度 | 高 (2400-3200 LOC) | 低 (删除+重构) | 中 (400-500 LOC) | 低 (代码) / 高 (基础设施) | 低 (75 LOC) / 高 (CI/CD) | 高 (800-3000 LOC) | 中 (800-1000 LOC) |
| 用户价值 | 中 | 中 | 高 | 低 | 中 | 中 | 高 |
| 风险等级 | 中-高 | 低 | 低-中 | 中 | 中 | 中-高 | 低-中 |

---

## Wave 3: 架构清理 + 跨 Provider + 多模态

### D2: Extension 系统清理 + Hook 统一

**研究结论**: Extension 系统 (799 LOC) 是**纯脚手架**，零运行时调用。Hook 系统 (536 LOC) 已生产就绪，harness.rs 中 11 处活跃调用。

**决策**: **废弃 Extension 系统，统一到 Hook 系统**。

**任务分解**:

| Task | 内容 | 文件 | 估计 |
|------|------|------|------|
| D2-1 | 废弃 `extension/` 模块：标记 `#[deprecated]` 或直接删除 | `extension/mod.rs`, `traits.rs`, `manager.rs`, `context.rs` | 0.5 天 |
| D2-2 | 清理 `agent/extension.rs` 轻量事件注册表，合并到 EventBus | `agent/extension.rs`, `event/bus.rs` | 0.5 天 |
| D2-3 | 更新 `lib.rs` 导出，移除 extension 模块引用 | `lib.rs`, 受影响的 import | 0.25 天 |
| D2-4 | 文档更新：Hook 系统为主要扩展点 | 无需新增文档文件 | — |

**总计**: ~1 天，净减少 ~900 LOC

**风险**: 低 — Extension 系统无运行时使用者

---

### D7: SmartRouting V2 — 跨 Provider 路由

**研究结论**: V1 仅做单 Provider 内 model override。V2 需支持 `tier → (provider, model)` 映射。ProviderChain 已有多实例管理但仅做 failover。

**核心架构变更**:
- `TierConfig` 扩展：添加 `provider: String` 字段
- `SmartRouterProvider` 重构：持有 `HashMap<QueryComplexity, Arc<dyn Provider>>` 而非单一 inner
- `runtime.rs` 支持多 Provider 加载

**任务分解**:

| Task | 内容 | 文件 | 估计 |
|------|------|------|------|
| D7-1 | 扩展 `TierConfig`：添加 `provider` 字段 + 配置反序列化 | `smart_router.rs`, `config.rs` | 0.5 天 |
| D7-2 | 新建 `CrossProviderRouter`：多 Provider 选择逻辑 | `smart_router.rs` (扩展) | 1 天 |
| D7-3 | `runtime.rs` 多 Provider 实例化 | `runtime.rs` | 0.5 天 |
| D7-4 | Pipeline 集成 `with_cross_provider_routing()` | `pipeline.rs` | 0.5 天 |
| D7-5 | 测试：跨 Provider 路由 + fallback + 配置反序列化 (10+ tests) | `tests/smart_routing_v2.rs` | 0.5 天 |

**总计**: ~3 天 (可与 D2 并行)，新增 ~800-1000 LOC

**风险**: 中 — Provider 间 tool 兼容性差异需处理

---

### D3: ContentBlock 扩展 (Image/Audio)

**研究结论**: `ContentBlock` 枚举当前仅支持 Text/ToolUse/ToolResult。Anthropic 和 OpenAI 均支持 image，但格式不同（Anthropic: base64, OpenAI: URL）。

**任务分解**:

| Task | 内容 | 文件 | 估计 |
|------|------|------|------|
| D3-1 | 扩展 `ContentBlock` 枚举：添加 `Image` 和 `Document` 变体 | `octo-types/src/message.rs` | 0.5 天 |
| D3-2 | Anthropic adapter：`convert_messages()` 处理 Image block (base64) | `providers/anthropic.rs` | 0.5 天 |
| D3-3 | OpenAI adapter：`convert_messages()` 处理 Image block (URL) | `providers/openai.rs` | 0.5 天 |
| D3-4 | 测试：Image ContentBlock 序列化/反序列化 + Provider 转换 (8+ tests) | `tests/content_block.rs` | 0.5 天 |

**总计**: ~2 天，新增 ~400-500 LOC

**Audio 暂缓**: API 支持尚不成熟，D3 仅实现 Image

**风险**: 低 — ContentBlock 已可扩展，Provider adapter 独立修改

---

## Wave 4: Byzantine 共识 + 桌面更新

### D1: Byzantine 共识 (PBFT lite)

**研究结论**: 当前协作系统仅支持 crash-fault tolerance（节点离线）。Byzantine fault tolerance 需要 3 阶段共识（pre-prepare/prepare/commit）+ 加密签名 + 仲裁阈值 + 视图变更。

**分阶段实施**:

| Phase | 内容 | 估计 |
|-------|------|------|
| **D1-P1: 基础共识** | 仲裁阈值 + 自动共识检查 + ConsensusPhase 状态机 | 2 天 |
| D1-P1-1 | `ConsensusPhase` 枚举 (PrePrepare/Prepare/Commit/Finalized) | `collaboration/context.rs` |
| D1-P1-2 | `ByzantineProposal` 结构体 + 仲裁计算 | `collaboration/consensus.rs` (新建) |
| D1-P1-3 | `ConsensusMessage` 消息类型 | `collaboration/channel.rs` |
| D1-P1-4 | 自动 phase 转换逻辑 | `collaboration/protocol.rs` |
| D1-P1-5 | 测试 (15+ tests) | `tests/byzantine_consensus.rs` |
| **D1-P2: 安全加固** | ED25519 签名 + 视图变更 | 3 天 |
| D1-P2-1 | `collaboration/crypto.rs` (新建): 签名/验证 | ed25519-dalek 依赖 |
| D1-P2-2 | 视图变更 + 主节点选举 | `collaboration/manager.rs` |
| D1-P2-3 | 超时检测 + 心跳 | Tokio interval |
| **D1-P3: 持久化** | 共识日志 + 前端可视化 | 2 天 |
| D1-P3-1 | SQLite 共识状态表 | DB migration |
| D1-P3-2 | ProposalList 显示共识阶段进度 | 前端组件 |

**总计**: ~7 天，新增 ~2400-3200 LOC

**建议**: D1-P1 可独立交付（基础共识），P2/P3 可后续迭代

**风险**: 中-高 — 多 Agent 消息顺序竞争条件

---

### D5: Tauri 自动更新

**研究结论**: Tauri 2.0 已就位，`tauri-plugin-updater` v2 成熟。代码量极小 (~75 LOC)，但需要发布基础设施（CI/CD + 签名 + artifact 托管）。

**任务分解**:

| Task | 内容 | 文件 | 估计 |
|------|------|------|------|
| D5-1 | 添加 `tauri-plugin-updater` 依赖 + 初始化 | `octo-desktop/Cargo.toml`, `lib.rs` | 0.25 天 |
| D5-2 | `tauri.conf.json` 更新器配置 | `tauri.conf.json` | 0.25 天 |
| D5-3 | GitHub Actions 多平台构建 workflow | `.github/workflows/release.yml` (新建) | 1 天 |
| D5-4 | ED25519 密钥生成 + manifest 生成脚本 | `scripts/release-sign.sh` | 0.5 天 |

**总计**: ~2 天

**前置条件**: GitHub Actions 权限 + artifact 存储决策（GitHub Releases vs S3）

**风险**: 中 — 基础设施依赖

---

## Wave 5 (暂缓): 离线同步 + 自动证书

### D6: 离线模式 SQLite 同步

**研究结论**: 数据库 schema 就绪（sessions、memories、collaboration events）。建议 LWW（last-write-wins）方案优先（~800-1000 LOC），CRDT 方案作为 V2（~2000-3000 LOC）。

**暂缓原因**:
- 需要先完成冲突解决策略设计文档
- 需要同步协议（增量交换）设计
- 依赖 D5（Tauri 更新器）完成后才有离线桌面场景

**预估**: 5-8 天 (LWW) / 15-20 天 (CRDT)

---

### D4: Let's Encrypt ACME 自动证书

**暂缓原因**:
- 需要公网域名和平台生产部署环境
- 当前开发阶段无需

---

## 依赖关系

```
D2 (Extension 清理) ──────(独立)
D3 (Image/Audio) ────────(独立)
D7 (SmartRouting V2) ────(独立，复用 V1)

D1 (Byzantine) ──────────→ D6 (离线同步需共识)
D5 (Tauri 更新) ─────────→ D6 (离线同步需桌面分发)
D4 (ACME 证书) ──────────(需生产环境)
```

## 执行波次

```
Wave 3 并行 (3-5 天):
  Agent-A: D2 (Extension 清理)           ← 独立，1 天
  Agent-B: D7 (SmartRouting V2)          ← 独立，3 天
  Agent-C: D3 (ContentBlock Image)       ← 独立，2 天

Wave 4 串行 (5-8 天):
  Agent-D: D1-P1 (基础共识)              ← 2 天
  Agent-D: D1-P2 (安全加固)              ← 3 天 (可选)
  Agent-E: D5 (Tauri 更新器)             ← 2 天 (可与 D1 并行)

Wave 5 暂缓:
  D6: 离线同步 (需 D1+D5 完成)
  D4: ACME 证书 (需生产环境)
```

## 提交策略

```
Wave 3:
  commit 1: "refactor: D2 — Deprecate Extension system, unify to Hook system"
  commit 2: "feat(providers): D7 — SmartRouting V2 cross-provider routing"
  commit 3: "feat(types): D3 — Image ContentBlock support"
  checkpoint: "checkpoint: Wave 3 COMPLETE — D2+D7+D3"

Wave 4:
  commit 4: "feat(collaboration): D1-P1 — PBFT-lite consensus state machine"
  commit 5: "feat(collaboration): D1-P2 — Cryptographic signing + view change"
  commit 6: "feat(desktop): D5 — Tauri auto-updater integration"
  checkpoint: "checkpoint: Wave 4 COMPLETE — D1+D5"
```

---

## Deferred（暂缓项） → 已迁移至 Wave 5 计划

> **所有暂缓项已迁移到**: `docs/plans/2026-03-11-wave5-execution.md`
> **设计文档**: `docs/design/WAVE5_DEFERRED_DESIGN.md`

| ID | 内容 | 新计划中的 Task | 状态 |
|----|------|---------------|------|
| D1-P3 | Byzantine 共识持久化 | Wave 5a: D1-P3-T1~T7 | 📋 READY |
| D4-lite | TLS 配置 + 自签名 + 部署模板 | Wave 5c: D4-T1~T6 | 📋 READY |
| D6 | 离线模式 SQLite 同步 (LWW+HLC) | Wave 5b: D6-T1~T9 | 📋 READY |
| D6-V2 | CRDT 离线同步 | 仍暂缓 | ⏳ |

---

## 验收标准

### Wave 3 完成标准
- [ ] `cargo check --workspace` 无错误
- [ ] `cargo test --workspace -- --test-threads=1` 全部通过
- [ ] Extension 模块已废弃/删除，Hook 系统为唯一扩展点
- [ ] SmartRouting V2: `simple → OpenAI gpt-4o-mini`, `complex → Anthropic opus` 路由验证
- [ ] ContentBlock: Image 类型在 Anthropic/OpenAI provider 正确序列化

### Wave 4 完成标准
- [ ] Byzantine: 4 agent 场景下，2f+1 prepare/commit → proposal finalized
- [ ] Byzantine: 签名验证通过，伪造投票被拒绝
- [ ] Tauri: 更新检查 API 可调用（本地 mock endpoint）
