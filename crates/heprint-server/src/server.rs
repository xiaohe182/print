//! Axum 服务启动 + 路由注册

use axum::{
    extract::ws::WebSocketUpgrade,
    response::IntoResponse,
    routing::get,
    Json, Router,
};
use serde_json::json;
use std::net::SocketAddr;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

use crate::print_manager::{spawn_workers, PrintManager};
use crate::ws::ws_handler;
use crate::HE_VERSION;

#[derive(Debug, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub http_port: u16,
    pub max_concurrent: usize,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            http_port: 18000,
            max_concurrent: 4,
        }
    }
}

/// 启动 Axum 服务（异步常驻）
pub async fn run(config: ServerConfig) -> anyhow::Result<()> {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // 创建全局 PrintManager + 启动 worker pool
    let print_manager = PrintManager::new(config.max_concurrent);
    spawn_workers(print_manager.clone(), config.max_concurrent);
    tracing::info!("PrintManager 已启动，worker pool: {} 个并发", config.max_concurrent);

    let app = Router::new()
        .route("/", get(index))
        .route("/version", get(version))
        .route("/health", get(health))
        .route("/printers", get(printers))
        .route("/printers/default", get(default_printer))
        .route("/ws", get(ws_upgrade))
        .route("/queue", get(queue_status))
        .with_state(print_manager.clone())
        .layer(cors);

    let addr: SocketAddr = format!("{}:{}", config.host, config.http_port).parse()?;
    tracing::info!("HePrint 服务启动: http://{}", addr);
    tracing::info!("WebSocket 端点: ws://{}/ws", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn index() -> impl IntoResponse {
    Json(json!({
        "service": "HePrint",
        "version": HE_VERSION,
        "endpoints": {
            "version": "GET /version",
            "health": "GET /health",
            "printers": "GET /printers",
            "default_printer": "GET /printers/default",
            "queue": "GET /queue",
            "websocket": "WS /ws"
        }
    }))
}

async fn version() -> impl IntoResponse {
    Json(json!({ "version": HE_VERSION }))
}

async fn health() -> impl IntoResponse {
    Json(json!({ "status": "ok" }))
}

async fn printers() -> impl IntoResponse {
    match heprint_print::enum_printers() {
        Ok(list) => Json(json!({ "ok": true, "printers": list })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn default_printer() -> impl IntoResponse {
    match heprint_print::get_default_printer() {
        Ok(name) => Json(json!({ "ok": true, "name": name })),
        Err(e) => Json(json!({ "ok": false, "error": e.to_string() })),
    }
}

async fn queue_status(
    axum::extract::State(mgr): axum::extract::State<Arc<PrintManager>>,
) -> impl IntoResponse {
    Json(json!({
        "runningJobs": mgr.running_count(),
        "queueLength": mgr.queue_len(),
    }))
}

async fn ws_upgrade(
    ws: WebSocketUpgrade,
    axum::extract::State(mgr): axum::extract::State<Arc<PrintManager>>,
) -> impl IntoResponse {
    ws.on_upgrade(move |socket| ws_handler(socket, mgr))
}
