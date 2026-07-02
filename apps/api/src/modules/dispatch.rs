use axum::{
    Json, Router,
    extract::State,
    routing::get,
};
use serde::{Deserialize, Serialize};
use sqlx::Row;

use crate::{config::DispatchMode, error::ApiError, state::AppState};
use super::ai::{
    AiDispatchEngine, AlgorithmDispatchEngine, DispatchPlan,
    PendingWaybill, PitStats, DriverStats, WpmaWeights,
};

/// 智能调度推荐模块
///
/// 根据运行时调度模式（纯算法 / AI增强）生成推荐方案。
/// - 纯算法模式：直接使用 WPMA 算法
/// - AI增强模式：先用 WPMA 生成基础方案，再调用盘古大模型优化
pub fn router() -> Router<AppState> {
    Router::new()
        .route("/recommendations", get(get_dispatch_recommendations))
}

#[derive(Deserialize)]
pub struct DispatchQuery {
    /// 推荐数量（默认 5）
    pub top_n: Option<usize>,
}

#[derive(Serialize)]
pub struct DispatchResponse {
    pub plan: DispatchPlan,
    pub mode: String,
    pub ai_enhanced: bool,
}

/// 获取智能调度推荐
///
/// 1. 查询所有待派运单（status = pending_dispatch）
/// 2. 查询所有空闲司机（status = idle）
/// 3. 查询所有坑口统计（队列长度、优先级）
/// 4. 根据运行时模式选择算法引擎生成推荐
async fn get_dispatch_recommendations(
    State(state): State<AppState>,
    axum::extract::Query(query): axum::extract::Query<DispatchQuery>,
) -> Result<Json<DispatchResponse>, ApiError> {
    let top_n = query.top_n.unwrap_or(5).min(20);

    // 1. 查询待派运单
    let waybill_rows = sqlx::query(
        r#"SELECT id, driver_id, pit_id,
                  (SELECT name FROM pits WHERE id = w.pit_id) AS pit_name,
                  created_at
           FROM waybills w
           WHERE status = 'pending_dispatch'
           ORDER BY created_at ASC
           LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to query pending waybills: {e}")))?;

    let pending_waybills: Vec<PendingWaybill> = waybill_rows.iter().map(|row| {
        PendingWaybill {
            waybill_id: row.get::<uuid::Uuid, _>("id").to_string(),
            driver_id: row.get::<uuid::Uuid, _>("driver_id").to_string(),
            pit_id: row.get::<uuid::Uuid, _>("pit_id").to_string(),
            pit_name: row.get::<Option<String>, _>("pit_name").unwrap_or_default(),
            created_at: row.get("created_at"),
        }
    }).collect();

    if pending_waybills.is_empty() {
        return Ok(Json(DispatchResponse {
            plan: DispatchPlan {
                recommendations: vec![],
                total_pending: 0,
                total_idle_drivers: 0,
                algorithm_version: "wpma-1.0".to_string(),
            },
            mode: "pure_algorithm".to_string(),
            ai_enhanced: false,
        }));
    }

    // 2. 查询空闲司机及其今日统计
    let driver_rows = sqlx::query(
        r#"SELECT d.id, d.name,
                  CASE WHEN w.last_active_at IS NOT NULL
                       THEN EXTRACT(EPOCH FROM (NOW() - w.last_active_at)) / 60
                       ELSE 999 END AS idle_minutes,
                  COALESCE(t.trips_today, 0) AS trips_today,
                  w.current_pit_id
           FROM drivers d
           LEFT JOIN LATERAL (
               SELECT MAX(updated_at) AS last_active_at,
                      (SELECT pit_id FROM waybills WHERE driver_id = d.id AND status = 'queueing' LIMIT 1) AS current_pit_id
               FROM waybills WHERE driver_id = d.id AND status IN ('arrived', 'queueing', 'loading')
           ) w ON TRUE
           LEFT JOIN LATERAL (
               SELECT COUNT(*)::bigint AS trips_today
               FROM waybills
               WHERE driver_id = d.id
                 AND status = 'completed'
                 AND completed_time::date = CURRENT_DATE
           ) t ON TRUE
           WHERE d.is_active = TRUE
             AND d.status = 'idle'
             AND NOT EXISTS (
                 SELECT 1 FROM waybills
                 WHERE driver_id = d.id
                   AND status IN ('pending_dispatch', 'dispatched', 'arrived', 'queueing', 'loading', 'loaded', 'weighing')
             )
           ORDER BY idle_minutes DESC
           LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to query idle drivers: {e}")))?;

    let idle_drivers: Vec<DriverStats> = driver_rows.iter().map(|row| {
        DriverStats {
            driver_id: row.get::<uuid::Uuid, _>("id").to_string(),
            driver_name: row.get::<String, _>("name"),
            idle_minutes: row.get::<Option<f64>, _>("idle_minutes").unwrap_or(999.0) as i64,
            trips_today: row.get::<Option<i64>, _>("trips_today").unwrap_or(0),
            current_pit_id: row.get::<Option<uuid::Uuid>, _>("current_pit_id").map(|id| id.to_string()),
        }
    }).collect();

    // 3. 查询坑口统计
    let pit_rows = sqlx::query(
        r#"SELECT p.id, p.name, p.current_queue_count AS queue_length,
                  COALESCE(p.queue_priority, 1) AS priority,
                  COALESCE(p.queue_capacity, 20) AS max_capacity
           FROM pits p
           WHERE p.is_active = TRUE
           ORDER BY p.name"#,
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to query pit stats: {e}")))?;

    let pit_stats: Vec<PitStats> = pit_rows.iter().map(|row| {
        PitStats {
            pit_id: row.get::<uuid::Uuid, _>("id").to_string(),
            pit_name: row.get::<String, _>("name"),
            queue_length: row.get::<i32, _>("queue_length"),
            priority: row.get::<Option<i32>, _>("priority").unwrap_or(1),
            max_capacity: row.get::<Option<i32>, _>("max_capacity").unwrap_or(20),
        }
    }).collect();

    // 4. 根据运行时模式选择算法引擎
    let mode = state.dispatch_mode.read().await.clone();
    let weights = WpmaWeights::default();

    let (plan, ai_enhanced) = match &mode {
        DispatchMode::AiEnhanced if state.config.ai.enabled && !state.config.ai.api_key.is_empty() => {
            // AI增强模式：先用 WPMA 生成基础方案，再尝试 AI 优化
            let base_plan = AlgorithmDispatchEngine::dispatch(
                &pending_waybills, &idle_drivers, &pit_stats, &weights, top_n,
            );

            // 尝试用 AI 优化每个推荐
            let ai_engine = AiDispatchEngine::new(state.config.ai.clone());
            let mut enhanced_plan = base_plan;

            for rec in &mut enhanced_plan.recommendations {
                let queue_len = pit_stats.iter()
                    .find(|p| p.pit_id == rec.pit_id)
                    .map(|p| p.queue_length)
                    .unwrap_or(0);

                match ai_engine.optimize_dispatch(
                    &rec.waybill_id, &rec.driver_id, &rec.pit_id,
                    queue_len, idle_drivers.len() as i32,
                ).await {
                    Ok(ai_suggestion) => {
                        if ai_suggestion.trim() != "OK" {
                            rec.reason = format!("AI建议: {}", ai_suggestion);
                        }
                        tracing::debug!("AI optimization for {}: {}", rec.waybill_id, ai_suggestion);
                    }
                    Err(e) => {
                        tracing::warn!("AI optimization failed for {}: {e}", rec.waybill_id);
                        // AI 失败时保持原推荐不变
                    }
                }
            }

            enhanced_plan.algorithm_version = "wpma-1.0+ai-pangu".to_string();
            (enhanced_plan, true)
        }
        _ => {
            // 纯算法模式
            let plan = AlgorithmDispatchEngine::dispatch(
                &pending_waybills, &idle_drivers, &pit_stats, &weights, top_n,
            );
            (plan, false)
        }
    };

    let mode_str = mode.as_str().to_string();

    Ok(Json(DispatchResponse {
        plan,
        mode: mode_str,
        ai_enhanced,
    }))
}
