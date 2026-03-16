use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DanmakuMessage {
    pub uid: u64,
    pub username: String,
    pub text: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "content")]
pub enum CommandType {
    SongRequest(String),
    SkipSong,
    NextVideo,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Command {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub user_uid: u64,
    pub username: String,
    pub command_type: CommandType,
    pub raw: String,
}

impl Command {
    pub fn new(msg: &DanmakuMessage, command_type: CommandType) -> Self {
        Self {
            id: Uuid::new_v4(),
            timestamp: msg.timestamp,
            user_uid: msg.uid,
            username: msg.username.clone(),
            command_type,
            raw: msg.text.clone(),
        }
    }
}
