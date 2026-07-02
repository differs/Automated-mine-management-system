use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde::{Deserialize, Serialize};

use crate::{config::DispatchMode, state::AppState};

/// 系统配置管理API
///
/// 用于运行时切换纯算法/AI模式，无需重启服务。
/// 使用 Arc<RwLock<DispatchMode>> 实现多请求共享的运行时状态。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/dispatch-mode", get(get_dispatch_mode))
        .route("/dispatch-mode", post(set_dispatch_mode))
        .route("/system-config", get(get_system_config))
}

#[derive(Serialize)]
struct DispatchModeResponse {
    mode: String,
    label: String,
}

#[derive(Deserialize)]
struct SetDispatchModeRequest {
    mode: String,
}

#[derive(Serialize)]
struct SystemConfigResponse {
    dispatch_mode: String,
    ai_enabled: bool,
    ai_model: String,
    description: String,
}

/// 获取当前调度模式（从共享状态读取）
async fn get_dispatch_mode(
    State(state): State<AppState>,
) -> Json<DispatchModeResponse> {
    let mode = state.dispatch_mode.read().await;
    let label = match *mode {
        DispatchMode::PureAlgorithm => "纯算法模式 — 基于FIFO+规则的确定性调度",
        DispatchMode::AiEnhanced => "AI增强模式 — 盘古大模型优化调度决策",
    };
    Json(DispatchModeResponse {
        mode: mode.as_str().to_string(),
        label: label.to_string(),
    })
}

/// 切换调度模式（运行时生效，不重启）
///
/// 通过 Arc<RwLock> 写入共享状态，所有后续请求都会读到新值。
async fn set_dispatch_mode(
    State(state): State<AppState>,
    Json(req): Json<SetDispatchModeRequest>,
) -> Json<DispatchModeResponse> {
    let new_mode = DispatchMode::from_str(&req.mode);

    // 写入共享状态
    {
        let mut mode = state.dispatch_mode.write().await;
        *mode = new_mode.clone();
    }

    let label = match &new_mode {
        DispatchMode::PureAlgorithm => "已切换至纯算法模式 — 基于FIFO+规则的确定性调度",
        DispatchMode::AiEnhanced => "已切换至AI增强模式 — 盘古大模型优化调度决策",
    };
    tracing::info!("dispatch mode switched to: {:?}", new_mode);
    Json(DispatchModeResponse {
        mode: new_mode.as_str().to_string(),
        label: label.to_string(),
    })
}

/// 获取完整系统配置
async fn get_system_config(
    State(state): State<AppState>,
) -> Json<SystemConfigResponse> {
    let mode = state.dispatch_mode.read().await;
    Json(SystemConfigResponse {
        dispatch_mode: mode.as_str().to_string(),
        ai_enabled: state.config.ai.enabled,
        ai_model: state.config.ai.model.clone(),
        description: format!(
            "当前模式：{}，AI：{}，模型：{}",
            mode.as_str(),
            if state.config.ai.enabled { "已启用" } else { "未启用" },
            state.config.ai.model
        ),
    })
}
