use axum::{
    Router,
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    response::Response,
    routing::get,
};
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use uuid::Uuid;

use crate::{middleware::auth::{Claims, decode_token}, state::AppState};

/// WebSocket event types pushed to clients
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event", content = "data")]
pub enum WsEvent {
    #[serde(rename = "waybill.dispatched")]
    WaybillDispatched(WaybillEventPayload),
    #[serde(rename = "queue.updated")]
    QueueUpdated(QueueEventPayload),
    #[serde(rename = "queue.called")]
    QueueCalled(QueueCallPayload),
    #[serde(rename = "loading.started")]
    LoadingStarted(LoadingEventPayload),
    #[serde(rename = "loading.finished")]
    LoadingFinished(LoadingEventPayload),
    #[serde(rename = "weighing.completed")]
    WeighingCompleted(WeighingEventPayload),
    #[serde(rename = "alert.created")]
    AlertCreated(AlertEventPayload),
}

#[derive(Debug, Clone, Serialize)]
pub struct WaybillEventPayload {
    pub waybill_id: Uuid,
    pub driver_id: Uuid,
    pub pit_id: Uuid,
    pub pit_name: String,
    pub serial_no: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueEventPayload {
    pub pit_id: Uuid,
    pub pit_name: String,
    pub current_queue_count: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct QueueCallPayload {
    pub waybill_id: Uuid,
    pub driver_id: Uuid,
    pub pit_id: Uuid,
    pub pit_name: String,
    pub queue_position: i32,
}

#[derive(Debug, Clone, Serialize)]
pub struct LoadingEventPayload {
    pub waybill_id: Uuid,
    pub driver_id: Uuid,
    pub pit_id: Uuid,
}

#[derive(Debug, Clone, Serialize)]
pub struct WeighingEventPayload {
    pub waybill_id: Uuid,
    pub driver_id: Uuid,
    pub net_weight_ton: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct AlertEventPayload {
    pub alert_id: Uuid,
    pub waybill_id: Uuid,
    pub r#type: String,
    pub description: String,
}

/// Shared broadcast channel for WebSocket events
pub type EventTx = broadcast::Sender<WsEvent>;

#[derive(Deserialize)]
pub struct WsQuery {
    /// JWT access token（浏览器无法设置 WebSocket 请求头，通过查询参数传递）
    pub token: String,
}

pub fn router() -> Router<AppState> {
    Router::new().route("/", get(ws_handler))
}

/// WebSocket 连接入口
///
/// 验证流程：
/// 1. 从查询参数 `?token=xxx` 提取 JWT
/// 2. 验证签名和有效期
/// 3. 验证通过后升级为 WebSocket 连接
/// 4. 验证失败返回 401
async fn ws_handler(
    ws: WebSocketUpgrade,
    Query(query): Query<WsQuery>,
    State(state): State<AppState>,
) -> Result<Response, String> {
    // 验证 JWT token
    let claims = decode_token(&state.config.jwt_secret, &query.token)
        .map_err(|e| {
            tracing::warn!("WebSocket auth failed: {e}");
            format!("authentication failed: {e}")
        })?;

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| "invalid user_id in token".to_string())?;

    tracing::info!("WebSocket connected: user={user_id}, role={}", claims.role);

    // 升级为 WebSocket 连接
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, user_id, claims)))
}

async fn handle_socket(mut socket: WebSocket, state: AppState, user_id: Uuid, claims: Claims) {
    let mut rx = state.ws_tx.subscribe();

    // 发送连接确认（包含用户信息）
    let welcome = serde_json::json!({
        "event": "connected",
        "data": {
            "server": "auto-mining-system",
            "version": env!("CARGO_PKG_VERSION"),
            "user_id": user_id,
            "role": claims.role,
        }
    });
    let _ = socket.send(Message::Text(welcome.to_string().into())).await;

    loop {
        tokio::select! {
            msg = socket.recv() => {
                match msg {
                    Some(Ok(Message::Ping(data))) => {
                        let _ = socket.send(Message::Pong(data)).await;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            event = rx.recv() => {
                match event {
                    Ok(event) => {
                        let payload = serde_json::to_string(&event).unwrap_or_default();
                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("WebSocket client lagged by {n} events");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    tracing::info!("WebSocket disconnected: user={user_id}");
}

/// Create a broadcast channel for WebSocket events
pub fn create_event_channel() -> EventTx {
    let (tx, _) = broadcast::channel(256);
    tx
}

/// 广播 WebSocket 事件（fire-and-forget，忽略发送错误）
pub fn broadcast_event(tx: &EventTx, event: WsEvent) {
    let _ = tx.send(event);
}
