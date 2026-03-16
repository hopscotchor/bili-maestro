use actix_web::{web, HttpResponse};
use serde::Deserialize;
use uuid::Uuid;

use super::AppState;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            .route("/health", web::get().to(health))
            .route("/status", web::get().to(status))
            .route("/commands", web::get().to(list_commands))
            .route("/commands/{id}/ack", web::post().to(ack_command))
            .route("/danmaku", web::get().to(list_danmaku)),
    );
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({"status": "ok"}))
}

async fn status(state: web::Data<AppState>) -> HttpResponse {
    let connected = *state.connected.read().await;
    let popularity = *state.popularity.read().await;
    HttpResponse::Ok().json(serde_json::json!({
        "room_id": state.room_id,
        "connected": connected,
        "popularity": popularity,
    }))
}

#[derive(Deserialize)]
pub struct CommandsQuery {
    #[serde(rename = "type")]
    pub type_filter: Option<String>,
}

async fn list_commands(
    state: web::Data<AppState>,
    query: web::Query<CommandsQuery>,
) -> HttpResponse {
    let commands = state
        .command_queue
        .list(query.type_filter.as_deref())
        .await;
    HttpResponse::Ok().json(commands)
}

async fn ack_command(state: web::Data<AppState>, path: web::Path<String>) -> HttpResponse {
    let id_str = path.into_inner();
    let Ok(id) = Uuid::parse_str(&id_str) else {
        return HttpResponse::BadRequest().json(serde_json::json!({"error": "invalid uuid"}));
    };

    if state.command_queue.ack(id).await {
        HttpResponse::Ok().json(serde_json::json!({"status": "acked"}))
    } else {
        HttpResponse::NotFound().json(serde_json::json!({"error": "command not found"}))
    }
}

#[derive(Deserialize)]
pub struct DanmakuQuery {
    pub limit: Option<usize>,
}

async fn list_danmaku(state: web::Data<AppState>, query: web::Query<DanmakuQuery>) -> HttpResponse {
    let limit = query.limit.unwrap_or(50).min(200);
    let recent = state.recent_danmaku.read().await;
    let items: Vec<_> = recent.iter().rev().take(limit).cloned().collect();
    HttpResponse::Ok().json(items)
}
