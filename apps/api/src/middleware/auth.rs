use axum::{
    Json,
    extract::FromRequestParts,
    http::{StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::state::AppState;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub display_name: String,
    pub exp: usize,
    pub iat: usize,
}

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
    pub role: String,
    pub display_name: String,
}

impl FromRequestParts<AppState> for AuthenticatedUser {
    type Rejection = AuthError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &AppState,
    ) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get(header::AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .ok_or_else(|| AuthError::unauthorized("missing Authorization header"))?;

        let token = auth_header
            .strip_prefix("Bearer ")
            .ok_or_else(|| AuthError::unauthorized("invalid Authorization header format"))?;

        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .map_err(|err| {
            tracing::warn!("JWT validation failed: {err}");
            AuthError::unauthorized("invalid or expired token")
        })?;

        let user_id = Uuid::parse_str(&token_data.claims.sub)
            .map_err(|_| AuthError::unauthorized("invalid user_id in token"))?;

        Ok(AuthenticatedUser {
            user_id,
            role: token_data.claims.role,
            display_name: token_data.claims.display_name,
        })
    }
}

#[derive(Debug)]
pub struct AuthError {
    status: StatusCode,
    message: String,
}

impl AuthError {
    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            message: message.into(),
        }
    }

    #[allow(dead_code)]
    pub fn forbidden(message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            message: message.into(),
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let body = serde_json::json!({
            "code": "auth_error",
            "message": self.message,
        });
        (self.status, Json(body)).into_response()
    }
}

/// Helper to generate a JWT token (24h expiry)
pub fn create_token(
    secret: &str,
    user_id: Uuid,
    role: &str,
    display_name: &str,
) -> anyhow::Result<String> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        role: role.to_string(),
        display_name: display_name.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::hours(24)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// Helper to generate a refresh token (30 days expiry)
pub fn create_refresh_token(
    secret: &str,
    user_id: Uuid,
) -> anyhow::Result<String> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: user_id.to_string(),
        role: String::new(),
        display_name: String::new(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::days(30)).timestamp() as usize,
    };

    let token = jsonwebtoken::encode(
        &jsonwebtoken::Header::default(),
        &claims,
        &jsonwebtoken::EncodingKey::from_secret(secret.as_bytes()),
    )?;

    Ok(token)
}

/// 验证 JWT token 并返回 Claims
pub fn decode_token(secret: &str, token: &str) -> Result<Claims, String> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|err| format!("invalid or expired token: {err}"))?;

    Ok(token_data.claims)
}

/// Tower middleware: 验证 Bearer JWT token
///
/// 从 Authorization header 中提取 Bearer token，验证签名和有效期。
/// 验证通过后将 Claims 信息注入 request extensions，供下游 handler 使用。
pub async fn require_auth(
    mut request: axum::http::Request<axum::body::Body>,
    next: Next,
) -> Result<Response, AuthError> {
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| AuthError::unauthorized("missing Authorization header"))?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or_else(|| AuthError::unauthorized("invalid Authorization header format"))?;

    // 从 request extensions 中获取 jwt_secret（由 build_router 注入）
    let secret = request
        .extensions()
        .get::<String>()
        .cloned()
        .ok_or_else(|| AuthError::unauthorized("jwt secret not configured"))?;

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &Validation::new(Algorithm::HS256),
    )
    .map_err(|err| {
        tracing::warn!("JWT validation failed: {err}");
        AuthError::unauthorized("invalid or expired token")
    })?;

    let user_id = Uuid::parse_str(&token_data.claims.sub)
        .map_err(|_| AuthError::unauthorized("invalid user_id in token"))?;

    let authenticated_user = AuthenticatedUser {
        user_id,
        role: token_data.claims.role,
        display_name: token_data.claims.display_name,
    };

    request.extensions_mut().insert(authenticated_user);

    Ok(next.run(request).await)
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[test]
    fn test_create_and_decode_token() {
        let secret = "test-secret-key";
        let user_id = Uuid::new_v4();
        let role = "dispatcher";
        let display_name = "测试调度员";

        let token = create_token(secret, user_id, role, display_name).unwrap();
        assert!(!token.is_empty());

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, user_id.to_string());
        assert_eq!(token_data.claims.role, role);
        assert_eq!(token_data.claims.display_name, display_name);
    }

    #[test]
    fn test_create_refresh_token() {
        let secret = "test-secret-key";
        let user_id = Uuid::new_v4();

        let token = create_refresh_token(secret, user_id).unwrap();
        assert!(!token.is_empty());

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();

        assert_eq!(token_data.claims.sub, user_id.to_string());
        assert!(token_data.claims.role.is_empty());
        assert!(token_data.claims.display_name.is_empty());
    }

    #[test]
    fn test_token_wrong_secret_fails() {
        let secret = "correct-secret";
        let user_id = Uuid::new_v4();

        let token = create_token(secret, user_id, "admin", "test").unwrap();

        let result = decode::<Claims>(
            &token,
            &DecodingKey::from_secret("wrong-secret".as_bytes()),
            &Validation::new(Algorithm::HS256),
        );

        assert!(result.is_err());
    }

    #[test]
    fn test_token_expiry_in_future() {
        let secret = "test-secret-key";
        let user_id = Uuid::new_v4();

        let token = create_token(secret, user_id, "admin", "test").unwrap();

        let token_data = decode::<Claims>(
            &token,
            &DecodingKey::from_secret(secret.as_bytes()),
            &Validation::new(Algorithm::HS256),
        )
        .unwrap();

        let now = chrono::Utc::now().timestamp() as usize;
        assert!(token_data.claims.exp > now);
        assert!(token_data.claims.exp <= now + 86400);
    }
}
