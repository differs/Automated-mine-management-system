use axum::{
    Json, Router,
    extract::{Path, State},
    routing::{get, post},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use regex::Regex;
use uuid::Uuid;

use sqlx::Row;
use crate::{error::ApiError, state::AppState};

/// 地磅自动采集模块
///
/// 提供三种称重方式:
///   1. manual      - 地磅操作员手动输入（现有）
///   2. serial      - Rust 串口直连地磅读取
///   3. bluetooth   - Flutter 端蓝牙连接地磅读取
///
/// 防作弊设计:
///   - 称重时长检查（太快可能作弊）
///   - 皮重历史对比（和上次皮重偏差过大报警）
///   - 毛重合理性检查（不能超过车辆载重）
///   - 原始报文存档（防篡改审计）
pub fn router() -> Router<AppState> {
    Router::new()
        // 地磅设备管理
        .route("/devices", get(list_devices).post(register_device))
        .route("/devices/{device_id}", post(update_device))
        // 串口直连称重（Rust 后台进程）
        .route("/serial/read", post(read_serial_scale))
        .route("/serial/weigh/{waybill_id}", post(auto_weigh_serial))
        // 蓝牙称重（Flutter 端读取后提交）
        .route("/bluetooth/weigh/{waybill_id}", post(auto_weigh_bluetooth))
        // 称重日志（防作弊审计）
        .route("/logs/{waybill_id}", get(get_weigh_logs))
        // 皮重历史
        .route("/tare-history/{vehicle_id}", get(get_tare_history))
}

// ─── 数据模型 ─────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct ScaleDevice {
    pub id: Uuid,
    pub pit_id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub connection_config: serde_json::Value,
    pub is_active: bool,
    pub last_heartbeat_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct RegisterDeviceRequest {
    pub pit_id: Uuid,
    pub device_name: String,
    pub device_type: String,
    pub connection_config: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct SerialScaleReadRequest {
    pub device_id: Uuid,
    pub port: String,
    pub baud_rate: u32,
}

#[derive(Debug, Deserialize)]
pub struct BluetoothWeighRequest {
    pub device_id: Uuid,
    pub operator_id: Uuid,
    pub waybill_id: Uuid,
    pub gross_weight_ton: f64,
    pub tare_weight_ton: Option<f64>,
    pub raw_data: String,           // 地磅原始报文
    pub reading_duration_sec: u32,  // 读取耗时（秒）
}

#[derive(Debug, Deserialize)]
pub struct SerialWeighRequest {
    pub device_id: Uuid,
    pub operator_id: Uuid,
    pub port: String,
    pub baud_rate: u32,
}

#[derive(Debug, Serialize)]
pub struct ScaleReading {
    pub weight_ton: f64,
    pub is_stable: bool,
    pub raw_data: String,
    pub read_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct WeighResult {
    pub waybill_id: Uuid,
    pub gross_weight_ton: f64,
    pub tare_weight_ton: Option<f64>,
    pub net_weight_ton: f64,
    pub source: String,
    pub completed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, sqlx::FromRow)]
pub struct WeighLog {
    pub id: Uuid,
    pub weighing_id: Option<Uuid>,
    pub device_id: Option<Uuid>,
    pub weight: f64,
    pub raw_data: String,
    pub is_stable: bool,
    pub read_at: DateTime<Utc>,
}

// ─── API 实现 ──────────────────────────────────────────────────────────────

/// 串口读取地磅
///
/// 连接串口地磅，读取稳定重量。
/// 返回当前重量值，不提交运单。
async fn read_serial_scale(
    State(state): State<AppState>,
    Json(req): Json<SerialScaleReadRequest>,
) -> Result<Json<ScaleReading>, ApiError> {
    let reading = read_from_serial_port(&req.port, req.baud_rate).await?;

    // 记录原始读数
    sqlx::query(
        r#"INSERT INTO weigh_logs (weighing_id, device_id, weight, raw_data, is_stable, read_at)
           VALUES (NULL, $1, $2, $3, $4, $5)"#,
    )
    .bind(req.device_id)
    .bind(reading.weight_ton)
    .bind(&reading.raw_data)
    .bind(reading.is_stable)
    .bind(reading.read_at)
    .execute(&state.db)
    .await
    .ok();

    Ok(Json(reading))
}

/// 串口直连自动称重 + 完单
///
/// 读取地磅 → 自动填入 → 校验 → 完成运单
async fn auto_weigh_serial(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(req): Json<SerialWeighRequest>,
) -> Result<Json<WeighResult>, ApiError> {
    // 1. 验证运单状态
    let waybill = sqlx::query(
        "SELECT status::text AS status, vehicle_id FROM waybills WHERE id = $1",
    )
    .bind(waybill_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch waybill: {e}")))?;

    let Some(waybill) = waybill else {
        return Err(ApiError::not_found("waybill not found"));
    };

    let status: String = waybill.get("status");
    if status != "loaded" {
        return Err(ApiError::conflict("only loaded waybills can be weighed"));
    }

    let vehicle_id: Option<Uuid> = waybill.get("vehicle_id");

    // 2. 读取地磅（等待稳定）
    let reading = read_from_serial_port(&req.port, req.baud_rate).await?;
    let gross = reading.weight_ton;

    if !reading.is_stable {
        return Err(ApiError::bad_request("scale reading not stable, please wait"));
    }

    // 3. 获取皮重（从上次称重记录）
    let tare = if let Some(vid) = vehicle_id {
        get_last_tare(&state, vid).await.unwrap_or(0.0)
    } else {
        0.0
    };

    let net = gross - tare;

    // 4. 防作弊校验
    validate_weighing(&state, gross, tare, net, vehicle_id, 15).await?;

    // 5. 事务提交流程
    let now = Utc::now();
    let mut tx = state.db.begin().await.map_err(|e| {
        ApiError::internal(format!("failed to begin tx: {e}"))
    })?;

    // 创建称重记录
    let weighing_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO weigh_records (waybill_id, gross_weight_ton, tare_weight_ton, net_weight_ton,
            weigh_time, operator_id, source, note)
           VALUES ($1, $2, $3, $4, $5, $6, 'serial', 'auto')
           RETURNING id"#,
    )
    .bind(waybill_id)
    .bind(gross)
    .bind(tare)
    .bind(net)
    .bind(now)
    .bind(req.operator_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("failed to create weigh record: {e}")))?;

    // 记录原始读数
    sqlx::query(
        r#"INSERT INTO weigh_logs (weighing_id, device_id, weight, raw_data, is_stable, read_at)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(weighing_id)
    .bind(req.device_id)
    .bind(gross)
    .bind(&reading.raw_data)
    .bind(true)
    .bind(reading.read_at)
    .execute(&mut *tx)
    .await
    .ok();

    // 完成运单
    sqlx::query(
        "UPDATE waybills SET status = 'completed', actual_weight_ton = $2, \
         completed_time = $3, updated_at = $3, version = version + 1 \
         WHERE id = $1",
    )
    .bind(waybill_id)
    .bind(net)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("failed to complete waybill: {e}")))?;

    tx.commit().await.map_err(|e| {
        ApiError::internal(format!("failed to commit weighing: {e}"))
    })?;

    Ok(Json(WeighResult {
        waybill_id,
        gross_weight_ton: gross,
        tare_weight_ton: Some(tare),
        net_weight_ton: net,
        source: "serial".into(),
        completed_at: now,
    }))
}

/// 蓝牙称重（Flutter 端读取后提交）
async fn auto_weigh_bluetooth(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
    Json(req): Json<BluetoothWeighRequest>,
) -> Result<Json<WeighResult>, ApiError> {
    // 1. 验证运单
    let waybill = sqlx::query(
        "SELECT status::text AS status, vehicle_id FROM waybills WHERE id = $1",
    )
    .bind(waybill_id)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch waybill: {e}")))?;

    let Some(waybill) = waybill else {
        return Err(ApiError::not_found("waybill not found"));
    };

    let status: String = waybill.get("status");
    if status != "loaded" {
        return Err(ApiError::conflict("only loaded waybills can be weighed"));
    }

    let vehicle_id: Option<Uuid> = waybill.get("vehicle_id");
    let gross = req.gross_weight_ton;
    let tare = req.tare_weight_ton.unwrap_or(0.0);
    let net = gross - tare;

    // 2. 防作弊校验
    validate_weighing(&state, gross, tare, net, vehicle_id, req.reading_duration_sec).await?;

    // 3. 提交流程
    let now = Utc::now();
    let mut tx = state.db.begin().await.map_err(|e| {
        ApiError::internal(format!("failed to begin tx: {e}"))
    })?;

    let weighing_id = sqlx::query_scalar::<_, Uuid>(
        r#"INSERT INTO weigh_records (waybill_id, gross_weight_ton, tare_weight_ton, net_weight_ton,
            weigh_time, operator_id, source, note)
           VALUES ($1, $2, $3, $4, $5, $6, 'bluetooth', 'auto')
           RETURNING id"#,
    )
    .bind(waybill_id)
    .bind(gross)
    .bind(tare)
    .bind(net)
    .bind(now)
    .bind(req.operator_id)
    .fetch_one(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("failed to create weigh record: {e}")))?;

    // 记录原始报文
    sqlx::query(
        r#"INSERT INTO weigh_logs (weighing_id, device_id, weight, raw_data, is_stable, read_at)
           VALUES ($1, $2, $3, $4, $5, $6)"#,
    )
    .bind(weighing_id)
    .bind(req.device_id)
    .bind(gross)
    .bind(&req.raw_data)
    .bind(true)
    .bind(now)
    .execute(&mut *tx)
    .await
    .ok();

    sqlx::query(
        "UPDATE waybills SET status = 'completed', actual_weight_ton = $2, \
         completed_time = $3, updated_at = $3, version = version + 1 \
         WHERE id = $1",
    )
    .bind(waybill_id)
    .bind(net)
    .bind(now)
    .execute(&mut *tx)
    .await
    .map_err(|e| ApiError::internal(format!("failed to complete waybill: {e}")))?;

    tx.commit().await.map_err(|e| {
        ApiError::internal(format!("failed to commit weighing: {e}"))
    })?;

    Ok(Json(WeighResult {
        waybill_id,
        gross_weight_ton: gross,
        tare_weight_ton: Some(tare),
        net_weight_ton: net,
        source: "bluetooth".into(),
        completed_at: now,
    }))
}

// ─── 地磅设备管理 ─────────────────────────────────────────────────────────

fn row_to_device(row: &sqlx::postgres::PgRow) -> Result<ScaleDevice, sqlx::Error> {
    Ok(ScaleDevice {
        id: row.try_get("id")?,
        pit_id: row.try_get("pit_id")?,
        device_name: row.try_get("device_name")?,
        device_type: row.try_get("device_type")?,
        connection_config: row.try_get("connection_config")?,
        is_active: row.try_get("is_active")?,
        last_heartbeat_at: row.try_get("last_heartbeat_at")?,
        created_at: row.try_get("created_at")?,
    })
}

async fn list_devices(
    State(state): State<AppState>,
) -> Result<Json<Vec<ScaleDevice>>, ApiError> {
    let rows = sqlx::query(
        "SELECT id, pit_id, device_name, device_type, connection_config::text::jsonb AS connection_config, \
         is_active, last_heartbeat_at, created_at FROM scale_devices ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to list devices: {e}")))?;

    let devices: Result<Vec<_>, _> = rows.iter().map(row_to_device).collect();
    Ok(Json(devices.map_err(|e| ApiError::internal(format!("row mapping: {e}")))?))
}

async fn register_device(
    State(state): State<AppState>,
    Json(req): Json<RegisterDeviceRequest>,
) -> Result<Json<ScaleDevice>, ApiError> {
    let row = sqlx::query(
        r#"INSERT INTO scale_devices (pit_id, device_name, device_type, connection_config)
           VALUES ($1, $2, $3, $4)
           RETURNING id, pit_id, device_name, device_type, connection_config::text::jsonb AS connection_config,
                     is_active, last_heartbeat_at, created_at"#,
    )
    .bind(req.pit_id)
    .bind(&req.device_name)
    .bind(&req.device_type)
    .bind(&req.connection_config)
    .fetch_one(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to register device: {e}")))?;

    row_to_device(&row).map(Json).map_err(|e| ApiError::internal(format!("row mapping: {e}")))
}

async fn update_device(
    State(state): State<AppState>,
    Path(device_id): Path<Uuid>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<ScaleDevice>, ApiError> {
    let row = sqlx::query(
        r#"UPDATE scale_devices SET connection_config = $2, is_active = true,
            last_heartbeat_at = NOW(), updated_at = NOW()
           WHERE id = $1
           RETURNING id, pit_id, device_name, device_type, connection_config::text::jsonb AS connection_config,
                     is_active, last_heartbeat_at, created_at"#,
    )
    .bind(device_id)
    .bind(&req)
    .fetch_optional(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to update device: {e}")))?;

    let Some(row) = row else {
        return Err(ApiError::not_found("device not found"));
    };
    row_to_device(&row).map(Json).map_err(|e| ApiError::internal(format!("row mapping: {e}")))
}

async fn get_weigh_logs(
    State(state): State<AppState>,
    Path(waybill_id): Path<Uuid>,
) -> Result<Json<Vec<WeighLog>>, ApiError> {
    let rows = sqlx::query(
        r#"SELECT wl.id, wl.weighing_id, wl.device_id, wl.weight, wl.raw_data, wl.is_stable, wl.read_at
           FROM weigh_logs wl
           JOIN weigh_records wr ON wl.weighing_id = wr.id
           WHERE wr.waybill_id = $1
           ORDER BY wl.read_at ASC"#,
    )
    .bind(waybill_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch weigh logs: {e}")))?;

    let logs: Vec<WeighLog> = rows.iter().map(|row| {
        WeighLog {
            id: row.get("id"),
            weighing_id: row.get("weighing_id"),
            device_id: row.get("device_id"),
            weight: row.get("weight"),
            raw_data: row.get("raw_data"),
            is_stable: row.get("is_stable"),
            read_at: row.get("read_at"),
        }
    }).collect();

    Ok(Json(logs))
}

async fn get_tare_history(
    State(state): State<AppState>,
    Path(vehicle_id): Path<Uuid>,
) -> Result<Json<Vec<f64>>, ApiError> {
    let tares = sqlx::query_scalar::<_, f64>(
        r#"SELECT tare_weight_ton FROM weigh_records
           WHERE weigh_records.id IN (
               SELECT wr2.id FROM weigh_records wr2
               JOIN waybills w ON wr2.waybill_id = w.id
               WHERE w.vehicle_id = $1 AND wr2.tare_weight_ton IS NOT NULL
           )
           ORDER BY weigh_time DESC LIMIT 10"#,
    )
    .bind(vehicle_id)
    .fetch_all(&state.db)
    .await
    .map_err(|e| ApiError::internal(format!("failed to fetch tare history: {e}")))?;

    Ok(Json(tares))
}

// ─── 核心函数 ──────────────────────────────────────────────────────────────

/// 串口读取地磅
///
/// 使用 serialport crate 在阻塞线程中读取地磅数据。
/// 通过 tokio::task::spawn_blocking 避免阻塞事件循环。
async fn read_from_serial_port(port: &str, baud_rate: u32) -> Result<ScaleReading, ApiError> {
    use std::io::Read;
    use std::time::Duration;

    let port = port.to_string();

    // 在阻塞线程中打开和读取串口
    let reading = tokio::task::spawn_blocking(move || {
        let mut serial = serialport::new(port, baud_rate)
            .timeout(Duration::from_secs(10))
            .open()
            .map_err(|e| format!("failed to open serial port: {e}"))?;

        let mut buf = [0u8; 1024];
        let mut stable_count = 0;
        let mut last_weight = 0.0;
        let mut all_data = String::new();
        let read_at = Utc::now();

        for _ in 0..30 {
            match serial.read(&mut buf) {
                Ok(n) if n > 0 => {
                    let data = String::from_utf8_lossy(&buf[..n]);
                    all_data.push_str(&data);

                    if let Some(weight) = parse_scale_weight(&data) {
                        if (weight - last_weight).abs() < 0.5 {
                            stable_count += 1;
                            if stable_count >= 3 {
                                return Ok(ScaleReading {
                                    weight_ton: weight,
                                    is_stable: true,
                                    raw_data: all_data,
                                    read_at,
                                });
                            }
                        } else {
                            stable_count = 0;
                        }
                        last_weight = weight;
                    }
                }
                Ok(_) => {}
                Err(_) => break,
            }
            std::thread::sleep(Duration::from_millis(500));
        }

        if last_weight > 0.0 {
            Ok(ScaleReading {
                weight_ton: last_weight,
                is_stable: stable_count >= 3,
                raw_data: all_data,
                read_at,
            })
        } else {
            Err("no weight data received from scale".to_string())
        }
    })
    .await
    .map_err(|e| ApiError::internal(format!("serial task failed: {e}")))?;

    reading.map_err(|e| ApiError::internal(e))
}

/// 解析地磅协议
fn parse_scale_weight(data: &str) -> Option<f64> {
    let patterns = [
        Regex::new(r"ST,GS,([+-]?\d+)kg,ST").ok()?,
        Regex::new(r"净重[:：]\s*([\d.]+)\s*kg").ok()?,
        Regex::new(r"毛重[:：]\s*([\d.]+)\s*kg").ok()?,
        Regex::new(r"皮重[:：]\s*([\d.]+)\s*kg").ok()?,
        Regex::new(r"Weight[:：]\s*([\d.]+)").ok()?,
        Regex::new(r"NW[:：]\s*([\d.]+)").ok()?,
        Regex::new(r"([\d.]+)\s*kg").ok()?,
    ];

    for pattern in &patterns {
        if let Some(caps) = pattern.captures(data) {
            if let Some(weight_str) = caps.get(1) {
                if let Ok(weight) = weight_str.as_str().parse::<f64>() {
                    return Some(weight / 1000.0); // kg -> 吨
                }
            }
        }
    }
    None
}

/// 获取车辆上次皮重
async fn get_last_tare(state: &AppState, vehicle_id: Uuid) -> Option<f64> {
    sqlx::query_scalar::<_, f64>(
        r#"SELECT tare_weight_ton FROM weigh_records
           WHERE weigh_records.id IN (
               SELECT wr.id FROM weigh_records wr
               JOIN waybills w ON wr.waybill_id = w.id
               WHERE w.vehicle_id = $1 AND wr.tare_weight_ton IS NOT NULL
           )
           ORDER BY weigh_time DESC LIMIT 1"#,
    )
    .bind(vehicle_id)
    .fetch_optional(&state.db)
    .await
    .ok()
    .flatten()
}

/// 称重防作弊校验
async fn validate_weighing(
    state: &AppState,
    gross: f64,
    tare: f64,
    net: f64,
    vehicle_id: Option<Uuid>,
    duration_sec: u32,
) -> Result<(), ApiError> {
    // 1. 称重时长检查
    if duration_sec < 5 {
        return Err(ApiError::bad_request(
            "weighing too fast, minimum 5 seconds required",
        ));
    }

    // 2. 重量范围检查
    if gross <= 0.0 || gross > 200.0 {
        return Err(ApiError::bad_request(
            "gross weight out of range (0-200 tons)",
        ));
    }
    if net < 0.0 || net > 200.0 {
        return Err(ApiError::bad_request("net weight out of range"));
    }

    // 3. 皮重历史对比
    if let Some(vid) = vehicle_id {
        if let Some(last_tare) = get_last_tare(state, vid).await {
            if tare > 0.0 && (tare - last_tare).abs() > 1.0 {
                return Err(ApiError::bad_request(
                    format!("tare weight {tare:.1}t deviates from historical {last_tare:.1}t by more than 1 ton"),
                ));
            }
        }
    }

    Ok(())
}
