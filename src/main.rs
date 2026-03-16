use std::path::PathBuf;
use std::time::Duration;

use actix_web::{web, App, HttpServer};
use clap::Parser;
use tokio::time;
use tracing::{error, info, warn};

use bili_maestro::api::AppState;
use bili_maestro::command::CommandParser;
use bili_maestro::config::Config;
use bili_maestro::source::websocket::WsSource;
use bili_maestro::source::DanmakuSource;

#[derive(Parser)]
#[command(name = "bili-maestro", about = "Bilibili弹幕点歌/切视频控制器")]
struct Cli {
    /// 直播间房间号
    #[arg(short, long)]
    room: Option<u64>,

    /// API 服务端口
    #[arg(short, long, default_value_t = 8080)]
    port: u16,

    /// 配置文件路径
    #[arg(short, long)]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "bili_maestro=info".into()),
        )
        .init();

    let cli = Cli::parse();

    let config = if let Some(config_path) = &cli.config {
        Config::from_file(config_path)?
    } else {
        let room_id = cli.room.expect("either --room or --config is required");
        Config {
            room_id,
            api_port: cli.port,
            commands: Default::default(),
        }
    };

    info!(room_id = config.room_id, port = config.api_port, "starting bili-maestro");

    let state = AppState::new(config.room_id);
    let state_clone = state.clone();
    let parser = CommandParser::new(config.commands.clone());

    // Spawn the danmaku reader task
    let danmaku_handle = tokio::spawn(async move {
        let mut retry_delay = Duration::from_secs(1);
        let max_delay = Duration::from_secs(60);

        loop {
            let mut source = WsSource::new(config.room_id);

            match source.connect().await {
                Ok(()) => {
                    info!("connected to bilibili live room");
                    *state_clone.connected.write().await = true;
                    retry_delay = Duration::from_secs(1);

                    // Spawn heartbeat task
                    let hb_state = state_clone.clone();
                    let hb_handle = tokio::spawn(async move {
                        let mut interval = time::interval(Duration::from_secs(30));
                        loop {
                            interval.tick().await;
                            // Heartbeat is handled inside the source, but we keep state alive
                            let _ = *hb_state.connected.read().await;
                        }
                    });

                    loop {
                        match source.next_message().await {
                            Ok(msg) => {
                                info!(
                                    user = %msg.username,
                                    text = %msg.text,
                                    "danmaku"
                                );

                                if let Some(cmd) = parser.parse(&msg) {
                                    info!(
                                        command = ?cmd.command_type,
                                        user = %cmd.username,
                                        "command detected"
                                    );
                                    state_clone.add_command(cmd).await;
                                }

                                state_clone.add_danmaku(msg).await;
                            }
                            Err(e) => {
                                error!("danmaku stream error: {}", e);
                                break;
                            }
                        }
                    }

                    hb_handle.abort();
                    *state_clone.connected.write().await = false;
                }
                Err(e) => {
                    error!("connection failed: {}", e);
                }
            }

            warn!(
                delay_secs = retry_delay.as_secs(),
                "reconnecting after delay"
            );
            time::sleep(retry_delay).await;
            retry_delay = (retry_delay * 2).min(max_delay);
        }
    });

    // Start API server
    let api_state = web::Data::new(state);
    let port = cli.port;

    info!(port, "starting API server");

    let server = HttpServer::new(move || {
        App::new()
            .app_data(api_state.clone())
            .configure(bili_maestro::api::http::configure)
            .configure(bili_maestro::api::ws::configure)
    })
    .bind(("0.0.0.0", port))?
    .run();

    tokio::select! {
        result = server => {
            if let Err(e) = result {
                error!("API server error: {}", e);
            }
        }
        _ = danmaku_handle => {
            error!("danmaku reader task ended unexpectedly");
        }
        _ = tokio::signal::ctrl_c() => {
            info!("shutting down");
        }
    }

    Ok(())
}
