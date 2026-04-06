# Grid Platform 下一会话指南

**最后更新**: 2026-04-06 14:00 GMT+8
**当前分支**: `Grid`
**当前状态**: Phase BD — grid-runtime 进行中 (2/6 waves, W3 plan ready)

---

## 当前活跃 Phase

**Phase BD — grid-runtime (EAASP L1)** — 33% 完成

| Wave | 内容 | 状态 | Commit |
|------|------|------|--------|
| W1 | 新建 crate + proto + RuntimeContract trait | ✅ | 02dfa82 |
| W2 | GridHarness 13 方法桥接 grid-engine | ✅ | f8b8e3d |
| W3 | proto v1.2 + gRPC server + config + tests + Dockerfile | ⏳ plan ready | — |
| W4 | 遥测 schema + 转换 | ⏳ | — |
| W5 | gRPC 集成测试 (certifier 降级) | ⏳ | — |
| W6 | Dockerfile + 容器化 | ⏳ | — |

### 本次会话产出

1. **EAASP 中长期路线图**: `docs/design/Grid/EAASP_ROADMAP.md`
   - 阅读了 EAASP 设计规范 v1.7（46 页 PDF）完整内容
   - 阅读了预设计文档 `docs/plans/claude-code-runtime/` 两份
   - 对齐规范五阶段演进策略（BD→BE→BF→BG→BH→BI→BJ+）
   - 9 个已确定设计决策（KD-1~KD-7），含 Tier 定义、容器化常态、SDK 定位

2. **W3 实施计划**: `docs/plans/2026-04-06-bd-w3-grpc-server.md`
   - 6 个 task：proto v1.2 升级 → config.rs → service.rs → main.rs → 集成测试 → Dockerfile
   - W5 certifier 降级为 crate 内集成测试（独立工具留 Phase BE）
   - proto v1.2 新增 3 个 RPC（DisconnectMcp/PauseSession/ResumeSession）

### 下一步: 执行 W3 计划

```bash
/resume-plan    # 加载 W3 计划
# 选择执行模式（subagent-driven 或 executing-plans）
```

W3 计划 6 个 task 的执行顺序：
1. Proto v1.2 升级 + contract/harness 更新
2. config.rs 配置模块
3. service.rs gRPC service 实现
4. main.rs server 入口
5. gRPC 集成测试
6. Dockerfile + Makefile

## 关键代码路径

- RuntimeContract trait: `crates/grid-runtime/src/contract.rs`
- GridHarness: `crates/grid-runtime/src/harness.rs`
- Proto: `proto/eaasp/runtime/v1/runtime.proto`
- Generated proto: `crates/grid-runtime/src/lib.rs` → `proto` module
- EAASP 规范: `docs/design/Grid/EAASP_-_企业自主智能体支撑平台设计规范_v1.7_.pdf`
- 路线图: `docs/design/Grid/EAASP_ROADMAP.md`

## 关键 API 模式（W2 中发现）

- `SessionId::from_string()` 不是 `from()`
- `AgentMessage::UserMessage { content, channel_id }` 是 struct variant
- `McpManager.add_server` 接受 `McpServerConfig`，需 `.into()` 从 V2 转
- `TelemetryBus.recent_events` 是 async
- `ToolRegistry.names()` 返回 `Vec<String>`
- `tokio-stream` 需要 `features=["sync"]` for `BroadcastStream`
- tonic 0.12 生成的 trait: `RuntimeService`，server: `RuntimeServiceServer`
- `type SendStream` 要求 `Stream<Item = Result<ResponseChunk, Status>> + Send + 'static`

## 设计决策要点（来自路线图）

- **KD-1**: T1=原生Harness, T2=HookBridge补全, T3=厚适配器（对齐规范§7.1）
- **KD-3**: L1/L3 通过 hooks 通信，不是 REST API（规范§9）
- **KD-4**: 容器临时的——无本地持久化，terminate 必须 flush（规范§11.5）
- **KD-5**: Enterprise SDK ≠ L1 Runtime，SDK 等 L3 稳定后再做
- **KD-6**: proto 全局共享在 `proto/eaasp/`

## Deferred Items

| ID | 内容 | 前置条件 |
|----|------|---------|
| BD-D1 | grid-hook-bridge（Tier 2/3 sidecar） | Phase BE HookBridge |
| BD-D2 | RuntimeSelector + AdapterRegistry | Phase BF L1 抽象 |
| BD-D3 | 盲盒对比 | BF 2+ 运行时 |
| BD-D4 | managed-settings.json 分发 | BH L3 治理层 |
| BD-D5 | SessionPayload 组织层级 | BH L4 多租户 |
