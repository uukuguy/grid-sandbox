# ADR-001: AgentRuntime 模块化架构

**项目**: octo-sandbox
**日期**: 2026-03-07
**状态**: 已接受

## 上下文

Agent 模块需要支持完整的 Agent 生命周期管理，包括：
- Agent 运行时初始化和配置
- Agent 实例创建和销毁
- 多租户隔离
- 工具和 MCP 服务集成

原有设计将所有功能集中在单一模块中，导致代码耦合度高。

## 决策

采用模块化架构，将 Agent 拆分为多个子模块：

| 子模块 | 职责 |
|--------|------|
| runtime.rs | AgentRuntime 主入口 |
| executor.rs | AgentExecutor，每个会话的 Agent 实例 |
| loop.rs | AgentLoop，单轮对话循环 |
| catalog.rs | AgentCatalog，状态机 |
| router.rs | AgentRouter，任务路由 |
| manifest_loader.rs | YAML 声明式加载 |

## 后果

- **正面**: 职责分离，可维护性提升
- **负面**: 初期开发工作量增加
