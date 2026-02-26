use anyhow::Result;
use async_trait::async_trait;
use octo_types::{MemoryBlock, SandboxId, UserId};

#[async_trait]
pub trait WorkingMemory: Send + Sync {
    async fn get_blocks(
        &self,
        user_id: &UserId,
        sandbox_id: &SandboxId,
    ) -> Result<Vec<MemoryBlock>>;

    async fn update_block(&self, block_id: &str, value: &str) -> Result<()>;

    async fn add_block(&self, block: MemoryBlock) -> Result<()>;

    async fn remove_block(&self, block_id: &str) -> Result<bool>;

    /// Expire blocks that have exceeded max_age_turns. Returns removed count.
    async fn expire_blocks(&self, current_turn: u32) -> Result<usize>;

    async fn compile(
        &self,
        user_id: &UserId,
        sandbox_id: &SandboxId,
    ) -> Result<String>;
}
