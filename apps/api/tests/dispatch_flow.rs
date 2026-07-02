use api::{
    app,
    config::{AiConfig, AppConfig, DispatchMode},
    state::AppState,
};
use axum::{
    body::Body,
    http::{Request, StatusCode},
};
use http_body_util::BodyExt;
use serde_json::{Value, json};
use sqlx::PgPool;
use tower::ServiceExt;

#[sqlx::test(migrations = "../../db/migrations")]
async fn dispatch_flow_completes_waybill(pool: PgPool) {
    seed_user(&pool, "dispatcher", "dispatcher").await;
    seed_user(&pool, "pit-operator", "pit_operator").await;
    seed_user(&pool, "weigh-operator", "weigh_operator").await;

    let state = AppState::for_test(test_config(), pool).await;
    let app = app::build_router(state);

    let driver = post_json(
        &app,
        "/api/v1/drivers",
        json!({
            "name": "测试司机",
            "phone": "13800000001",
            "license_plate": "贵A10001",
            "vehicle_type": "dump_truck",
            "capacity_ton": 35.0
        }),
        StatusCode::OK,
    )
    .await;

    let driver_id = driver["id"].as_str().unwrap().to_string();

    let pit = post_json(
        &app,
        "/api/v1/pits",
        json!({
            "name": "测试1号坑",
            "code": "PIT-T-001",
            "location_text": "贵州测试矿区",
            "queue_capacity": 20
        }),
        StatusCode::OK,
    )
    .await;

    let pit_id = pit["id"].as_str().unwrap().to_string();

    let waybill = post_json(
        &app,
        "/api/v1/waybills",
        json!({
            "driver_id": driver_id,
            "pit_id": pit_id,
            "estimated_weight_ton": 32.5
        }),
        StatusCode::OK,
    )
    .await;

    let waybill_id = waybill["id"].as_str().unwrap().to_string();

    post_json(
        &app,
        &format!("/api/v1/waybills/{waybill_id}/dispatch"),
        json!({ "dispatcher_id": seeded_uuid("dispatcher") }),
        StatusCode::OK,
    )
    .await;

    post_json(
        &app,
        &format!("/api/v1/waybills/{waybill_id}/arrive"),
        json!({ "arrival_source": "driver_app" }),
        StatusCode::OK,
    )
    .await;

    let queue_join = post_json(
        &app,
        &format!("/api/v1/queue/waybills/{waybill_id}/join"),
        json!({
            "driver_id": driver["id"],
            "pit_id": pit["id"],
            "arrival_method": "gps"
        }),
        StatusCode::OK,
    )
    .await;

    assert_eq!(queue_join["status"], "queueing");

    post_json(
        &app,
        &format!("/api/v1/queue/waybills/{waybill_id}/call-next"),
        json!({ "operator_id": seeded_uuid("pit-operator"), "reason": "叫号进场" }),
        StatusCode::OK,
    )
    .await;

    post_json(
        &app,
        &format!("/api/v1/loading/waybills/{waybill_id}/start"),
        json!({
            "operator_id": seeded_uuid("pit-operator"),
            "loader_name": "挖机A",
            "notes": "开始装车"
        }),
        StatusCode::OK,
    )
    .await;

    post_json(
        &app,
        &format!("/api/v1/loading/waybills/{waybill_id}/finish"),
        json!({
            "operator_id": seeded_uuid("pit-operator"),
            "notes": "装车完成"
        }),
        StatusCode::OK,
    )
    .await;

    let weighing = post_json(
        &app,
        &format!("/api/v1/weighing/waybills/{waybill_id}"),
        json!({
            "operator_id": seeded_uuid("weigh-operator"),
            "gross_weight_ton": 48.8,
            "tare_weight_ton": 16.3,
            "net_weight_ton": 32.5,
            "source": "manual",
            "note": "测试过磅"
        }),
        StatusCode::OK,
    )
    .await;

    assert_eq!(weighing["status"], "completed");
    assert_eq!(weighing["net_weight_ton"], 32.5);

    let final_waybill = get_json(
        &app,
        &format!("/api/v1/waybills/{waybill_id}"),
        StatusCode::OK,
    )
    .await;

    assert_eq!(final_waybill["status"], "completed");
    assert_eq!(final_waybill["actual_weight_ton"], 32.5);
}

#[sqlx::test(migrations = "../../db/migrations")]
async fn queue_join_requires_arrived_status(pool: PgPool) {
    let state = AppState::for_test(test_config(), pool).await;
    let app = app::build_router(state);

    let driver = post_json(
        &app,
        "/api/v1/drivers",
        json!({
            "name": "未到场司机",
            "phone": "13800000002",
            "license_plate": "贵A10002",
            "vehicle_type": "dump_truck",
            "capacity_ton": 30.0
        }),
        StatusCode::OK,
    )
    .await;

    let pit = post_json(
        &app,
        "/api/v1/pits",
        json!({
            "name": "测试2号坑",
            "code": "PIT-T-002",
            "location_text": "贵州测试矿区2",
            "queue_capacity": 20
        }),
        StatusCode::OK,
    )
    .await;

    let waybill = post_json(
        &app,
        "/api/v1/waybills",
        json!({
            "driver_id": driver["id"],
            "pit_id": pit["id"],
            "estimated_weight_ton": 30.0
        }),
        StatusCode::OK,
    )
    .await;

    let response = request_json(
        &app,
        "POST",
        &format!("/api/v1/queue/waybills/{}/join", waybill["id"].as_str().unwrap()),
        json!({
            "driver_id": driver["id"],
            "pit_id": pit["id"],
            "arrival_method": "gps"
        }),
    )
    .await;

    assert_eq!(response.0, StatusCode::CONFLICT);
    assert_eq!(response.1["code"], "conflict");
}

fn test_config() -> AppConfig {
    AppConfig {
        host: "127.0.0.1".to_string(),
        port: 3000,
        database_url: "postgres://postgres:postgres@localhost:5432/auto_mining_system".to_string(),
        redis_url: "redis://localhost:6379".to_string(),
        rust_log: "warn".to_string(),
        jwt_secret: "test-secret".to_string(),
        dispatch_mode: DispatchMode::PureAlgorithm,
        ai: AiConfig {
            enabled: false,
            api_url: String::new(),
            api_key: String::new(),
            model: String::new(),
        },
    }
}

/// 生成测试用 JWT token
fn test_token(role: &str) -> String {
    api::middleware::auth::create_token(
        "test-secret",
        uuid::Uuid::new_v4(),
        role,
        "test-user",
    )
    .unwrap()
}

fn seeded_uuid(label: &str) -> String {
    let mut bytes = [0u8; 16];
    for (idx, byte) in label.as_bytes().iter().take(16).enumerate() {
        bytes[idx] = *byte;
    }
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    uuid::Uuid::from_bytes(bytes).to_string()
}

async fn seed_user(pool: &PgPool, label: &str, role: &str) {
    let user_id = uuid::Uuid::parse_str(&seeded_uuid(label)).unwrap();
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, role) \
         VALUES ($1, $2, $3, $4, $5::user_role)",
    )
    .bind(user_id)
    .bind(format!("{label}@test.local"))
    .bind("test-password-hash")
    .bind(label)
    .bind(role)
    .execute(pool)
    .await
    .unwrap();
}

async fn post_json(
    app: &axum::Router,
    path: &str,
    payload: Value,
    expected_status: StatusCode,
) -> Value {
    let (status, body) = request_json(app, "POST", path, payload).await;
    assert_eq!(status, expected_status, "unexpected status for POST {path}: {body}");
    body
}

async fn get_json(
    app: &axum::Router,
    path: &str,
    expected_status: StatusCode,
) -> Value {
    let token = test_token("dispatcher");
    let request = Request::builder()
        .method("GET")
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::empty())
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json = parse_json_body(&body);
    assert_eq!(status, expected_status, "unexpected status for GET {path}: {json}");
    json
}

async fn request_json(
    app: &axum::Router,
    method: &str,
    path: &str,
    payload: Value,
) -> (StatusCode, Value) {
    let token = test_token("dispatcher");
    let request = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json")
        .header("authorization", format!("Bearer {token}"))
        .body(Body::from(payload.to_string()))
        .unwrap();

    let response = app.clone().oneshot(request).await.unwrap();
    let status = response.status();
    let body = response.into_body().collect().await.unwrap().to_bytes();
    let json = parse_json_body(&body);
    (status, json)
}

fn parse_json_body(body: &[u8]) -> Value {
    if body.is_empty() {
        Value::Null
    } else {
        serde_json::from_slice(body).unwrap()
    }
}
