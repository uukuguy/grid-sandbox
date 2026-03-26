# octo-sandbox 下一会话指南

**最后更新**: 2026-03-26 17:30 GMT+8
**当前分支**: `main`
**当前状态**: MCP/TUI 健壮性 + 自定义命令完成，无活跃阶段

---

## 项目状态

```
Custom Commands + TUI Fixes               → COMPLETE @ 263eeb2
Post-AF: Builtin Skills + Config + TUI Fix → COMPLETE @ 072c15b
Phase AF: SSM Wiring + Deferred Batch     → COMPLETE @ 976e813
Phase AE: Agent Workspace Architecture    → COMPLETE @ ee4986f
Phase AD: Container Image Enhancement     → COMPLETE @ 73295f5
Phase AC: Sandbox Container               → COMPLETE @ 184b1ab
Phase AB: 智能体工具执行环境               → COMPLETE @ 282d3f6
Phase AA: Octo 部署配置架构               → COMPLETE @ 4fbc30d
Phase Z-A: Core Engine + CLI + Eval       → ALL COMPLETE
Wave 1-10: Foundation                     → COMPLETE @ 675155d
```

### 基线数据

- **Tests**: 2476 passing
- **测试命令**: `cargo test --workspace -- --test-threads=1`

---

## 本次会话完成摘要

1. **TUI stdin 隔离**: 8 个子进程创建点加 `Stdio::null()`，防止 ANSI escape 泄漏到输入区
2. **UTF-8 安全截断**: 4 处字符串截断改用 `safe_truncate_utf8()`
3. **表格渲染**: 自适应终端宽度 + HTML 标签清理
4. **Qwen XML 工具调用恢复**: 支持非标准 LLM 的 XML 风格工具调用
5. **自定义斜杠命令**: `.octo/commands/*.md` → TUI `/命令名` + 自动补全 + 参数展开
6. **10 个内置命令**: review, explain, refactor, test, fix, doc, optimize, summarize, translate, commit

---

## Deferred 未清项

| 来源 | ID | 内容 | 前置条件 | 状态 |
|------|----|----|---------|------|
| Phase AB | AB-D1 | Octo sandbox Docker image | 基础串联完成 | 🟢 可实施 |
| Phase AB | AB-D2 | E2B provider 完整实现 | External trait 稳定 | 🟢 可实施 |
| Phase AB | AB-D3 | WASM plugin loading | WASM 路由激活 | ⏳ |
| Phase AB | AB-D4 | Session Sandbox 持久化 | BashTool 沙箱集成 | 🟢 可实施 |
| Phase AB | AB-D5 | CredentialResolver → sandbox env 注入 | Z-D1 完成 | ⏳ |
| Phase AB | AB-D6 | gVisor / Firecracker provider | External trait 稳定 | 🟢 可实施 |
| Phase AC | AC-D1 | CI/CD pipeline (GitHub Actions) | 低优先级 | ⏳ |
| Phase AC | AC-D4 | Multi-image support | 低优先级 | ⏳ |
| Phase AC | AC-D5 | Container snapshots | 低优先级 | ⏳ |
| Phase AC | AC-D6 | Docker Compose | 低优先级 | ⏳ |
| Phase AD | AD-D1 | LibreOffice in container | 镜像体积考虑 | ⏳ |
| Phase AD | AD-D2 | Cloud variant images | 低优先级 | ⏳ |
| Phase AD | AD-D3 | cosign image signing | 安全增强 | ⏳ |
| Phase AD | AD-D4 | Octo CLI in container | CLI 稳定后 | ⏳ |
| Phase AD | AD-D6 | Docling in container | 文档处理增强 | ⏳ |
| Phase AA | AA-D1 | `octo auth login/status/logout` | UX 设计 | ⏳ |
| Phase AA | AA-D3 | XDG Base Directory 支持 | 低优先级 | ⏳ |
| Phase AA | AA-D4 | Config 热重载 | 未来增强 | ⏳ |
| Phase Z | Z-D1 | CredentialResolver → provider chain | Config 稳定 | 🟡 |

---

## 关键代码路径

| 文件 | 作用 |
|------|------|
| `crates/octo-engine/src/commands.rs` | 自定义命令加载器 + builtin sync |
| `crates/octo-engine/builtin/commands/` | 10 个内置命令模板 |
| `crates/octo-engine/builtin/skills/` | 内置 skills 源目录（编译进二进制） |
| `crates/octo-engine/src/skills/initializer.rs` | include_dir! 嵌入 + sync_builtin_skills() |
| `crates/octo-engine/src/root.rs` | OctoRoot + commands_dirs() + seed_default_config() |
| `crates/octo-engine/src/sandbox/` | SandboxProfile, SSM, Docker, 路由 |
| `crates/octo-engine/src/agent/runtime.rs` | AgentRuntime SSM 集成 + skills sync |
| `crates/octo-cli/src/tui/key_handler.rs` | async execute_slash_command + 自定义命令分发 |
| `crates/octo-cli/src/tui/mod.rs` | TUI 启动 sync + 命令加载 + 自动补全注册 |
| `config.default.yaml` | 全量配置参考（编译时嵌入） |

---

## 快速启动

```bash
# 编译检查
cargo check --workspace

# 全量测试
cargo test --workspace -- --test-threads=1

# TUI 模式（使用 demo-project）
make cli-tui

# CLI 交互模式
make cli-run

# 启动 server + web
make dev
```
