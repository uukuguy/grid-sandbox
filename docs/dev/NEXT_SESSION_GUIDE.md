# Grid Platform 下一会话指南

**最后更新**: 2026-04-06 16:00 GMT+8
**当前分支**: `Grid`
**当前状态**: Phase BD 完成 — grid-runtime EAASP L1 ✅

---

## 完成清单

- [x] Phase A-Z — Core Engine + Eval + TUI + Skills
- [x] Phase AA-AF — Sandbox/Config/Workspace architecture
- [x] Phase AG-AI — Memory/Hooks/WASM enhancement
- [x] Phase AJ-AO — 多会话/安全/前端/服务器
- [x] Phase AP-AV — 追赶 CC-OSS + 安全对齐
- [x] Phase AW-AY — 工具/Agent/SubAgent 体系
- [x] Phase AZ — Cleanup/Transcript/Completion
- [x] Phase BA — Octo to Grid 重命名 + TUI 完善
- [x] Phase BB-BC — TUI 视觉升级 + Deferred 补齐
- [x] **Phase BD — grid-runtime EAASP L1** (6/6, 37 tests @ ae4b337)

## 下一步优先级

按 EAASP 路线图 (`docs/design/Grid/EAASP_ROADMAP.md`) 推进：

1. **Phase BE — eaasp-certifier** (§7.2)
   - 独立验收工具，验证任意运行时是否符合 EAASP 契约
   - gRPC 客户端 + 测试场景 + 评分报告
   - 可先用 grpcurl 手动验证 grid-runtime

2. **Phase BF — hook.proto + HookBridge** (§6.3, §10.4)
   - L3 → L1 hooks 通信协议
   - HookBridge sidecar 为 Tier 2/3 运行时补全 hooks 能力

3. **Phase BG — L2 技能资产层** (§5)

## 关键代码路径

| 组件 | 路径 |
|------|------|
| RuntimeContract trait | `crates/grid-runtime/src/contract.rs` |
| GridHarness | `crates/grid-runtime/src/harness.rs` |
| gRPC service | `crates/grid-runtime/src/service.rs` |
| Telemetry | `crates/grid-runtime/src/telemetry.rs` |
| Server entry | `crates/grid-runtime/src/main.rs` |
| Config | `crates/grid-runtime/src/config.rs` |
| Proto | `proto/eaasp/runtime/v1/runtime.proto` |
| EAASP 规范 | `docs/design/Grid/EAASP_-_企业自主智能体支撑平台设计规范_v1.7_.pdf` |
| 路线图 | `docs/design/Grid/EAASP_ROADMAP.md` |
| 沙箱执行设计 | `docs/design/Grid/EAASP_SANDBOX_EXECUTION_DESIGN.md` |

## 关键 API 模式（BD 中发现）

- `SessionId::from_string()` 不是 `from()`
- `AgentMessage::UserMessage { content, channel_id }` 是 struct variant
- `McpManager.add_server` 接受 `McpServerConfig`，需 `.into()` 从 V2 转
- `TelemetryBus.recent_events` 是 async
- `ToolRegistry.names()` 返回 `Vec<String>`
- tonic 0.12 生成的 trait: `RuntimeService`，server: `RuntimeServiceServer`
- `type SendStream` 要求 `Stream<Item = Result<ResponseChunk, Status>> + Send + 'static`

## ⚠️ Deferred 未清项（下次 session 启动时必查）

| 来源 | ID | 内容 | 前置条件 |
|------|----|----|---------|
| BD | D1 | grid-hook-bridge crate（Tier 2/3 sidecar） | hook.proto + L3 治理层 |
| BD | D2 | RuntimeSelector + AdapterRegistry | 2+ 运行时可对比 |
| BD | D3 | 盲盒对比 | 非 Grid 运行时接入 |
| BD | D4 | managed-settings.json 分发 | L3 治理层 crate |
| BD | D5 | SessionPayload 组织层级 | L4 多租户层 |
| BD | D6 | initialize() payload 字段传递到 engine | grid-engine start_session 扩展参数 |
| BD | D7 | emit_telemetry 填充 user_id | session 存储 user_id 关联 |
