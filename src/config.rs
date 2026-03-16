use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub room_id: u64,
    #[serde(default = "default_api_port")]
    pub api_port: u16,
    #[serde(default)]
    pub commands: CommandConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CommandConfig {
    #[serde(default = "default_song_prefix")]
    pub song_prefix: String,
    #[serde(default = "default_skip_keyword")]
    pub skip_keyword: String,
    #[serde(default = "default_next_keywords")]
    pub next_keywords: Vec<String>,
}

impl Default for CommandConfig {
    fn default() -> Self {
        Self {
            song_prefix: default_song_prefix(),
            skip_keyword: default_skip_keyword(),
            next_keywords: default_next_keywords(),
        }
    }
}

fn default_api_port() -> u16 {
    8080
}

fn default_song_prefix() -> String {
    "点歌 ".to_string()
}

fn default_skip_keyword() -> String {
    "切歌".to_string()
}

fn default_next_keywords() -> Vec<String> {
    vec!["切视频".to_string(), "下一个".to_string()]
}

impl Config {
    pub fn from_file(path: &Path) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&content)?;
        Ok(config)
    }
}
