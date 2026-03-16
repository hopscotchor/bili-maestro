pub mod types;

use std::collections::VecDeque;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::CommandConfig;
use types::{Command, CommandType, DanmakuMessage};

pub struct CommandParser {
    config: CommandConfig,
}

impl CommandParser {
    pub fn new(config: CommandConfig) -> Self {
        Self { config }
    }

    pub fn parse(&self, msg: &DanmakuMessage) -> Option<Command> {
        let text = msg.text.trim();

        if let Some(song_name) = text.strip_prefix(&self.config.song_prefix) {
            let song_name = song_name.trim();
            if !song_name.is_empty() {
                return Some(Command::new(
                    msg,
                    CommandType::SongRequest(song_name.to_string()),
                ));
            }
        }

        if text == self.config.skip_keyword {
            return Some(Command::new(msg, CommandType::SkipSong));
        }

        for keyword in &self.config.next_keywords {
            if text == keyword {
                return Some(Command::new(msg, CommandType::NextVideo));
            }
        }

        None
    }
}

#[derive(Clone)]
pub struct CommandQueue {
    inner: Arc<RwLock<VecDeque<Command>>>,
}

impl CommandQueue {
    pub fn new() -> Self {
        Self {
            inner: Arc::new(RwLock::new(VecDeque::new())),
        }
    }

    pub async fn push(&self, cmd: Command) {
        self.inner.write().await.push_back(cmd);
    }

    pub async fn list(&self, type_filter: Option<&str>) -> Vec<Command> {
        let queue = self.inner.read().await;
        queue
            .iter()
            .filter(|cmd| {
                if let Some(filter) = type_filter {
                    match (&cmd.command_type, filter) {
                        (CommandType::SongRequest(_), "song") => true,
                        (CommandType::SkipSong, "skip") => true,
                        (CommandType::NextVideo, "next") => true,
                        _ => false,
                    }
                } else {
                    true
                }
            })
            .cloned()
            .collect()
    }

    pub async fn ack(&self, id: Uuid) -> bool {
        let mut queue = self.inner.write().await;
        if let Some(pos) = queue.iter().position(|cmd| cmd.id == id) {
            queue.remove(pos);
            true
        } else {
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn make_msg(text: &str) -> DanmakuMessage {
        DanmakuMessage {
            uid: 12345,
            username: "testuser".to_string(),
            text: text.to_string(),
            timestamp: Utc::now(),
        }
    }

    #[test]
    fn test_parse_song_request() {
        let parser = CommandParser::new(CommandConfig::default());
        let msg = make_msg("点歌 晴天");
        let cmd = parser.parse(&msg).unwrap();
        match cmd.command_type {
            CommandType::SongRequest(name) => assert_eq!(name, "晴天"),
            _ => panic!("expected SongRequest"),
        }
    }

    #[test]
    fn test_parse_song_request_empty() {
        let parser = CommandParser::new(CommandConfig::default());
        let msg = make_msg("点歌 ");
        assert!(parser.parse(&msg).is_none());
    }

    #[test]
    fn test_parse_skip_song() {
        let parser = CommandParser::new(CommandConfig::default());
        let msg = make_msg("切歌");
        let cmd = parser.parse(&msg).unwrap();
        assert!(matches!(cmd.command_type, CommandType::SkipSong));
    }

    #[test]
    fn test_parse_next_video() {
        let parser = CommandParser::new(CommandConfig::default());

        let msg = make_msg("切视频");
        let cmd = parser.parse(&msg).unwrap();
        assert!(matches!(cmd.command_type, CommandType::NextVideo));

        let msg = make_msg("下一个");
        let cmd = parser.parse(&msg).unwrap();
        assert!(matches!(cmd.command_type, CommandType::NextVideo));
    }

    #[test]
    fn test_parse_no_command() {
        let parser = CommandParser::new(CommandConfig::default());
        let msg = make_msg("hello world");
        assert!(parser.parse(&msg).is_none());
    }

    #[tokio::test]
    async fn test_command_queue() {
        let queue = CommandQueue::new();
        let msg = make_msg("点歌 晴天");
        let parser = CommandParser::new(CommandConfig::default());
        let cmd = parser.parse(&msg).unwrap();
        let cmd_id = cmd.id;

        queue.push(cmd).await;
        assert_eq!(queue.list(None).await.len(), 1);
        assert_eq!(queue.list(Some("song")).await.len(), 1);
        assert_eq!(queue.list(Some("skip")).await.len(), 0);

        assert!(queue.ack(cmd_id).await);
        assert_eq!(queue.list(None).await.len(), 0);
    }
}
