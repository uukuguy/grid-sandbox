# Provider × Model 能力矩阵

**创建日期**: 2026-04-14
**关联**: D87 Fix 2 (L2b) · ADR-V2-016 · `crates/grid-engine/src/providers/capabilities.rs`

---

## 背景

Agent loop 的部分机制（如 D87 的 `tool_choice=Required` continuation）**只在特定 provider × model 组合下可用**。业界没有可靠的能力声明 API（OpenRouter `/models` 端点不返回 `supports_tool_choice`），所以 grid-engine 自己维护一张能力表，外加启动探测补全。

**原则**：
1. **能力归 provider + model 组合**，不归 skill（skill 作者不决定是否用 tool_choice）
2. **大声失败 > 静默降级**：已知不支持的组合，runtime 根本不触发该机制；已知支持但运行时 400，真的报错，不重试
3. **探测一次、缓存一次**：每个 `(provider, model, base_url)` 组合的 probe 结果缓存在进程内，session 级不重测

---

## 能力矩阵（当前已知）

### `tool_choice` 支持性

| Provider | base_url 特征 | Model 特征 | 支持? | 来源 |
|---------|--------------|----------|-----|------|
| openai | `api.openai.com` 或空 | `gpt-4*` / `gpt-3.5-turbo*` | ✅ Supported | 官方文档 |
| anthropic | `api.anthropic.com` 或空 | `claude-3*` / `claude-sonnet*` / `claude-opus*` / `claude-haiku*` | ✅ Supported | 官方文档 |
| openai（proxy 模式） | `openrouter.ai` | any | ⚠️ Unknown → 探测 | OpenRouter 动态路由 |
| 其他 | 其他 base_url（vLLM / LM Studio / …） | any | ⚠️ Unknown → 探测 | 自定义部署 |

**具体后端观察到的不支持案例**（OpenRouter → 某后端）：

| OpenRouter 路由后端 | 现象 |
|------------------|-----|
| AtlasCloud（qwen3.5） | `400 invalid parameter`（2026-04-14 观察） |
| Together、Fireworks、Groq | 通常支持（待逐个探测确认） |

**OpenRouter 路由动态性**：同一个 `(openrouter, qwen3.5)` 组合，不同时间可能路由到不同后端。**进程级缓存可能会陈旧**——一旦被降级为 Unsupported，重启进程才重新探测。这是已知限制。

---

## 判定流程

```
启动探测（session initialize 时）
  ├─ 静态 baseline 命中 Supported  → 直接用
  ├─ 静态 baseline 命中 Unsupported → 跳过相关机制（D87 continuation 不触发）
  └─ Unknown → 发 probe request
        ├─ 200 → 缓存为 Supported
        └─ 400  → 缓存为 Unsupported + warn log
```

**Probe request 设计**（最小成本）：
- 方法：`stream()`（保持和真实 request 一致的路径）
- messages: 一条 user `ping`
- tools: 一个最简 stub tool（`{"name":"ping","parameters":{"type":"object","properties":{}}}`）
- tool_choice: `Required`
- max_tokens: 8（极省 token）
- 对响应内容不做验证，只看 HTTP 状态码

### Probe 缓存粒度

缓存 key：`(provider, model, base_url)`。`base_url` 区分了 "OpenAI 直连" vs "OpenRouter" vs "vLLM 私有部署"。

---

## 配置方式（用户侧）

### 覆盖默认能力（如果需要）

未来可扩展 `config.yaml`：

```yaml
provider_capabilities:
  - provider: openai
    base_url: https://openrouter.ai/api/v1
    model: qwen/qwen3.5-122b-a10b
    tool_choice: unsupported   # 强制跳过探测
```

（当前实现未支持 YAML 覆盖，只有代码内的 `CapabilityStore::record()`；配置覆盖是 Phase 3 待办。）

---

## 为什么不靠 fallback

我们试过"400 自动降级去掉 tool_choice 重发一次"，放弃了：

1. **两次请求不优雅**：latency × 2；排查困难（用户看到日志是 "成功" 但是 fallback 降级后的）
2. **违反 loud failure 原则**：配置错了就应该报错，不应该静默绕过
3. **掩盖配置问题**：第一次就 fallback 的用户永远不知道自己的 provider 不支持 tool_choice
4. **与 runtime 侦错体验不符**：grid-engine 的错误应该精确归因到配置

正确做法是**启动时就知道**（静态表或 probe），**不支持就不用**，**支持就放心用**。

---

## Phase 扩展路线

### 当前（Phase 2 S1.T1 持续中）
- [x] `capabilities.rs` 静态表 + `CapabilityStore` 缓存
- [ ] 启动探测 logic（Step 4）
- [ ] harness 决策根据 capability 决定是否 arm `force_tool_choice_next_call`

### Phase 3（独立 plan）
- [ ] 原生 OpenRouter provider（`OpenAICompatibleProvider + OpenAIFlavor`）—— 可以读 `x-openrouter-provider` 响应头按真实后端做更精准的 capability 记录
- [ ] vLLM provider
- [ ] LM Studio provider
- [ ] YAML-based capability override
- [ ] 发现新 provider 时自动追加静态表 PR 的 CI 工作流

### Phase 4 及以后
- [ ] Capability auto-discovery — 按响应头反推后端（OpenRouter 专用）
- [ ] 跨 session 缓存持久化（磁盘/Redis）
- [ ] 前端 UI 暴露 capability 表供 ops 观察

---

## 设计决策日志

**Q1: 为什么不在 skill frontmatter 声明 tool_choice?**
A: tool_choice 是 provider + model 的**技术能力**，不是 skill 作者能决定的。skill 作者只知道"我这个工作流需要多步调用"，不知道底层 LLM 支不支持。让 skill 作者填这个字段违反职责分离。

**Q2: 为什么不每次请求都 probe?**
A: Probe 有成本（一次 LLM 调用），重复做浪费 token + latency。进程级缓存够用——运维重启服务就重新探测。

**Q3: 为什么 OpenRouter 默认 Unknown 而不是 Unsupported?**
A: OpenRouter 路由的后端大多数是支持的（Together/Fireworks 都 OK），只有少数像 AtlasCloud 不支持。默认标 Unsupported 会牺牲大多数场景。探测一次成本可接受。

**Q4: 如果 probe 成功但后来 runtime 调用 400 怎么办?**
A: 当作真错误处理，不再自动降级。错误日志里明确提示 "provider 能力可能已变更，考虑清除 capability 缓存"。用户可以手动重启服务重探测。

---

## 关联文档

- `docs/design/EAASP/AGENT_LOOP_ROOT_CAUSE_ANALYSIS.md` — D87 根因
- `docs/design/EAASP/AGENT_LOOP_PATTERNS_TO_ADOPT.md` — 可吸收 loop 优点清单
- `docs/design/EAASP/adrs/ADR-V2-016-agent-loop-generic-principle.md` — ADR 草稿
- `docs/plans/2026-04-14-v2-phase2-plan.md` — Phase 2 plan
