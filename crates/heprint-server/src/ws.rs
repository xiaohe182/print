//! WebSocket 处理器：JSON-RPC 2.0 协议
//!
//! 收到客户端消息 → 解析 → 路由到 router → 返回响应
//! v1.1：支持多任务并行（HE_OPEN_TASK / HE_PRINT_TASK）

use axum::extract::ws::{Message, WebSocket};
use futures_util::{sink::SinkExt, stream::StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::Arc;

use crate::print_manager::PrintManager;
use crate::router::dispatch;
use crate::session::Session;

#[derive(Debug, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Value,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: &'static str,
    pub id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Serialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcResponse {
    pub fn ok(id: Value, result: Value) -> Self {
        Self { jsonrpc: "2.0", id, result: Some(result), error: None }
    }
    pub fn err(id: Value, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0", id, result: None,
            error: Some(JsonRpcError { code, message, data: None }),
        }
    }
}

pub async fn ws_handler(socket: WebSocket, mgr: Arc<PrintManager>) {
    let session = Session::new();
    handle_socket(socket, session, mgr).await;
}

async fn handle_socket(socket: WebSocket, session: Session, mgr: Arc<PrintManager>) {
    let (mut sender, mut receiver) = socket.split();

    while let Some(Ok(msg)) = receiver.next().await {
        match msg {
            Message::Text(text) => {
                let response = process_text(&text, &session, &mgr).await;
                if let Ok(s) = serde_json::to_string(&response) {
                    if sender.send(Message::Text(s.into())).await.is_err() {
                        break;
                    }
                }
            }
            Message::Close(_) => break,
            Message::Ping(p) => { let _ = sender.send(Message::Pong(p)).await; }
            _ => {}
        }
    }
    tracing::debug!("WS 连接关闭");
}

async fn process_text(text: &str, session: &Session, mgr: &Arc<PrintManager>) -> JsonRpcResponse {
    let req: JsonRpcRequest = match serde_json::from_str(text) {
        Ok(r) => r,
        Err(e) => return JsonRpcResponse::err(Value::Null, 1003, format!("JSON-RPC 解析失败: {e}")),
    };
    if req.jsonrpc != "2.0" {
        return JsonRpcResponse::err(req.id, 1003, "jsonrpc 必须为 2.0".to_string());
    }
    match dispatch(&req.method, req.params, session, mgr).await {
        Ok(value) => JsonRpcResponse::ok(req.id, value),
        Err(e) => {
            let code = e.error_code() as i32;
            JsonRpcResponse::err(req.id, code, e.to_string())
        }
    }
}
