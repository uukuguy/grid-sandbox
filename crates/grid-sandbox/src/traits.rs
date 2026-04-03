use anyhow::Result;
use async_trait::async_trait;
use grid_types::{ExecResult, RuntimeType};

#[async_trait]
pub trait RuntimeAdapter: Send + Sync {
    fn runtime_type(&self) -> RuntimeType;
    async fn execute(&self, cmd: &str, working_dir: &str) -> Result<ExecResult>;
}
