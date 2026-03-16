use std::io::Read as _;

use async_trait::async_trait;
use chrono::Utc;
use flate2::read::ZlibDecoder;
use futures_util::{SinkExt, StreamExt};
use md5::{Digest, Md5};
use serde::Deserialize;
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};
use tracing::{debug, info, warn};

use super::DanmakuSource;
use crate::command::types::DanmakuMessage;

// --- Wbi signing ---

const MIXIN_KEY_ENC_TAB: [u8; 64] = [
    46, 47, 18, 2, 53, 8, 23, 32, 15, 50, 10, 31, 58, 3, 45, 35, 27, 43, 5, 49, 33, 9, 42, 19,
    29, 28, 14, 39, 12, 38, 41, 13, 37, 48, 7, 16, 24, 55, 40, 61, 26, 17, 0, 1, 60, 51, 30, 4,
    22, 25, 54, 21, 56, 59, 6, 63, 57, 62, 11, 36, 20, 34, 44, 52,
];

fn get_mixin_key(img_key: &str, sub_key: &str) -> String {
    let raw = format!("{}{}", img_key, sub_key);
    let binding = raw.as_bytes();
    MIXIN_KEY_ENC_TAB
        .iter()
        .take(32)
        .map(|&i| binding[i as usize] as char)
        .collect()
}

fn wbi_sign(params: &mut Vec<(String, String)>, mixin_key: &str) {
    let wts = Utc::now().timestamp().to_string();
    params.push(("wts".to_string(), wts));
    params.sort_by(|a, b| a.0.cmp(&b.0));

    let query: String = params
        .iter()
        .map(|(k, v)| {
            let v_clean: String = v.chars().filter(|c| !"!'()*".contains(*c)).collect();
            format!("{}={}", urlencoding::encode(k), urlencoding::encode(&v_clean))
        })
        .collect::<Vec<_>>()
        .join("&");

    let to_hash = format!("{}{}", query, mixin_key);
    let mut hasher = Md5::new();
    hasher.update(to_hash.as_bytes());
    let w_rid = format!("{:x}", hasher.finalize());
    params.push(("w_rid".to_string(), w_rid));
}

// --- Bilibili API responses ---

#[derive(Deserialize)]
struct ApiResponse<T> {
    code: i32,
    data: Option<T>,
}

#[derive(Deserialize)]
struct FingerSpiData {
    b_3: String,
}

#[derive(Deserialize)]
struct NavData {
    wbi_img: WbiImg,
}

#[derive(Deserialize)]
struct WbiImg {
    img_url: String,
    sub_url: String,
}

#[derive(Deserialize)]
struct DanmuInfoData {
    token: String,
    host_list: Vec<HostInfo>,
}

#[derive(Deserialize)]
struct HostInfo {
    host: String,
    wss_port: u16,
}

// --- Packet codec ---

const HEADER_SIZE: usize = 16;

#[derive(Debug, Clone, Copy, PartialEq)]
#[repr(u32)]
enum Operation {
    Heartbeat = 2,
    HeartbeatReply = 3,
    Message = 5,
    Auth = 7,
    AuthReply = 8,
}

impl Operation {
    fn from_u32(v: u32) -> Option<Self> {
        match v {
            2 => Some(Self::Heartbeat),
            3 => Some(Self::HeartbeatReply),
            5 => Some(Self::Message),
            7 => Some(Self::Auth),
            8 => Some(Self::AuthReply),
            _ => None,
        }
    }
}

struct Packet {
    version: u16,
    operation: Operation,
    body: Vec<u8>,
}

impl Packet {
    fn encode(&self) -> Vec<u8> {
        let total_len = (HEADER_SIZE + self.body.len()) as u32;
        let mut buf = Vec::with_capacity(total_len as usize);
        buf.extend_from_slice(&total_len.to_be_bytes());
        buf.extend_from_slice(&(HEADER_SIZE as u16).to_be_bytes());
        buf.extend_from_slice(&self.version.to_be_bytes());
        buf.extend_from_slice(&(self.operation as u32).to_be_bytes());
        buf.extend_from_slice(&1u32.to_be_bytes()); // sequence
        buf.extend_from_slice(&self.body);
        buf
    }

    fn decode(data: &[u8]) -> Vec<Packet> {
        let mut packets = Vec::new();
        let mut offset = 0;

        while offset + HEADER_SIZE <= data.len() {
            let total_len = u32::from_be_bytes([
                data[offset],
                data[offset + 1],
                data[offset + 2],
                data[offset + 3],
            ]) as usize;
            let header_len = u16::from_be_bytes([data[offset + 4], data[offset + 5]]) as usize;
            let version = u16::from_be_bytes([data[offset + 6], data[offset + 7]]);
            let op_code = u32::from_be_bytes([
                data[offset + 8],
                data[offset + 9],
                data[offset + 10],
                data[offset + 11],
            ]);

            if offset + total_len > data.len() {
                break;
            }

            let body = data[offset + header_len..offset + total_len].to_vec();

            if let Some(operation) = Operation::from_u32(op_code) {
                packets.push(Packet {
                    version,
                    operation,
                    body,
                });
            }

            offset += total_len;
        }

        packets
    }
}

fn decompress_body(version: u16, body: &[u8]) -> Vec<u8> {
    match version {
        2 => {
            let mut decoder = ZlibDecoder::new(body);
            let mut decompressed = Vec::new();
            if decoder.read_to_end(&mut decompressed).is_ok() {
                decompressed
            } else {
                warn!("zlib decompression failed");
                body.to_vec()
            }
        }
        3 => {
            let mut decompressed = Vec::new();
            if brotli::BrotliDecompress(&mut &body[..], &mut decompressed).is_ok() {
                decompressed
            } else {
                warn!("brotli decompression failed");
                body.to_vec()
            }
        }
        _ => body.to_vec(),
    }
}

fn parse_danmu_messages(data: &[u8]) -> Vec<DanmakuMessage> {
    let packets = Packet::decode(data);
    let mut messages = Vec::new();

    for packet in packets {
        if packet.operation != Operation::Message {
            continue;
        }

        let decompressed = decompress_body(packet.version, &packet.body);

        // Decompressed data may contain multiple sub-packets
        let sub_packets = if packet.version == 2 || packet.version == 3 {
            Packet::decode(&decompressed)
        } else {
            vec![packet]
        };

        for sub in sub_packets {
            if sub.operation != Operation::Message {
                continue;
            }

            let Ok(json) = serde_json::from_slice::<serde_json::Value>(&sub.body) else {
                continue;
            };

            let Some(cmd) = json.get("cmd").and_then(|c| c.as_str()) else {
                continue;
            };

            if !cmd.starts_with("DANMU_MSG") {
                continue;
            }

            let Some(info) = json.get("info").and_then(|i| i.as_array()) else {
                continue;
            };

            // info[1] = danmaku text, info[2] = [uid, username, ...]
            let text = info
                .get(1)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();

            let (uid, username) = info
                .get(2)
                .and_then(|v| v.as_array())
                .map(|arr| {
                    let uid = arr.first().and_then(|v| v.as_u64()).unwrap_or(0);
                    let name = arr
                        .get(1)
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    (uid, name)
                })
                .unwrap_or((0, String::new()));

            if !text.is_empty() {
                messages.push(DanmakuMessage {
                    uid,
                    username,
                    text,
                    timestamp: Utc::now(),
                });
            }
        }
    }

    messages
}

// --- WebSocket source ---

pub struct WsSource {
    room_id: u64,
    ws: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    http: reqwest::Client,
}

impl WsSource {
    pub fn new(room_id: u64) -> Self {
        Self {
            room_id,
            ws: None,
            http: reqwest::Client::new(),
        }
    }

    async fn get_buvid3(&self) -> anyhow::Result<String> {
        let resp: ApiResponse<FingerSpiData> = self
            .http
            .get("https://api.bilibili.com/x/frontend/finger/spi")
            .send()
            .await?
            .json()
            .await?;
        let data = resp
            .data
            .ok_or_else(|| anyhow::anyhow!("finger/spi returned no data"))?;
        Ok(data.b_3)
    }

    async fn get_wbi_keys(&self) -> anyhow::Result<(String, String)> {
        let resp: ApiResponse<NavData> = self
            .http
            .get("https://api.bilibili.com/x/web-interface/nav")
            .send()
            .await?
            .json()
            .await?;

        let data = resp
            .data
            .ok_or_else(|| anyhow::anyhow!("nav returned no data"))?;

        // Extract key from URL: .../xxx.png -> xxx
        let img_key = data
            .wbi_img
            .img_url
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".png")
            .to_string();
        let sub_key = data
            .wbi_img
            .sub_url
            .rsplit('/')
            .next()
            .unwrap_or("")
            .trim_end_matches(".png")
            .to_string();

        Ok((img_key, sub_key))
    }

    async fn get_danmu_info(&self, buvid: &str) -> anyhow::Result<(String, String, u16)> {
        let (img_key, sub_key) = self.get_wbi_keys().await?;
        let mixin_key = get_mixin_key(&img_key, &sub_key);

        let mut params = vec![("id".to_string(), self.room_id.to_string())];
        wbi_sign(&mut params, &mixin_key);

        let query: String = params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        let url = format!(
            "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo?{}",
            query
        );

        let resp: ApiResponse<DanmuInfoData> = self
            .http
            .get(&url)
            .header("Cookie", format!("buvid3={}", buvid))
            .send()
            .await?
            .json()
            .await?;

        if resp.code != 0 {
            anyhow::bail!("getDanmuInfo failed with code {}", resp.code);
        }

        let data = resp
            .data
            .ok_or_else(|| anyhow::anyhow!("getDanmuInfo returned no data"))?;

        let host = data
            .host_list
            .first()
            .ok_or_else(|| anyhow::anyhow!("empty host list"))?;

        Ok((data.token, host.host.clone(), host.wss_port))
    }

    fn build_auth_packet(&self, token: &str, buvid: &str) -> Vec<u8> {
        let auth_body = serde_json::json!({
            "uid": 0,
            "roomid": self.room_id,
            "protover": 3,
            "buvid": buvid,
            "platform": "web",
            "type": 2,
            "key": token,
        });

        Packet {
            version: 1,
            operation: Operation::Auth,
            body: auth_body.to_string().into_bytes(),
        }
        .encode()
    }

    fn build_heartbeat_packet() -> Vec<u8> {
        Packet {
            version: 1,
            operation: Operation::Heartbeat,
            body: vec![],
        }
        .encode()
    }
}

#[async_trait]
impl DanmakuSource for WsSource {
    async fn connect(&mut self) -> anyhow::Result<()> {
        info!(room_id = self.room_id, "connecting to bilibili live room");

        let buvid = self.get_buvid3().await?;
        debug!("got buvid3");

        let (token, host, wss_port) = self.get_danmu_info(&buvid).await?;
        debug!(host = %host, port = wss_port, "got danmu info");

        let url = format!("wss://{}:{}/sub", host, wss_port);
        let (ws, _) = connect_async(&url).await?;
        self.ws = Some(ws);

        // Send auth packet
        let auth = self.build_auth_packet(&token, &buvid);
        self.ws
            .as_mut()
            .unwrap()
            .send(Message::Binary(auth.into()))
            .await?;
        info!("auth packet sent, waiting for reply");

        // Wait for auth reply
        if let Some(msg) = self.ws.as_mut().unwrap().next().await {
            let msg = msg?;
            if let Message::Binary(data) = msg {
                let packets = Packet::decode(&data);
                for p in &packets {
                    if p.operation == Operation::AuthReply {
                        info!("auth successful");
                    }
                }
            }
        }

        // Start heartbeat task
        let ws_for_hb = self.ws.as_mut().unwrap();
        // We'll send first heartbeat immediately
        ws_for_hb
            .send(Message::Binary(Self::build_heartbeat_packet().into()))
            .await?;

        Ok(())
    }

    async fn next_message(&mut self) -> anyhow::Result<DanmakuMessage> {
        let ws = self
            .ws
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("not connected"))?;

        loop {
            let msg = ws
                .next()
                .await
                .ok_or_else(|| anyhow::anyhow!("connection closed"))??;

            match msg {
                Message::Binary(data) => {
                    let messages = parse_danmu_messages(&data);
                    if let Some(danmaku) = messages.into_iter().next() {
                        return Ok(danmaku);
                    }
                    // If no danmaku messages, continue reading
                }
                Message::Ping(payload) => {
                    ws.send(Message::Pong(payload)).await?;
                }
                Message::Close(_) => {
                    anyhow::bail!("server closed connection");
                }
                _ => {}
            }
        }
    }
}

// --- Tests ---

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mixin_key() {
        let img_key = "7cd084941338484aae1ad9425b84077c";
        let sub_key = "4932caff0ff746eab6f01bf08b70ac45";
        let mixin = get_mixin_key(img_key, sub_key);
        assert_eq!(mixin, "ea1db124af3c7062474693fa704f4ff8");
    }

    #[test]
    fn test_packet_encode_decode() {
        let body = b"hello".to_vec();
        let packet = Packet {
            version: 1,
            operation: Operation::Heartbeat,
            body: body.clone(),
        };

        let encoded = packet.encode();
        assert_eq!(encoded.len(), HEADER_SIZE + body.len());

        let decoded = Packet::decode(&encoded);
        assert_eq!(decoded.len(), 1);
        assert_eq!(decoded[0].version, 1);
        assert_eq!(decoded[0].operation, Operation::Heartbeat);
        assert_eq!(decoded[0].body, body);
    }

    #[test]
    fn test_packet_multiple() {
        let p1 = Packet {
            version: 0,
            operation: Operation::Message,
            body: b"msg1".to_vec(),
        };
        let p2 = Packet {
            version: 0,
            operation: Operation::Message,
            body: b"msg2".to_vec(),
        };

        let mut data = p1.encode();
        data.extend_from_slice(&p2.encode());

        let decoded = Packet::decode(&data);
        assert_eq!(decoded.len(), 2);
        assert_eq!(decoded[0].body, b"msg1");
        assert_eq!(decoded[1].body, b"msg2");
    }

    #[test]
    fn test_parse_danmu_msg() {
        let json = serde_json::json!({
            "cmd": "DANMU_MSG",
            "info": [
                [],
                "点歌 晴天",
                [12345, "testuser"],
            ]
        });

        let body = serde_json::to_vec(&json).unwrap();
        let packet = Packet {
            version: 0,
            operation: Operation::Message,
            body,
        };

        let data = packet.encode();
        let messages = parse_danmu_messages(&data);
        assert_eq!(messages.len(), 1);
        assert_eq!(messages[0].text, "点歌 晴天");
        assert_eq!(messages[0].uid, 12345);
        assert_eq!(messages[0].username, "testuser");
    }
}
