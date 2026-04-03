use anyhow::{anyhow, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::info;

use grid_types::{CompletionRequest, CompletionResponse};

/// LLM 实例配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmInstance {
    pub id: String,
    pub provider: String,
    pub api_key: String,
    pub base_url: Option<String>,
    pub model: String,
    pub priority: u8,
    pub max_rpm: Option<u32>,
    pub enabled: bool,
}

/// 实例健康状态
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum InstanceHealth {
    Healthy,
    Unhealthy {
        reason: String,
        failed_at: DateTime<Utc>,
    },
    Unknown,
}

/// 故障切换策略
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum FailoverPolicy {
    #[default]
    Automatic,
    Manual,
    Hybrid,
}

/// 健康检查配置
#[derive(Debug, Clone)]
pub struct HealthCheckConfig {
    pub interval: Duration,
    pub timeout: Duration,
}

impl Default for HealthCheckConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
        }
    }
}

/// Result of a single failover attempt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AttemptResult {
    Success,
    Failed(String),
    NoInstance(String),
}

/// A single attempt to call an LLM instance
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverAttempt {
    pub instance_id: String,
    pub duration_ms: u64,
    pub result: AttemptResult,
}

/// Trace of a complete failover sequence for one request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FailoverTrace {
    pub request_id: u64,
    pub started_at: DateTime<Utc>,
    pub attempts: Vec<FailoverAttempt>,
    pub total_duration_ms: u64,
}

/// Per-instance statistics computed from recent traces
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstanceStats {
    pub latency_p50_ms: Option<u64>,
    pub latency_p99_ms: Option<u64>,
    pub request_count: u64,
    pub error_count: u64,
    pub failover_count: u64,
}

/// ProviderChain 管理多个 LLM 实例
pub struct ProviderChain {
    instances: Arc<RwLock<Vec<LlmInstance>>>,
    health: Arc<RwLock<HashMap<String, InstanceHealth>>>,
    policy: FailoverPolicy,
    manual_instance_id: Arc<RwLock<Option<String>>>,
    recent_traces: Arc<RwLock<VecDeque<FailoverTrace>>>,
    trace_counter: Arc<std::sync::atomic::AtomicU64>,
}

impl ProviderChain {
    /// 创建新的 ProviderChain
    pub fn new(policy: FailoverPolicy) -> Self {
        Self {
            instances: Arc::new(RwLock::new(Vec::new())),
            health: Arc::new(RwLock::new(HashMap::new())),
            policy,
            manual_instance_id: Arc::new(RwLock::new(None)),
            recent_traces: Arc::new(RwLock::new(VecDeque::with_capacity(100))),
            trace_counter: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        }
    }

    /// Get the failover policy
    pub fn policy(&self) -> FailoverPolicy {
        self.policy
    }

    /// 添加实例
    pub async fn add_instance(&self, instance: LlmInstance) {
        let mut instances = self.instances.write().await;
        instances.push(instance.clone());

        // 初始化健康状态
        let mut health = self.health.write().await;
        health.insert(instance.id, InstanceHealth::Unknown);
    }

    /// 移除实例
    pub async fn remove_instance(&self, id: &str) -> Result<()> {
        let mut instances = self.instances.write().await;
        let len_before = instances.len();
        instances.retain(|i| i.id != id);

        if instances.len() == len_before {
            return Err(anyhow!("Instance not found: {}", id));
        }

        let mut health = self.health.write().await;
        health.remove(id);

        // 如果移除的是手动选择的实例，清除选择
        let mut manual = self.manual_instance_id.write().await;
        if manual.as_deref() == Some(id) {
            *manual = None;
        }

        Ok(())
    }

    /// 列出所有实例
    pub async fn list_instances(&self) -> Vec<LlmInstance> {
        self.instances.read().await.clone()
    }

    /// 获取实例健康状态
    pub async fn get_health(&self, id: &str) -> InstanceHealth {
        let health = self.health.read().await;
        health.get(id).cloned().unwrap_or(InstanceHealth::Unknown)
    }

    /// 获取可用的实例
    pub async fn get_available(&self) -> Result<Arc<LlmInstance>> {
        // 1. 手动选择优先
        if let Some(id) = self.manual_instance_id.read().await.as_ref() {
            let instances = self.instances.read().await;
            if let Some(instance) = instances.iter().find(|i| &i.id == id) {
                if instance.enabled {
                    let health = self.health.read().await;
                    if matches!(
                        health.get(&instance.id),
                        Some(InstanceHealth::Healthy) | None | Some(InstanceHealth::Unknown)
                    ) {
                        return Ok(Arc::new(instance.clone()));
                    }
                }
            }
        }

        // 2. 自动模式
        match self.policy {
            FailoverPolicy::Manual => Err(anyhow!("No manual instance selected")),
            _ => self.get_next_healthy_instance().await,
        }
    }

    async fn get_next_healthy_instance(&self) -> Result<Arc<LlmInstance>> {
        let instances = self.instances.read().await;
        let health = self.health.read().await;

        let mut sorted: Vec<_> = instances.iter().filter(|i| i.enabled).collect();
        sorted.sort_by_key(|i| i.priority);

        for instance in sorted {
            let instance_health = health.get(&instance.id);
            if matches!(
                instance_health,
                Some(InstanceHealth::Healthy) | None | Some(InstanceHealth::Unknown)
            ) {
                return Ok(Arc::new(instance.clone()));
            }
        }

        Err(anyhow!("No healthy instances available"))
    }

    /// 标记实例不健康
    pub async fn mark_unhealthy(&self, instance_id: &str, reason: &str) {
        let mut health = self.health.write().await;
        health.insert(
            instance_id.to_string(),
            InstanceHealth::Unhealthy {
                reason: reason.to_string(),
                failed_at: Utc::now(),
            },
        );
    }

    /// 手动选择实例
    pub async fn select_instance(&self, instance_id: &str) -> Result<()> {
        let instances = self.instances.read().await;
        if !instances.iter().any(|i| i.id == instance_id) {
            return Err(anyhow!("Instance not found: {}", instance_id));
        }
        drop(instances);

        let mut manual = self.manual_instance_id.write().await;
        *manual = Some(instance_id.to_string());
        Ok(())
    }

    /// 清除手动选择
    pub async fn clear_selection(&self) {
        let mut manual = self.manual_instance_id.write().await;
        *manual = None;
    }

    /// 获取当前选择
    pub async fn get_current_selection(&self) -> Option<String> {
        self.manual_instance_id.read().await.clone()
    }

    /// 重置实例健康状态
    pub async fn reset_health(&self, instance_id: &str) -> Result<()> {
        let instances = self.instances.read().await;
        if !instances.iter().any(|i| i.id == instance_id) {
            return Err(anyhow!("Instance not found: {}", instance_id));
        }

        let mut health = self.health.write().await;
        health.insert(instance_id.to_string(), InstanceHealth::Healthy);
        Ok(())
    }

    /// Get all health statuses
    pub async fn get_all_health(&self) -> HashMap<String, InstanceHealth> {
        self.health.read().await.clone()
    }

    /// Compute per-instance stats from recent traces
    pub async fn instance_stats(&self) -> HashMap<String, InstanceStats> {
        let traces = self.recent_traces.read().await;
        let mut per_instance: HashMap<String, Vec<u64>> = HashMap::new();
        let mut error_counts: HashMap<String, u64> = HashMap::new();
        let mut request_counts: HashMap<String, u64> = HashMap::new();
        let mut failover_counts: HashMap<String, u64> = HashMap::new();

        for trace in traces.iter() {
            let is_failover = trace.attempts.len() > 1;
            for attempt in &trace.attempts {
                if attempt.instance_id == "none" {
                    continue;
                }
                *request_counts
                    .entry(attempt.instance_id.clone())
                    .or_default() += 1;
                match &attempt.result {
                    AttemptResult::Success => {
                        per_instance
                            .entry(attempt.instance_id.clone())
                            .or_default()
                            .push(attempt.duration_ms);
                    }
                    AttemptResult::Failed(_) => {
                        *error_counts
                            .entry(attempt.instance_id.clone())
                            .or_default() += 1;
                    }
                    AttemptResult::NoInstance(_) => {}
                }
                if is_failover {
                    *failover_counts
                        .entry(attempt.instance_id.clone())
                        .or_default() += 1;
                }
            }
        }

        let mut result = HashMap::new();
        // Collect all instance IDs from requests and latencies
        let all_ids: std::collections::HashSet<&String> = request_counts
            .keys()
            .chain(per_instance.keys())
            .collect();

        for id in all_ids {
            let mut durations = per_instance.get(id).cloned().unwrap_or_default();
            durations.sort_unstable();
            let (p50, p99) = if durations.is_empty() {
                (None, None)
            } else {
                let p50_idx = (durations.len() as f64 * 0.50).ceil() as usize;
                let p99_idx = (durations.len() as f64 * 0.99).ceil() as usize;
                (
                    Some(durations[p50_idx.saturating_sub(1).min(durations.len() - 1)]),
                    Some(durations[p99_idx.saturating_sub(1).min(durations.len() - 1)]),
                )
            };
            result.insert(
                id.clone(),
                InstanceStats {
                    latency_p50_ms: p50,
                    latency_p99_ms: p99,
                    request_count: *request_counts.get(id).unwrap_or(&0),
                    error_count: *error_counts.get(id).unwrap_or(&0),
                    failover_count: *failover_counts.get(id).unwrap_or(&0),
                },
            );
        }

        result
    }

    /// Get recent failover traces (last N, max 100)
    pub async fn recent_traces(&self, limit: usize) -> Vec<FailoverTrace> {
        let traces = self.recent_traces.read().await;
        traces.iter().rev().take(limit).cloned().collect()
    }

    /// Record a failover trace (internal)
    pub(crate) async fn record_trace(&self, trace: FailoverTrace) {
        let mut traces = self.recent_traces.write().await;
        if traces.len() >= 100 {
            traces.pop_front();
        }
        traces.push_back(trace);
    }

    /// Get next request ID (internal)
    pub(crate) fn next_request_id(&self) -> u64 {
        self.trace_counter
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    /// 启动健康检查任务
    pub async fn start_health_checker(&self, config: HealthCheckConfig) {
        let instances = Arc::clone(&self.instances);
        let health = Arc::clone(&self.health);

        tokio::spawn(async move {
            loop {
                sleep(config.interval).await;

                let instance_ids: Vec<String> = {
                    let instances = instances.read().await;
                    instances.iter().map(|i| i.id.clone()).collect()
                };

                for id in instance_ids {
                    // 只检查 Unknown 或 Unhealthy 的实例
                    let should_check = {
                        let h = health.read().await;
                        matches!(h.get(&id), Some(InstanceHealth::Unhealthy { .. }) | None)
                    };

                    if should_check {
                        // 简单健康检查：创建 provider 测试
                        if Self::check_instance(&id, &instances, &health, config.timeout).await {
                            let mut h = health.write().await;
                            h.insert(id.clone(), InstanceHealth::Healthy);
                            info!("Instance {} recovered to healthy", id);
                        }
                    }
                }
            }
        });
    }

    async fn check_instance(
        id: &str,
        instances: &Arc<RwLock<Vec<LlmInstance>>>,
        _health: &Arc<RwLock<HashMap<String, InstanceHealth>>>,
        timeout: Duration,
    ) -> bool {
        let instance = {
            let instances = instances.read().await;
            instances.iter().find(|i| i.id == id).cloned()
        };

        let Some(instance) = instance else {
            return false;
        };

        // 尝试创建 provider（不实际调用 API）
        // 如果能创建成功，认为实例可用
        let _provider = super::create_provider(
            &instance.provider,
            instance.api_key.clone(),
            instance.base_url.clone(),
        );

        // 可以在这里添加实际的 ping 调用
        // 目前简单返回 true
        let _ = timeout;
        true
    }
}

/// 包装 ProviderChain 为单一 Provider 接口
pub struct ChainProvider {
    chain: Arc<ProviderChain>,
    max_retries: u32,
}

impl ChainProvider {
    pub fn new(chain: Arc<ProviderChain>, max_retries: u32) -> Self {
        Self { chain, max_retries }
    }

    pub fn chain(&self) -> &Arc<ProviderChain> {
        &self.chain
    }
}

#[async_trait]
impl crate::providers::Provider for ChainProvider {
    fn id(&self) -> &str {
        "chain"
    }

    async fn complete(&self, request: CompletionRequest) -> Result<CompletionResponse> {
        let request_id = self.chain.next_request_id();
        let trace_start = std::time::Instant::now();
        let started_at = Utc::now();
        let mut attempts = Vec::new();
        let mut last_error = None;

        for _ in 0..self.max_retries {
            let attempt_start = std::time::Instant::now();

            let instance = match self.chain.get_available().await {
                Ok(i) => i,
                Err(e) => {
                    attempts.push(FailoverAttempt {
                        instance_id: "none".to_string(),
                        duration_ms: attempt_start.elapsed().as_millis() as u64,
                        result: AttemptResult::NoInstance(e.to_string()),
                    });
                    last_error = Some(e);
                    continue;
                }
            };

            let provider = crate::providers::create_provider(
                &instance.provider,
                instance.api_key.clone(),
                instance.base_url.clone(),
            );

            match provider.complete(request.clone()).await {
                Ok(response) => {
                    attempts.push(FailoverAttempt {
                        instance_id: instance.id.clone(),
                        duration_ms: attempt_start.elapsed().as_millis() as u64,
                        result: AttemptResult::Success,
                    });
                    let trace = FailoverTrace {
                        request_id,
                        started_at,
                        attempts,
                        total_duration_ms: trace_start.elapsed().as_millis() as u64,
                    };
                    self.chain.record_trace(trace).await;
                    return Ok(response);
                }
                Err(e) => {
                    attempts.push(FailoverAttempt {
                        instance_id: instance.id.clone(),
                        duration_ms: attempt_start.elapsed().as_millis() as u64,
                        result: AttemptResult::Failed(e.to_string()),
                    });
                    self.chain
                        .mark_unhealthy(&instance.id, &e.to_string())
                        .await;
                    last_error = Some(e);
                }
            }
        }

        // Record trace even on total failure
        let trace = FailoverTrace {
            request_id,
            started_at,
            attempts,
            total_duration_ms: trace_start.elapsed().as_millis() as u64,
        };
        self.chain.record_trace(trace).await;

        Err(last_error.unwrap_or_else(|| anyhow!("All instances failed")))
    }

    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<crate::providers::CompletionStream> {
        let request_id = self.chain.next_request_id();
        let trace_start = std::time::Instant::now();
        let started_at = Utc::now();
        let mut attempts = Vec::new();
        let mut last_error = None;

        for _ in 0..self.max_retries {
            let attempt_start = std::time::Instant::now();

            let instance = match self.chain.get_available().await {
                Ok(i) => i,
                Err(e) => {
                    attempts.push(FailoverAttempt {
                        instance_id: "none".to_string(),
                        duration_ms: attempt_start.elapsed().as_millis() as u64,
                        result: AttemptResult::NoInstance(e.to_string()),
                    });
                    last_error = Some(e);
                    continue;
                }
            };

            let provider = crate::providers::create_provider(
                &instance.provider,
                instance.api_key.clone(),
                instance.base_url.clone(),
            );

            match provider.stream(request.clone()).await {
                Ok(stream) => {
                    attempts.push(FailoverAttempt {
                        instance_id: instance.id.clone(),
                        duration_ms: attempt_start.elapsed().as_millis() as u64,
                        result: AttemptResult::Success,
                    });
                    let trace = FailoverTrace {
                        request_id,
                        started_at,
                        attempts,
                        total_duration_ms: trace_start.elapsed().as_millis() as u64,
                    };
                    self.chain.record_trace(trace).await;
                    return Ok(stream);
                }
                Err(e) => {
                    attempts.push(FailoverAttempt {
                        instance_id: instance.id.clone(),
                        duration_ms: attempt_start.elapsed().as_millis() as u64,
                        result: AttemptResult::Failed(e.to_string()),
                    });
                    self.chain
                        .mark_unhealthy(&instance.id, &e.to_string())
                        .await;
                    last_error = Some(e);
                }
            }
        }

        // Record trace even on total failure
        let trace = FailoverTrace {
            request_id,
            started_at,
            attempts,
            total_duration_ms: trace_start.elapsed().as_millis() as u64,
        };
        self.chain.record_trace(trace).await;

        Err(last_error.unwrap_or_else(|| anyhow!("All instances failed to stream")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_add_and_list_instances() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);

        chain
            .add_instance(LlmInstance {
                id: "test-1".to_string(),
                provider: "anthropic".to_string(),
                api_key: "test-key".to_string(),
                base_url: None,
                model: "claude-3-sonnet".to_string(),
                priority: 0,
                max_rpm: None,
                enabled: true,
            })
            .await;

        let instances = chain.list_instances().await;
        assert_eq!(instances.len(), 1);
        assert_eq!(instances[0].id, "test-1");
    }

    #[tokio::test]
    async fn test_get_available_auto_mode() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);

        chain
            .add_instance(LlmInstance {
                id: "test-1".to_string(),
                provider: "anthropic".to_string(),
                api_key: "test-key".to_string(),
                base_url: None,
                model: "claude-3-sonnet".to_string(),
                priority: 0,
                max_rpm: None,
                enabled: true,
            })
            .await;

        let instance = chain.get_available().await.unwrap();
        assert_eq!(instance.id, "test-1");
    }

    #[tokio::test]
    async fn test_manual_selection() {
        let chain = ProviderChain::new(FailoverPolicy::Hybrid);

        chain
            .add_instance(LlmInstance {
                id: "test-1".to_string(),
                provider: "anthropic".to_string(),
                api_key: "key-1".to_string(),
                base_url: None,
                model: "claude-3-sonnet".to_string(),
                priority: 0,
                max_rpm: None,
                enabled: true,
            })
            .await;

        chain
            .add_instance(LlmInstance {
                id: "test-2".to_string(),
                provider: "openai".to_string(),
                api_key: "key-2".to_string(),
                base_url: None,
                model: "gpt-4".to_string(),
                priority: 1,
                max_rpm: None,
                enabled: true,
            })
            .await;

        // 手动选择
        chain.select_instance("test-2").await.unwrap();

        let selected = chain.get_current_selection().await;
        assert_eq!(selected, Some("test-2".to_string()));

        let instance = chain.get_available().await.unwrap();
        assert_eq!(instance.id, "test-2");
    }

    #[tokio::test]
    async fn test_mark_unhealthy() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);

        chain
            .add_instance(LlmInstance {
                id: "test-1".to_string(),
                provider: "anthropic".to_string(),
                api_key: "test-key".to_string(),
                base_url: None,
                model: "claude-3-sonnet".to_string(),
                priority: 0,
                max_rpm: None,
                enabled: true,
            })
            .await;

        // 标记不健康
        chain.mark_unhealthy("test-1", "rate limit").await;

        let health = chain.get_health("test-1").await;
        assert!(matches!(health, InstanceHealth::Unhealthy { .. }));
    }

    #[tokio::test]
    async fn test_remove_instance() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);

        chain
            .add_instance(LlmInstance {
                id: "test-1".to_string(),
                provider: "anthropic".to_string(),
                api_key: "test-key".to_string(),
                base_url: None,
                model: "claude-3-sonnet".to_string(),
                priority: 0,
                max_rpm: None,
                enabled: true,
            })
            .await;

        chain.remove_instance("test-1").await.unwrap();

        let instances = chain.list_instances().await;
        assert!(instances.is_empty());
    }

    #[tokio::test]
    async fn test_recent_traces_empty() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);
        let traces = chain.recent_traces(10).await;
        assert!(traces.is_empty());
    }

    #[tokio::test]
    async fn test_record_and_retrieve_trace() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);
        let trace = FailoverTrace {
            request_id: 0,
            started_at: Utc::now(),
            attempts: vec![FailoverAttempt {
                instance_id: "test-1".to_string(),
                duration_ms: 100,
                result: AttemptResult::Success,
            }],
            total_duration_ms: 100,
        };
        chain.record_trace(trace).await;
        let traces = chain.recent_traces(10).await;
        assert_eq!(traces.len(), 1);
        assert_eq!(traces[0].request_id, 0);
    }

    #[tokio::test]
    async fn test_trace_buffer_capacity() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);
        for i in 0..110 {
            let trace = FailoverTrace {
                request_id: i,
                started_at: Utc::now(),
                attempts: vec![],
                total_duration_ms: 0,
            };
            chain.record_trace(trace).await;
        }
        let all = chain.recent_traces(200).await;
        assert_eq!(all.len(), 100); // capped at 100
        // Most recent should be first (iter().rev())
        assert_eq!(all[0].request_id, 109);
    }

    #[tokio::test]
    async fn test_attempt_result_variants() {
        let s = AttemptResult::Success;
        let f = AttemptResult::Failed("timeout".to_string());
        let n = AttemptResult::NoInstance("no healthy".to_string());
        // Verify they can be created and debug-printed
        assert!(format!("{:?}", s).contains("Success"));
        assert!(format!("{:?}", f).contains("timeout"));
        assert!(format!("{:?}", n).contains("no healthy"));
    }

    #[tokio::test]
    async fn test_next_request_id_increments() {
        let chain = ProviderChain::new(FailoverPolicy::Automatic);
        let id1 = chain.next_request_id();
        let id2 = chain.next_request_id();
        assert_eq!(id2, id1 + 1);
    }
}
