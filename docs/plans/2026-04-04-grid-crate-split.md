# Grid Crate 拆分实施计划

> 创建日期: 2026-04-04
> 状态: 待执行
> 设计文档: [GRID_CRATE_SPLIT_DESIGN.md](../design/Grid/GRID_CRATE_SPLIT_DESIGN.md)
> 产品文档: [GRID_PRODUCT_DESIGN.md](../design/Grid/GRID_PRODUCT_DESIGN.md)

---

## 目标

将 `octo-cli` (24,160 行) 拆分为三个独立 crate，并完成全局 octo → grid 品牌重命名。

**前提**：新项目未上线，无向后兼容需求，一步到位。

---

## Phase BA：Grid Crate 拆分 + 品牌重命名

### Step 1: 全局 Crate 重命名（骨架）

**目标**：重命名所有 crate，保持编译通过，不拆分 octo-cli。

| 任务 | 说明 | 文件 |
|------|------|------|
| BA-S1.1 | workspace Cargo.toml 更新 crate 名和路径 | `Cargo.toml` |
| BA-S1.2 | 重命名 crate 目录: `octo-types` → `grid-types` 等 | 所有 `crates/` |
| BA-S1.3 | 更新所有 `Cargo.toml` 中的依赖引用 | 每个 crate 的 `Cargo.toml` |
| BA-S1.4 | 全局替换 `use octo_types::` → `use grid_types::` 等 | 所有 `.rs` 文件 |
| BA-S1.5 | 更新 Makefile 中的 crate 引用 | `Makefile` |
| BA-S1.6 | `cargo check --workspace` 通过 | — |
| BA-S1.7 | `cargo test --workspace -- --test-threads=1` 通过 | — |

**产出**：所有 crate 重命名为 grid-*，编译测试通过。

### Step 2: 环境变量 + 路径重命名

**目标**：OCTO_* → GRID_*，.octo/ → .grid/

| 任务 | 说明 | 文件 |
|------|------|------|
| BA-S2.1 | OctoRoot → GridRoot 类型重命名 | `grid-engine/src/root/` |
| BA-S2.2 | `OCTO_*` 环境变量 → `GRID_*` | 全局 grep + 替换 |
| BA-S2.3 | `.octo/` 目录引用 → `.grid/` | 全局 grep + 替换 |
| BA-S2.4 | `data/octo.db` → `data/grid.db` | config, root 模块 |
| BA-S2.5 | 配置文件中的 octo → grid | `config.yaml`, `.env.example` |
| BA-S2.6 | `cargo check --workspace` 通过 | — |

**产出**：所有环境变量、路径、配置统一为 grid 命名。

### Step 3: 创建 grid-cli-common crate

**目标**：抽取共享层。

| 任务 | 说明 | 源 → 目标 |
|------|------|----------|
| BA-S3.1 | 创建 `crates/grid-cli-common/` 目录和 Cargo.toml | 新建 |
| BA-S3.2 | 搬迁 `state.rs` + `types.rs` | `grid-cli/commands/` → `grid-cli-common/src/` |
| BA-S3.3 | 搬迁 `output/` 目录 | `grid-cli/output/` → `grid-cli-common/src/output/` |
| BA-S3.4 | 搬迁 `ui/` 目录 | `grid-cli/ui/` → `grid-cli-common/src/ui/` |
| BA-S3.5 | 搬迁共享 commands (10 个) | `grid-cli/commands/{agent,session,memory,mcp,tools,config,auth,skill,eval_cmd}.rs` → `grid-cli-common/src/commands/` |
| BA-S3.6 | 更新 grid-cli 的 `lib.rs` 和 `Cargo.toml` | 依赖 grid-cli-common, re-export |
| BA-S3.7 | 修正所有 `use crate::` 路径 | grid-cli-common 内部 |
| BA-S3.8 | `cargo check --workspace` 通过 | — |

**产出**：grid-cli-common 独立编译通过，grid-cli 通过 re-export 保持原有 API。

### Step 4: 创建 grid-studio crate

**目标**：从 grid-cli 中分离 TUI + Dashboard。

| 任务 | 说明 | 源 → 目标 |
|------|------|----------|
| BA-S4.1 | 创建 `crates/grid-studio/` 目录和 Cargo.toml | 新建 |
| BA-S4.2 | 搬迁 `tui/` 目录（47 文件整体） | `grid-cli/tui/` → `grid-studio/src/tui/` |
| BA-S4.3 | 搬迁 dashboard 文件 | `grid-cli/commands/dashboard*.rs` + `grid-cli/dashboard/` → `grid-studio/src/dashboard/` |
| BA-S4.4 | 创建 `grid-studio/src/main.rs` | TUI + Dashboard 入口 |
| BA-S4.5 | 创建 `grid-studio/src/lib.rs` | 暴露 `dashboard::build_router` 给 grid-desktop |
| BA-S4.6 | 修正所有 `use crate::` 路径 | grid-studio 内部 |
| BA-S4.7 | 更新 grid-desktop 依赖 | `octo-cli` → `grid-studio` |
| BA-S4.8 | 从 grid-cli 的 main.rs 中移除 Tui/Dashboard 命令 | grid-cli/src/main.rs |
| BA-S4.9 | 从 grid-cli 的 Cargo.toml 中移除 ratatui/crossterm/axum/tower-http | grid-cli/Cargo.toml |
| BA-S4.10 | `cargo check --workspace` 通过 | — |
| BA-S4.11 | `cargo test --workspace -- --test-threads=1` 通过 | — |

**产出**：grid-studio 独立编译，grid-cli 不再依赖 TUI 框架。

### Step 5: 品牌视觉替换

**目标**：Octo → Grid 品牌替换。

| 任务 | 说明 | 文件 |
|------|------|------|
| BA-S5.1 | CLI about/version 文字 | grid-cli/src/lib.rs |
| BA-S5.2 | TUI Welcome ASCII Art: "GRID" 圆角线条 | grid-studio/src/tui/widgets/welcome_panel/ |
| BA-S5.3 | 状态栏: "🦑 Octo" → "◆ Grid" | grid-studio/src/tui/widgets/status_bar.rs |
| BA-S5.4 | 默认主题: Cyan → Indigo #5E6AD2 | grid-cli-common/src/ui/theme.rs |
| BA-S5.5 | 日志前缀: "octo" → "grid" | 各 crate 的 tracing 配置 |
| BA-S5.6 | REPL 提示符更新 | grid-cli/src/repl/ |

**产出**：所有用户可见的品牌元素统一为 Grid。

### Step 6: 清理 + 验证

| 任务 | 说明 |
|------|------|
| BA-S6.1 | 全局搜索残留 "octo"（排除 git history） |
| BA-S6.2 | `cargo check --workspace` 通过 |
| BA-S6.3 | `cargo test --workspace -- --test-threads=1` 通过 |
| BA-S6.4 | `cargo clippy --workspace` 无 warning |
| BA-S6.5 | 更新 CLAUDE.md 中的 crate 引用 |
| BA-S6.6 | 更新 Makefile 中的 binary 名 |
| BA-S6.7 | 提交最终 commit |

---

## 执行顺序和依赖关系

```
S1 (crate rename) ──► S2 (env/path rename) ──► S3 (创建 common)
                                                     │
                                              S4 (创建 studio) ──► S5 (品牌) ──► S6 (清理)
```

- S1 → S2：必须先重命名 crate，再改环境变量（否则 use 路径混乱）
- S2 → S3：环境变量改完后再抽共享层（避免改两次）
- S3 → S4：共享层就绪后才能拆 studio
- S4 → S5：结构稳定后再改品牌（避免反复）
- S5 → S6：最后统一验证

---

## 风险检查点

| 检查点 | 位置 | 条件 |
|--------|------|------|
| **CP1** | S1 完成后 | `cargo test --workspace` 全通过 |
| **CP2** | S3 完成后 | grid-cli-common 独立编译，grid-cli re-export 正常 |
| **CP3** | S4 完成后 | grid-cli 和 grid-studio 都独立编译通过，grid-desktop 正常 |
| **CP4** | S6 完成后 | 全量测试通过，无 "octo" 残留 |

---

## 预计工作量

| Step | 预计改动文件数 | 复杂度 |
|------|--------------|--------|
| S1 | ~200+ | 高（全局替换，但机械化） |
| S2 | ~30 | 中 |
| S3 | ~20 | 中（搬迁 + 路径修正） |
| S4 | ~50 | 高（搬迁 + 新 main.rs） |
| S5 | ~10 | 低 |
| S6 | ~10 | 低 |

---

## Deferred Items

无。新项目一步到位，不留延期项。

---

## Notes

- 本计划基于 GRID_CRATE_SPLIT_DESIGN.md 的架构设计
- 执行时每个 Step 完成后提交一次 commit
- 如果 S1 的全局替换导致意外编译错误，逐 crate 修复
