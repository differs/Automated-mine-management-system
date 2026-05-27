use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};

use crate::{error::ApiError, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub username: String,
    pub password: String,
}

#[derive(Serialize)]
pub struct LoginResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub role: &'static str,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Serialize)]
pub struct RefreshTokenResponse {
    pub access_token: String,
}

async fn login(
    State(state): State<AppState>,
    Json(payload): Json<LoginRequest>,
) -> Result<Json<LoginResponse>, ApiError> {
    if payload.username.trim().is_empty() || payload.password.trim().is_empty() {
        return Err(ApiError::bad_request("username and password are required"));
    }

    let token_seed = format!("{}:{}", payload.username, state.config.jwt_secret.len());

    Ok(Json(LoginResponse {
        access_token: format!("mock-access-token-{token_seed}"),
        refresh_token: format!("mock-refresh-token-{token_seed}"),
        role: "dispatcher",
        display_name: payload.username,
    }))
}

async fn refresh_token(
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, ApiError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(ApiError::unauthorized("refresh token is required"));
    }

    Ok(Json(RefreshTokenResponse {
        access_token: format!("mock-access-token-from-{}", payload.refresh_token),
    }))
}
