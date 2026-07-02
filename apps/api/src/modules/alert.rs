use axum::{
    Json, Router,
    extract::{Path, Query, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, QueryBuilder, Row};
use uuid::Uuid;

use crate::{error::ApiError, pagination::{Pagination, PagedResponse}, state::AppState};

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/", get(list_alerts))
        .route("/{alert_id}/resolve", post(resolve_alert))
}

#[derive(Deserialize)]
pub struct AlertListQuery {
    pub status: Option<String>,
    pub r#type: Option<String>,
    #[serde(flatten)]
    pub pagination: Pagination,
}

#[derive(Serialize, FromRow)]
pub struct AlertRecord {
    pub id: Uuid,
    pub waybill_id: Uuid,
    pub r#type: String,
    pub severity: i16,
    pub description: String,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Serialize)]
pub struct AlertResponse {
    pub id: Uuid,
    pub status: String,
    pub resolved_at: DateTime<Utc>,
}

async fn list_alerts(
    State(state): State<AppState>,
    Query(query): Query<AlertListQuery>,
) -> Result<Json<PagedResponse<AlertRecord>>, ApiError> {
    let (offset, limit) = query.pagination.offset_limit();

    // 先查询总数
    let mut count_qb = QueryBuilder::new("SELECT COUNT(*)::bigint FROM exception_records WHERE 1=1");
    if let Some(status) = &query.status {
        count_qb.push(" AND status::text = ").push_bind(status.as_str());
    }
    if let Some(r#type) = &query.r#type {
        count_qb.push(" AND type::text = ").push_bind(r#type.as_str());
    }
    let total: i64 = count_qb
        .build_query_scalar()
        .fetch_one(&state.db)
        .await
        .map_err(|err| ApiError::internal(format!("failed to count alerts: {err}")))?;

    // 查询数据
    let mut qb = QueryBuilder::new(
        "SELECT id, waybill_id, type::text AS type, severity, description, \
         status::text AS status, created_at, resolved_at \
         FROM exception_records WHERE 1=1",
    );

    if let Some(status) = &query.status {
        qb.push(" AND status::text = ").push_bind(status.as_str());
    }
    if let Some(r#type) = &query.r#type {
        qb.push(" AND type::text = ").push_bind(r#type.as_str());
    }

    qb.push(" ORDER BY created_at DESC LIMIT ")
        .push_bind(limit)
        .push(" OFFSET ")
        .push_bind(offset);

    let rows = qb
        .build_query_as::<AlertRecord>()
        .fetch_all(&state.db)
        .await
        .map_err(|err| ApiError::internal(format!("failed to list alerts: {err}")))?;

    Ok(Json(PagedResponse::new(
        rows,
        total,
        query.pagination.page(),
        query.pagination.page_size(),
    )))
}

async fn resolve_alert(
    State(state): State<AppState>,
    Path(alert_id): Path<Uuid>,
) -> Result<Json<AlertResponse>, ApiError> {
    let now = Utc::now();
    let row = sqlx::query(
        "UPDATE exception_records SET status = 'resolved', resolved_at = $2, updated_at = $2 \
         WHERE id = $1 AND status = 'open' \
         RETURNING id, status::text AS status, resolved_at",
    )
    .bind(alert_id)
    .bind(now)
    .fetch_optional(&state.db)
    .await
    .map_err(|err| ApiError::internal(format!("failed to resolve alert: {err}")))?;

    let Some(row) = row else {
        return Err(ApiError::not_found("alert not found or already resolved"));
    };

    Ok(Json(AlertResponse {
        id: row.get("id"),
        status: row.get("status"),
        resolved_at: row.get("resolved_at"),
    }))
}
