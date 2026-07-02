use axum::{
    Json, Router,
    extract::State,
    routing::post,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{error::ApiError, state::AppState};

/// 离线调度模块
///
/// 为移动端（司机App/坑口端App）提供离线操作同步接口。
///
/// 离线工作流程:
///   1. 移动端在无网/弱网时，将操作写入本地 SQLite
///   2. 网络恢复后，调用 POST /api/v1/offline/sync 批量提交
///   3. 服务端用 idempotency_key 去重，用 version 做乐观锁冲突检测
///   4. 返回每个操作的结果（成功/冲突/失败）
///
/// 事务保证: 幂等检查 + 业务操作 + 幂等键记录在同一事务中。
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/sync", post(sync_offline_operations))
        .route("/sync/state", post(sync_fetch_state))
}

// ─── 数据模型 ─────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct OfflineSyncRequest {
    pub device_id: String,
    pub operator_id: Uuid,
    pub operations: Vec<OfflineOperation>,
    pub last_synced_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Deserialize)]
pub struct OfflineOperation {
    pub idempotency_key: String,
    pub operation_type: String,
    pub waybill_id: Uuid,
    pub waybill_version: i32,
    pub payload: serde_json::Value,
    pub occurred_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct OfflineSyncResponse {
    pub results: Vec<SyncResult>,
    pub server_state: serde_json::Value,
    pub server_time: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct SyncResult {
    pub idempotency_key: String,
    pub waybill_id: Uuid,
    pub status: String,
    pub message: String,
    pub server_version: i32,
}

#[derive(Debug, Deserialize)]
pub struct SyncStateRequest {
    pub operator_id: Uuid,
    pub operator_type: String,
    pub last_synced_at: Option<DateTime<Utc>>,
}

// ─── API 实现 ──────────────────────────────────────────────────────────────

/// 批量提交离线操作
async fn sync_offline_operations(
    State(state): State<AppState>,
    Json(req): Json<OfflineSyncRequest>,
) -> Result<Json<OfflineSyncResponse>, ApiError> {
    let mut results = Vec::with_capacity(req.operations.len());

    for op in &req.operations {
        let result = process_offline_operation(&state, op).await;
        results.push(result);
    }

    let server_state = fetch_server_state(&state, &req.operator_id, &req.last_synced_at).await?;

    Ok(Json(OfflineSyncResponse {
        results,
        server_state,
        server_time: Utc::now(),
    }))
}

/// 处理单条离线操作（事务化）
///
/// 将幂等检查 + 版本校验 + 业务操作 + 幂等键记录放在同一事务中，
/// 防止并发请求导致数据不一致。
async fn process_offline_operation(
    state: &AppState,
    op: &OfflineOperation,
) -> SyncResult {
    let mut tx = match state.db.begin().await {
        Ok(tx) => tx,
        Err(e) => {
            return SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "error".into(),
                message: format!("failed to begin transaction: {e}"),
                server_version: 0,
            };
        }
    };

    // 1. 幂等性检查（在事务内）
    let existing = sqlx::query_scalar::<_, String>(
        "SELECT result FROM idempotency_keys WHERE key = $1 AND expires_at > NOW()",
    )
    .bind(&op.idempotency_key)
    .fetch_optional(&mut *tx)
    .await;

    match existing {
        Ok(Some(_)) => {
            let _ = tx.rollback().await;
            return SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "skipped".into(),
                message: "already processed".into(),
                server_version: 0,
            };
        }
        Ok(None) => { /* 未处理，继续 */ }
        Err(e) => {
            let _ = tx.rollback().await;
            return SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "error".into(),
                message: format!("idempotency check failed: {e}"),
                server_version: 0,
            };
        }
    }

    // 2. 乐观锁版本检查（在事务内）
    let current_version = sqlx::query_scalar::<_, i32>(
        "SELECT version FROM waybills WHERE id = $1",
    )
    .bind(op.waybill_id)
    .fetch_optional(&mut *tx)
    .await;

    let current_version = match current_version {
        Ok(Some(v)) => v,
        Ok(None) => {
            let _ = tx.rollback().await;
            return SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "error".into(),
                message: "waybill not found".into(),
                server_version: 0,
            };
        }
        Err(e) => {
            let _ = tx.rollback().await;
            return SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "error".into(),
                message: format!("version check failed: {e}"),
                server_version: 0,
            };
        }
    };

    if current_version != op.waybill_version {
        let _ = tx.rollback().await;
        return SyncResult {
            idempotency_key: op.idempotency_key.clone(),
            waybill_id: op.waybill_id,
            status: "conflict".into(),
            message: format!(
                "version mismatch: client={}, server={}",
                op.waybill_version, current_version
            ),
            server_version: current_version,
        };
    }

    // 3. 执行业务操作（在事务内）
    let result = execute_operation_in_tx(&mut tx, op).await;

    // 如果业务操作失败，回滚事务
    if result.status != "synced" {
        let _ = tx.rollback().await;
        return result;
    }

    // 4. 记录幂等键（在事务内，7天过期）
    let result_json = serde_json::to_string(&result).unwrap_or_default();
    if let Err(e) = sqlx::query(
        "INSERT INTO idempotency_keys (key, result, expires_at) \
         VALUES ($1, $2, NOW() + INTERVAL '7 days') \
         ON CONFLICT (key) DO NOTHING",
    )
    .bind(&op.idempotency_key)
    .bind(&result_json)
    .execute(&mut *tx)
    .await
    {
        let _ = tx.rollback().await;
        return SyncResult {
            idempotency_key: op.idempotency_key.clone(),
            waybill_id: op.waybill_id,
            status: "error".into(),
            message: format!("failed to record idempotency key: {e}"),
            server_version: current_version,
        };
    }

    // 5. 提交事务
    if let Err(e) = tx.commit().await {
        return SyncResult {
            idempotency_key: op.idempotency_key.clone(),
            waybill_id: op.waybill_id,
            status: "error".into(),
            message: format!("failed to commit transaction: {e}"),
            server_version: current_version,
        };
    }

    result
}

/// 在事务内执行具体的离线操作
async fn execute_operation_in_tx(
    tx: &mut sqlx::PgConnection,
    op: &OfflineOperation,
) -> SyncResult {
    let now = Utc::now();

    let db_result = match op.operation_type.as_str() {
        "arrive" => {
            sqlx::query(
                "UPDATE waybills SET status = 'arrived', arrive_time = $2, \
                 arrival_source = 'offline_app', updated_at = $2, version = version + 1 \
                 WHERE id = $1 AND status IN ('dispatched', 'arrived')",
            )
            .bind(op.waybill_id)
            .bind(now)
            .execute(&mut *tx)
            .await
        }
        "queue_join" => {
            sqlx::query(
                "UPDATE waybills SET status = 'queueing', queue_enter_time = $2, \
                 updated_at = $2, version = version + 1 \
                 WHERE id = $1 AND status = 'arrived'",
            )
            .bind(op.waybill_id)
            .bind(now)
            .execute(&mut *tx)
            .await
        }
        "loading_start" => {
            sqlx::query(
                "UPDATE waybills SET status = 'loading', load_start_time = $2, \
                 updated_at = $2, version = version + 1 \
                 WHERE id = $1 AND status IN ('queueing', 'arrived')",
            )
            .bind(op.waybill_id)
            .bind(now)
            .execute(&mut *tx)
            .await
        }
        "loading_finish" => {
            sqlx::query(
                "UPDATE waybills SET status = 'loaded', load_end_time = $2, \
                 updated_at = $2, version = version + 1 \
                 WHERE id = $1 AND status = 'loading'",
            )
            .bind(op.waybill_id)
            .bind(now)
            .execute(&mut *tx)
            .await
        }
        "weigh" => {
            let gross = op.payload.get("gross_weight").and_then(|v| v.as_f64());
            let tare = op.payload.get("tare_weight").and_then(|v| v.as_f64());

            sqlx::query(
                "UPDATE waybills SET status = 'completed', completed_at = $2, \
                 gross_weight_ton = COALESCE($3, gross_weight_ton), \
                 tare_weight_ton = COALESCE($4, tare_weight_ton), \
                 actual_weight_ton = COALESCE($3, 0) - COALESCE($4, 0), \
                 updated_at = $2, version = version + 1 \
                 WHERE id = $1 AND status IN ('loaded', 'weighing')",
            )
            .bind(op.waybill_id)
            .bind(now)
            .bind(gross)
            .bind(tare)
            .execute(&mut *tx)
            .await
        }
        _ => {
            return SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "error".into(),
                message: format!("unknown operation type: {}", op.operation_type),
                server_version: 0,
            };
        }
    };

    match db_result {
        Ok(r) if r.rows_affected() > 0 => {
            let new_version = op.waybill_version + 1;
            SyncResult {
                idempotency_key: op.idempotency_key.clone(),
                waybill_id: op.waybill_id,
                status: "synced".into(),
                message: "ok".into(),
                server_version: new_version,
            }
        }
        Ok(_) => SyncResult {
            idempotency_key: op.idempotency_key.clone(),
            waybill_id: op.waybill_id,
            status: "conflict".into(),
            message: "status transition not allowed".into(),
            server_version: op.waybill_version,
        },
        Err(e) => SyncResult {
            idempotency_key: op.idempotency_key.clone(),
            waybill_id: op.waybill_id,
            status: "error".into(),
            message: format!("db error: {e}"),
            server_version: op.waybill_version,
        },
    }
}

#[derive(Serialize, sqlx::FromRow)]
struct SyncWaybillState {
    id: Uuid,
    serial_no: String,
    driver_id: Uuid,
    pit_id: Uuid,
    #[sqlx(rename = "status")]
    status_raw: String,
    queue_number: Option<i32>,
    estimated_weight_ton: Option<f64>,
    actual_weight_ton: Option<f64>,
    dispatch_time: Option<DateTime<Utc>>,
    arrive_time: Option<DateTime<Utc>>,
    version: i32,
}

/// 获取服务端最新状态（供客户端同步）
async fn fetch_server_state(
    state: &AppState,
    operator_id: &Uuid,
    last_synced_at: &Option<DateTime<Utc>>,
) -> Result<serde_json::Value, ApiError> {
    let waybills: Vec<SyncWaybillState> = if let Some(since) = last_synced_at {
        sqlx::query_as::<_, SyncWaybillState>(
            r#"
            SELECT id, serial_no, driver_id, pit_id, status::text AS status_raw,
                   queue_number, estimated_weight_ton, actual_weight_ton,
                   dispatch_time, arrive_time, version
            FROM waybills
            WHERE (driver_id = $1 OR EXISTS(
                SELECT 1 FROM pits WHERE id = waybills.pit_id AND manager_id = $1
            ))
            AND updated_at > $2
            ORDER BY updated_at DESC
            "#,
        )
        .bind(operator_id)
        .bind(since)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("failed to fetch state: {e}")))?
    } else {
        sqlx::query_as::<_, SyncWaybillState>(
            r#"
            SELECT id, serial_no, driver_id, pit_id, status::text AS status_raw,
                   queue_number, estimated_weight_ton, actual_weight_ton,
                   dispatch_time, arrive_time, version
            FROM waybills
            WHERE (driver_id = $1 OR EXISTS(
                SELECT 1 FROM pits WHERE id = waybills.pit_id AND manager_id = $1
            ))
            AND status NOT IN ('completed', 'cancelled')
            ORDER BY created_at DESC
            "#,
        )
        .bind(operator_id)
        .fetch_all(&state.db)
        .await
        .map_err(|e| ApiError::internal(format!("failed to fetch state: {e}")))?
    };

    Ok(serde_json::json!({
        "waybills": waybills,
        "timestamp": Utc::now(),
    }))
}

/// 客户端获取服务端全量/增量状态
async fn sync_fetch_state(
    State(state): State<AppState>,
    Json(req): Json<SyncStateRequest>,
) -> Result<Json<serde_json::Value>, ApiError> {
    let state = fetch_server_state(&state, &req.operator_id, &req.last_synced_at).await?;
    Ok(Json(state))
}
