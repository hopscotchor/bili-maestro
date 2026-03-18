pub enum HttpUrls {
    DanmuInfo,
    RoomPlayInfo, // real room id
}

impl HttpUrls {
    pub fn url(&self) -> String {
        match &self {
            HttpUrls::DanmuInfo => {
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo".into()
            }
            HttpUrls::RoomPlayInfo => {
                "https://api.live.bilibili.com/xlive/web-room/v1/index/getRoomPlayInfo".into()
            }
        }
    }
}
