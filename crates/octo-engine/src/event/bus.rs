use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::metrics::MetricsRegistry;

/// octo-engine 内部事件（参考 ARCHITECTURE_DESIGN.md §Phase 2.4）
#[derive(Debug, Clone)]
pub enum OctoEvent {
    /// Agent Loop 开始新一轮
    LoopTurnStarted { session_id: String, turn: u32 },
    /// 工具调用开始
    ToolCallStarted {
        session_id: String,
        tool_name: String,
    },
    /// 工具调用完成
    ToolCallCompleted {
        session_id: String,
        tool_name: String,
        duration_ms: u64,
    },
    /// 上下文降级触发
    ContextDegraded { session_id: String, level: String },
    /// Loop Guard 触发
    LoopGuardTriggered { session_id: String, reason: String },
    /// Token 预算快照
    TokenBudgetUpdated {
        session_id: String,
        used: u64,
        total: u64,
        ratio: f64,
    },
}

/// 内部事件广播总线
///
/// 设计：broadcast::Sender（1000 容量）+ 环形缓冲区历史（最近 1000 条）
/// 参考：OpenFang openfang-kernel/src/event/bus.rs
pub struct EventBus {
    sender: broadcast::Sender<OctoEvent>,
    history: Arc<RwLock<VecDeque<OctoEvent>>>,
    history_capacity: usize,
    metrics: Arc<MetricsRegistry>,
}

impl EventBus {
    pub fn new(
        channel_capacity: usize,
        history_capacity: usize,
        metrics: Arc<MetricsRegistry>,
    ) -> Self {
        let (sender, _) = broadcast::channel(channel_capacity);
        Self {
            sender,
            history: Arc::new(RwLock::new(VecDeque::with_capacity(history_capacity))),
            history_capacity,
            metrics,
        }
    }

    /// 发布事件（fire-and-forget，不阻塞发送方）
    pub async fn publish(&self, event: OctoEvent) {
        // 记录指标
        self.record_metrics(&event);

        // 存入历史环形缓冲区
        {
            let mut history = self.history.write().await;
            if history.len() >= self.history_capacity {
                history.pop_front();
            }
            history.push_back(event.clone());
        }
        // 广播给订阅者（忽略无订阅者的错误）
        let _ = self.sender.send(event);
    }

    /// 根据事件类型记录指标
    fn record_metrics(&self, event: &OctoEvent) {
        match event {
            OctoEvent::ToolCallCompleted {
                tool_name: _,
                duration_ms,
                ..
            } => {
                self.metrics.counter("octo.tools.executions.total").inc();
                self.metrics
                    .histogram(
                        "octo.tools.executions.duration_ms",
                        vec![
                            10.0, 50.0, 100.0, 250.0, 500.0, 1000.0, 2500.0, 5000.0, 10000.0,
                        ],
                    )
                    .observe(*duration_ms as f64);
            }
            OctoEvent::LoopTurnStarted { turn, .. } => {
                self.metrics.counter("octo.sessions.turns.total").inc();
                self.metrics
                    .histogram(
                        "octo.sessions.turns.number",
                        vec![1.0, 5.0, 10.0, 20.0, 50.0, 100.0],
                    )
                    .observe(*turn as f64);
            }
            OctoEvent::ToolCallStarted { tool_name: _, .. } => {
                self.metrics.counter("octo.tools.calls.started.total").inc();
            }
            OctoEvent::ContextDegraded { level: _, .. } => {
                self.metrics
                    .counter("octo.context.degradations.total")
                    .inc();
            }
            OctoEvent::LoopGuardTriggered { reason: _, .. } => {
                self.metrics
                    .counter("octo.sessions.guards.triggered.total")
                    .inc();
            }
            OctoEvent::TokenBudgetUpdated {
                used, total, ratio, ..
            } => {
                self.metrics
                    .gauge("octo.context.tokens.used")
                    .set(*used as i64);
                self.metrics
                    .gauge("octo.context.tokens.total")
                    .set(*total as i64);
                // ratio is f64, but gauge only supports i64, so we store it as basis points (x10000)
                self.metrics
                    .gauge("octo.context.tokens.ratio")
                    .set((ratio * 10000.0) as i64);
            }
        }
    }

    /// 订阅事件流（每个订阅者独立接收）
    pub fn subscribe(&self) -> broadcast::Receiver<OctoEvent> {
        self.sender.subscribe()
    }

    /// 获取最近 N 条历史事件
    pub async fn recent_events(&self, n: usize) -> Vec<OctoEvent> {
        let history = self.history.read().await;
        let collected: Vec<_> = history.iter().rev().take(n).cloned().collect();
        collected.into_iter().rev().collect()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new(1000, 1000, Arc::new(MetricsRegistry::new()))
    }
}
