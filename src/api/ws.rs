use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use tokio::sync::broadcast;
use tracing::{debug, warn};

use super::AppState;
use crate::command::types::{Command, DanmakuMessage};

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/ws/commands", web::get().to(ws_commands))
        .route("/ws/danmaku", web::get().to(ws_danmaku));
}

async fn ws_commands(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    let (response, mut session, _msg_stream) = actix_ws::handle(&req, stream)?;

    let mut rx: broadcast::Receiver<Command> = state.command_tx.subscribe();

    actix_web::rt::spawn(async move {
        loop {
            match rx.recv().await {
                Ok(cmd) => {
                    let json = serde_json::to_string(&cmd).unwrap_or_default();
                    if session.text(json).await.is_err() {
                        break;
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    warn!("ws/commands client lagged by {} messages", n);
                }
                Err(broadcast::error::RecvError::Closed) => {
                    debug!("command broadcast channel closed");
                    break;
                }
            }
        }
    });

    Ok(response)
}

async fn ws_danmaku(
    req: HttpRequest,
    stream: web::Payload,
    state: web::Data<AppState>,
) -> actix_web::Result<HttpResponse> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, stream)?;

    let mut rx: broadcast::Receiver<DanmakuMessage> = state.danmaku_tx.subscribe();

    actix_web::rt::spawn(async move {
        loop {
            tokio::select! {
                result = rx.recv() => {
                    match result {
                        Ok(msg) => {
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            if session.text(json).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(n)) => {
                            warn!("ws/danmaku client lagged by {} messages", n);
                        }
                        Err(broadcast::error::RecvError::Closed) => {
                            break;
                        }
                    }
                }
                Some(Ok(msg)) = msg_stream.recv() => {
                    if matches!(msg, Message::Close(_)) {
                        break;
                    }
                }
            }
        }
    });

    Ok(response)
}
