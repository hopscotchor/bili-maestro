pub mod http;
pub mod ws;

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};

use crate::command::types::{Command, DanmakuMessage};
use crate::command::CommandQueue;

#[derive(Clone)]
pub struct AppState {
    pub command_queue: CommandQueue,
    pub danmaku_tx: broadcast::Sender<DanmakuMessage>,
    pub command_tx: broadcast::Sender<Command>,
    pub recent_danmaku: Arc<RwLock<VecDeque<DanmakuMessage>>>,
    pub room_id: u64,
    pub connected: Arc<RwLock<bool>>,
    pub popularity: Arc<RwLock<u32>>,
}

impl AppState {
    pub fn new(room_id: u64) -> Self {
        let (danmaku_tx, _) = broadcast::channel(256);
        let (command_tx, _) = broadcast::channel(64);
        Self {
            command_queue: CommandQueue::new(),
            danmaku_tx,
            command_tx,
            recent_danmaku: Arc::new(RwLock::new(VecDeque::with_capacity(200))),
            room_id,
            connected: Arc::new(RwLock::new(false)),
            popularity: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn add_danmaku(&self, msg: DanmakuMessage) {
        let _ = self.danmaku_tx.send(msg.clone());
        let mut recent = self.recent_danmaku.write().await;
        recent.push_back(msg);
        if recent.len() > 200 {
            recent.pop_front();
        }
    }

    pub async fn add_command(&self, cmd: Command) {
        let _ = self.command_tx.send(cmd.clone());
        self.command_queue.push(cmd).await;
    }
}
