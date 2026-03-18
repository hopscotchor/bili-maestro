use anyhow::Result;

const DANMU_INFO_URL: &str = "https://api.live.bilibili.com/xlive/web-room/v1/index/getDanmuInfo";

#[tokio::main]
async fn main() -> Result<()> {
    let client = reqwest::Client::new();
    let res = client.get(DANMU_INFO_URL).send().await?;
    Ok(())
}
