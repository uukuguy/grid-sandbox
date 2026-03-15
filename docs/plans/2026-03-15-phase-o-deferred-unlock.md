# Phase O: Deferred 暂缓项全解锁 Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** 解决全部 3 个阻塞因素（TUI Input Widget / ProviderChain FailoverTrace / Session Event 广播），解锁 M-a、M-b、N 三个阶段共 10 个暂缓项。

**Architecture:** 分 4 个 Group 按依赖顺序执行：
- G1 (TUI Input Widget) 和 G2 (FailoverTrace) 可并行
- G3 (Session Event) 依赖 G1 完成
- G4 (Workbench 收尾) 在 G1+G2+G3 全部完成后执行

**Tech Stack:** Ratatui 0.29, crossterm, tokio broadcast channel, VecDeque

**Prerequisite:** Phase N complete (TUI dual-view + Dev-Agent panel)

**Deferred Items Resolved:**

| Source | ID | Content | Blocker |
|--------|----|---------|---------|
| M-a | D1 | TUI 双视图 + Eval 面板 | Phase M-b (already done) |
| M-a | D2 | Agent 调试面板 + Inspector | Phase N (already done) |
| M-a | D3 | watch 实时 TUI 进度条 | TUI framework → G1 |
| M-b | D1 | Eval shortcut dialogs | TUI input widget → G1 |
| M-b | D2 | Eval filter popup | TUI input widget → G1 |
| M-b | D3 | Dev-Agent panel | Phase N (already done) |
| N | D1 | Session 实时数据流 | WS 集成 → G3 |
| N | D2 | Memory 搜索交互 | TUI input widget → G1 |
| N | D3 | Provider failover 链路可视化 | ProviderChain API → G2 |
| N | D4 | 完整 Workbench 模式 | Phase N + 集成 → G4 |

---

## Group 1: TUI Input Widget 抽取 (O-T1 ~ O-T6)

> 解锁: M-a D3, M-b D1, M-b D2, N D2

### Task 1: TextInput 组件抽取

**Files:**
- Create: `crates/octo-cli/src/tui/widgets/mod.rs`
- Create: `crates/octo-cli/src/tui/widgets/text_input.rs`
- Modify: `crates/octo-cli/src/tui/mod.rs` (add `pub mod widgets;`)

**Step 1: Create widgets module**

```rust
// tui/widgets/mod.rs
pub mod text_input;
pub use text_input::TextInput;
```

**Step 2: Implement TextInput**

```rust
// tui/widgets/text_input.rs
use ratatui::prelude::*;
use ratatui::widgets::{Block, Paragraph};
use crossterm::event::KeyCode;
use crate::tui::theme::TuiTheme;

pub struct TextInput {
    input: String,
    cursor: usize,
    placeholder: String,
    active: bool,
}

impl TextInput {
    pub fn new(placeholder: impl Into<String>) -> Self {
        Self {
            input: String::new(),
            cursor: 0,
            placeholder: placeholder.into(),
            active: false,
        }
    }

    pub fn value(&self) -> &str { &self.input }
    pub fn is_empty(&self) -> bool { self.input.is_empty() }
    pub fn is_active(&self) -> bool { self.active }
    pub fn activate(&mut self) { self.active = true; }
    pub fn deactivate(&mut self) { self.active = false; }

    pub fn clear(&mut self) {
        self.input.clear();
        self.cursor = 0;
    }

    pub fn set_value(&mut self, value: impl Into<String>) {
        self.input = value.into();
        self.cursor = self.input.len();
    }

    /// Handle key input. Returns true if key was consumed.
    pub fn handle_key(&mut self, key: KeyCode) -> bool {
        if !self.active { return false; }
        match key {
            KeyCode::Char(c) => {
                self.input.insert(self.cursor, c);
                self.cursor += c.len_utf8();
                true
            }
            KeyCode::Backspace => {
                if self.cursor > 0 {
                    // Find prev char boundary
                    let prev = self.input[..self.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                    self.input.remove(prev);
                    self.cursor = prev;
                }
                true
            }
            KeyCode::Left => {
                if self.cursor > 0 {
                    self.cursor = self.input[..self.cursor]
                        .char_indices()
                        .last()
                        .map(|(i, _)| i)
                        .unwrap_or(0);
                }
                true
            }
            KeyCode::Right => {
                if self.cursor < self.input.len() {
                    self.cursor += self.input[self.cursor..]
                        .chars()
                        .next()
                        .map(|c| c.len_utf8())
                        .unwrap_or(0);
                }
                true
            }
            KeyCode::Home => { self.cursor = 0; true }
            KeyCode::End => { self.cursor = self.input.len(); true }
            KeyCode::Esc => { self.clear(); self.deactivate(); true }
            _ => false,
        }
    }

    /// Render the input widget in the given area.
    pub fn render(&self, frame: &mut Frame, area: Rect, theme: &TuiTheme, block: Option<Block>) {
        let display = if self.input.is_empty() && !self.active {
            Span::styled(&self.placeholder, theme.dim_style())
        } else {
            Span::styled(&self.input, theme.text_style())
        };

        let mut paragraph = Paragraph::new(Line::from(display));
        if let Some(b) = block {
            paragraph = paragraph.block(b);
        }
        frame.render_widget(paragraph, area);

        if self.active {
            // Calculate cursor X position (account for block border)
            let inner = if area.width > 2 { area.x + 1 } else { area.x };
            let cursor_x = inner + self.input[..self.cursor].chars().count() as u16;
            let cursor_y = area.y + 1; // account for border
            if cursor_x < area.x + area.width - 1 {
                frame.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}
```

**Step 3: Add unit tests**

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_defaults() { ... }
    #[test]
    fn test_char_insert() { ... }
    #[test]
    fn test_backspace() { ... }
    #[test]
    fn test_cursor_movement() { ... }
    #[test]
    fn test_home_end() { ... }
    #[test]
    fn test_esc_clears() { ... }
    #[test]
    fn test_inactive_ignores_keys() { ... }
    #[test]
    fn test_unicode_handling() { ... }
}
```

**Commit:** `feat(tui): extract reusable TextInput widget from ChatScreen`

---

### Task 2: ChatScreen 重构使用 TextInput

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/chat.rs`

**Step 1:** Replace `input: String, cursor: usize` fields with `input: TextInput`
**Step 2:** Update `handle_event()` to delegate to `self.input.handle_key()`
**Step 3:** Update `render()` to call `self.input.render()`
**Step 4:** Verify existing ChatScreen tests pass unchanged

**Commit:** `refactor(tui): ChatScreen uses shared TextInput widget`

---

### Task 3: DevEvalScreen shortcut dialogs

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/dev_eval.rs`

**Step 1:** Add `search_input: TextInput` and `command_input: TextInput` fields
**Step 2:** Add `InputMode` enum: `Normal`, `Search`, `RunSuite`, `DiffInput`
**Step 3:** Handle hotkeys:
  - `/` → activate search_input (filter runs by text)
  - `r` → activate command_input with "Run suite: " prompt
  - `d` → activate command_input with "Diff: " prompt (expects 2 run IDs)
**Step 4:** When input active, render input bar at bottom of panel; Esc returns to Normal
**Step 5:** Add tests for mode transitions

**Commit:** `feat(tui): Eval shortcut dialogs with TextInput`

---

### Task 4: DevEvalScreen filter popup

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/dev_eval.rs`

**Step 1:** Add `filter_input: TextInput` and `FilterTarget` enum: `Suite`, `Date`, `Tag`
**Step 2:** `f` key → cycle through filter targets, show filter bar
**Step 3:** Apply filter to runs list (case-insensitive substring match)
**Step 4:** Active filter shown as chip: `[suite:tool_call]` in header
**Step 5:** Add tests for filter application

**Commit:** `feat(tui): Eval filter popup for suite/date/tag`

---

### Task 5: DevAgentScreen Memory 搜索

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/dev_agent.rs`

**Step 1:** Add `memory_search: TextInput` field
**Step 2:** When Inspector focus + Memory panel, `/` activates search
**Step 3:** Filter memory entries by search text
**Step 4:** Add tests

**Commit:** `feat(tui): Memory panel search with TextInput`

---

### Task 6: Eval watch 实时进度条

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/dev_eval.rs`

**Step 1:** Add `watch_progress: Option<(usize, usize)>` (completed, total)
**Step 2:** When watch active, render `Gauge` widget showing completion %
**Step 3:** Tick event updates progress from AppState
**Step 4:** Add tests

**Commit:** `feat(tui): Eval watch progress bar with Gauge`

---

## Group 2: ProviderChain Failover Trace (O-T7 ~ O-T9)

> 解锁: N D3

### Task 7: FailoverTrace 数据结构

**Files:**
- Modify: `crates/octo-engine/src/providers/chain.rs`

**Step 1:** Add data structures:

```rust
use std::collections::VecDeque;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub struct FailoverAttempt {
    pub instance_id: String,
    pub attempted_at: DateTime<Utc>,
    pub duration_ms: u64,
    pub result: AttemptResult,
}

#[derive(Debug, Clone)]
pub enum AttemptResult {
    Success,
    Failed(String),
    RateLimited,
    Timeout,
}

#[derive(Debug, Clone)]
pub struct FailoverTrace {
    pub request_id: String,
    pub started_at: DateTime<Utc>,
    pub attempts: Vec<FailoverAttempt>,
    pub total_duration_ms: u64,
}
```

**Step 2:** Add `recent_traces: Arc<RwLock<VecDeque<FailoverTrace>>>` to ProviderChain (capacity 100)
**Step 3:** Add `pub fn recent_traces(&self) -> Vec<FailoverTrace>` method
**Step 4:** Add tests for trace storage and capacity

**Commit:** `feat(providers): FailoverTrace data structures + circular buffer`

---

### Task 8: ChainProvider 记录 Trace

**Files:**
- Modify: `crates/octo-engine/src/providers/chain.rs`

**Step 1:** In `ChainProvider::complete()`, before retry loop:
  - Generate `request_id` (uuid or counter)
  - Record `started_at`
**Step 2:** In each retry iteration:
  - Record attempt start time
  - On success/failure, create `FailoverAttempt`
**Step 3:** After loop (success or exhaustion), push `FailoverTrace` to buffer
**Step 4:** Same for `ChainProvider::stream()`
**Step 5:** Add integration tests

**Commit:** `feat(providers): record failover traces in ChainProvider`

---

### Task 9: Provider Inspector 链路可视化

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/dev_agent.rs`

**Step 1:** In Provider Inspector panel, add section: "Recent Failover"
**Step 2:** Render each trace as timeline:
  ```
  req-001 [14:30:05] 150ms
    ├─ claude-opus  ✓ 150ms
  req-002 [14:30:10] 5120ms
    ├─ claude-opus  ✗ rate_limited 5000ms
    └─ claude-sonnet ✓ 120ms
  ```
**Step 3:** Color code: green for success, red for failure, yellow for rate-limited
**Step 4:** Add tests for trace rendering

**Commit:** `feat(tui): Provider Inspector failover chain visualization`

---

## Group 3: Session Event 广播 (O-T10 ~ O-T13)

> 解锁: N D1

### Task 10: SessionEvent 枚举

**Files:**
- Create: `crates/octo-engine/src/session/events.rs`
- Modify: `crates/octo-engine/src/session/mod.rs`

**Step 1:** Define SessionEvent:

```rust
use chrono::{DateTime, Utc};

#[derive(Debug, Clone)]
pub enum SessionEvent {
    Created { session_id: String, agent_id: Option<String>, at: DateTime<Utc> },
    MessageAdded { session_id: String, role: String, at: DateTime<Utc> },
    ContextUpdated { session_id: String, token_count: usize, at: DateTime<Utc> },
    Closed { session_id: String, at: DateTime<Utc> },
}
```

**Step 2:** Add `session_events_tx: broadcast::Sender<SessionEvent>` to session module
**Step 3:** Add tests

**Commit:** `feat(session): SessionEvent enum for real-time notifications`

---

### Task 11: SessionStore 广播集成

**Files:**
- Modify: `crates/octo-engine/src/session/store.rs` (or equivalent)

**Step 1:** Add broadcast sender to SessionStore implementations
**Step 2:** After `create_session()` → emit `SessionEvent::Created`
**Step 3:** After `push_message()` → emit `SessionEvent::MessageAdded`
**Step 4:** After context updates → emit `SessionEvent::ContextUpdated`
**Step 5:** Add tests verifying event emission

**Commit:** `feat(session): SessionStore emits events on mutations`

---

### Task 12: WebSocket 推送 SessionEvent

**Files:**
- Modify: `crates/octo-server/src/ws.rs`

**Step 1:** Add `SessionUpdate` variant to `ServerMessage` enum:
```rust
SessionUpdate {
    event_type: String,  // "created" | "message_added" | "context_updated" | "closed"
    session_id: String,
    details: serde_json::Value,
}
```
**Step 2:** In WS handler, subscribe to session event broadcast channel
**Step 3:** Bridge SessionEvent → ServerMessage::SessionUpdate
**Step 4:** Add tests

**Commit:** `feat(ws): push SessionEvent updates to WebSocket clients`

---

### Task 13: DevAgentScreen 事件驱动刷新

**Files:**
- Modify: `crates/octo-cli/src/tui/screens/dev_agent.rs`

**Step 1:** Add `session_events: Vec<SessionEvent>` buffer (populated from AppState or channel)
**Step 2:** On Tick event, drain session events and update session list
**Step 3:** New session → auto-add to list; message added → update count
**Step 4:** Add tests

**Commit:** `feat(tui): DevAgent session list with event-driven refresh`

---

## Group 4: Workbench 模式收尾 (O-T14 ~ O-T15)

> 解锁: N D4

### Task 14: Workbench 模式对标 §6.9.2

**Files:**
- Modify: `crates/octo-cli/src/tui/mod.rs`
- Read: `docs/design/AGENT_CLI_DESIGN.md` §6.9.2

**Step 1:** Read §6.9.2, list all required features
**Step 2:** Compare with current implementation, identify remaining gaps
**Step 3:** Implement missing keybindings and panel interactions
**Step 4:** Add tests

**Commit:** `feat(tui): complete Workbench mode per AGENT_CLI_DESIGN.md §6.9.2`

---

### Task 15: Deferred 状态更新 + 收尾

**Files:**
- Modify: `docs/plans/2026-03-15-phase-m-eval-cli.md`
- Modify: `docs/plans/2026-03-15-phase-mb-tui-dual-view.md`
- Modify: `docs/plans/2026-03-15-phase-n-agent-debug.md`
- Modify: `docs/plans/.checkpoint.json`

**Step 1:** Update all 10 deferred items to `✅ 已补`
**Step 2:** Update checkpoint to Phase O COMPLETE
**Step 3:** Update MEMORY.md with Phase O status

**Commit:** `docs: Phase O complete — all deferred items resolved`

---

## Deferred

| ID | 内容 | 前置条件 | 状态 |
|----|------|---------|------|
| (none) | — | — | — |

Phase O 目标是清零所有暂缓项，不产生新暂缓项。
如实施中发现新 gap，立即记录。
