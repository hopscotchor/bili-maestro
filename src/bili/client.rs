use anyhow::Result;
use tokio::net::TcpStream;
use tokio_tungstenite::{connect_async, MaybeTlsStream, WebSocketStream};

pub struct BiliClient {
    http_client: reqwest::Client,
    ws_client: Option<WebSocketStream<MaybeTlsStream<TcpStream>>>,
}

impl BiliClient {
    pub fn new() -> Self {
        Self {
            http_client: reqwest::Client::new(),
            ws_client: None,
        }
    }

    pub async fn ws_connect(&mut self, url: impl Into<String>) -> Result<()> {
        let (ws, _) = connect_async(url.into()).await?;
        self.ws_client = Some(ws);
        Ok(())
    }
}
