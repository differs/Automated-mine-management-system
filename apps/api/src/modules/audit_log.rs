use sqlx::PgPool;
use uuid::Uuid;

/// 记录操作日志（fire-and-forget，忽略错误）
///
/// 写入 operation_logs 表，用于审计追踪。
/// 所有关键业务操作（状态变更、创建、删除）都应调用此函数。
pub async fn log_operation(
    pool: &PgPool,
    entity_type: &str,
    entity_id: Uuid,
    action: &str,
    before_data: Option<serde_json::Value>,
    after_data: Option<serde_json::Value>,
    operator_id: Option<Uuid>,
    operator_name: Option<&str>,
    reason: Option<&str>,
) {
    let _ = sqlx::query(
        "INSERT INTO operation_logs \
         (entity_type, entity_id, action, before_data, after_data, operator_id, operator_name, reason) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(entity_type)
    .bind(entity_id)
    .bind(action)
    .bind(&before_data)
    .bind(&after_data)
    .bind(operator_id)
    .bind(operator_name)
    .bind(reason)
    .execute(pool)
    .await;
}

/// 运单操作日志快捷方法
pub async fn log_waybill_operation(
    pool: &PgPool,
    waybill_id: Uuid,
    action: &str,
    before_status: Option<&str>,
    after_status: Option<&str>,
    operator_id: Option<Uuid>,
    reason: Option<&str>,
) {
    let before = before_status.map(|s| serde_json::json!({ "status": s }));
    let after = after_status.map(|s| serde_json::json!({ "status": s }));
    log_operation(
        pool,
        "waybill",
        waybill_id,
        action,
        before,
        after,
        operator_id,
        None,
        reason,
    )
    .await;
}
