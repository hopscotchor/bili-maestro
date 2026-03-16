pub mod websocket;

use async_trait::async_trait;

use crate::command::types::DanmakuMessage;

#[async_trait]
pub trait DanmakuSource: Send {
    async fn connect(&mut self) -> anyhow::Result<()>;
    async fn next_message(&mut self) -> anyhow::Result<DanmakuMessage>;
}
