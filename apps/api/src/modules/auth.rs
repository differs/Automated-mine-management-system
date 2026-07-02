use axum::{Json, Router, extract::State, routing::post};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    error::ApiError,
    middleware::auth::{create_refresh_token, create_token},
    state::AppState,
};

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
    pub role: String,
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

    // Query user from database
    let user_row = sqlx::query_as::<_, UserRow>(
        "SELECT id, username, password_hash, display_name, role::text AS role, is_active \
         FROM users WHERE username = $1",
    )
    .bind(payload.username.trim())
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("database error: {err}")))?
    .ok_or_else(|| ApiError::unauthorized("invalid username or password"))?;

    if !user_row.is_active {
        return Err(ApiError::unauthorized("account is disabled"));
    }

    // Verify password
    let valid = bcrypt::verify(payload.password.trim(), &user_row.password_hash)
        .unwrap_or(false);
    if !valid {
        return Err(ApiError::unauthorized("invalid username or password"));
    }

    // Generate tokens
    let access_token = create_token(
        &state.config.jwt_secret,
        user_row.id,
        &user_row.role,
        &user_row.display_name,
    )
    .map_err(|err| ApiError::internal(format!("failed to generate token: {err}")))?;

    let refresh_token = create_refresh_token(&state.config.jwt_secret, user_row.id)
        .map_err(|err| ApiError::internal(format!("failed to generate refresh token: {err}")))?;

    Ok(Json(LoginResponse {
        access_token,
        refresh_token,
        role: user_row.role,
        display_name: user_row.display_name,
    }))
}

async fn refresh_token(
    State(state): State<AppState>,
    Json(payload): Json<RefreshTokenRequest>,
) -> Result<Json<RefreshTokenResponse>, ApiError> {
    if payload.refresh_token.trim().is_empty() {
        return Err(ApiError::unauthorized("refresh token is required"));
    }

    // Decode and validate refresh token
    let token_data = jsonwebtoken::decode::<serde_json::Value>(
        payload.refresh_token.trim(),
        &jsonwebtoken::DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256),
    )
    .map_err(|_| ApiError::unauthorized("invalid or expired refresh token"))?;

    let user_id_str = token_data.claims.get("sub")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ApiError::unauthorized("invalid token payload"))?;

    let user_id = Uuid::parse_str(user_id_str)
        .map_err(|_| ApiError::unauthorized("invalid user_id in token"))?;

    // Fetch user to get current role and name
    let user_row = sqlx::query_as::<_, UserRow>(
        "SELECT id, username, password_hash, display_name, role::text AS role, is_active \
         FROM users WHERE id = $1 AND is_active = TRUE",
    )
    .bind(user_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("database error: {err}")))?
    .ok_or_else(|| ApiError::unauthorized("user not found or disabled"))?;

    let access_token = create_token(
        &state.config.jwt_secret,
        user_row.id,
        &user_row.role,
        &user_row.display_name,
    )
    .map_err(|err| ApiError::internal(format!("failed to generate token: {err}")))?;

    Ok(Json(RefreshTokenResponse { access_token }))
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct UserRow {
    id: Uuid,
    username: String,
    password_hash: String,
    display_name: String,
    role: String,
    is_active: bool,
}
