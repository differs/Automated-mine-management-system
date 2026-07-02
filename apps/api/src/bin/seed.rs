use api::{config::AppConfig, state::AppState};
use bcrypt::DEFAULT_COST;
use uuid::Uuid;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = AppConfig::from_env();
    let state = AppState::bootstrap(config).await?;

    println!("seeding database...");

    // Create admin user (password: admin123)
    let admin_id = Uuid::new_v4();
    let admin_hash = bcrypt::hash("admin123", DEFAULT_COST)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, role) \
         VALUES ($1, 'admin', $2, '系统管理员', 'super_admin') \
         ON CONFLICT (username) DO NOTHING",
    )
    .bind(admin_id)
    .bind(&admin_hash)
    .execute(&state.db)
    .await?;
    println!("  created admin user (password: admin123)");

    // Create dispatcher user
    let dispatcher_hash = bcrypt::hash("disp123", DEFAULT_COST)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, role) \
         VALUES ($1, 'dispatcher', $2, '调度员小王', 'dispatcher') \
         ON CONFLICT (username) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(&dispatcher_hash)
    .execute(&state.db)
    .await?;
    println!("  created dispatcher user (password: disp123)");

    // Create pit operator
    let pitop_hash = bcrypt::hash("pit123", DEFAULT_COST)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, role) \
         VALUES ($1, 'pitop', $2, '坑口管理员老李', 'pit_operator') \
         ON CONFLICT (username) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(&pitop_hash)
    .execute(&state.db)
    .await?;
    println!("  created pit operator (password: pit123)");

    // Create weigh operator
    let weigh_hash = bcrypt::hash("weigh123", DEFAULT_COST)?;
    sqlx::query(
        "INSERT INTO users (id, username, password_hash, display_name, role) \
         VALUES ($1, 'weighop', $2, '地磅员小赵', 'weigh_operator') \
         ON CONFLICT (username) DO NOTHING",
    )
    .bind(Uuid::new_v4())
    .bind(&weigh_hash)
    .execute(&state.db)
    .await?;
    println!("  created weigh operator (password: weigh123)");

    // Create pits
    let pits = vec![
        ("1号采区", "PIT-001", "矿区东侧", 15),
        ("2号采区", "PIT-002", "矿区西侧", 12),
        ("3号采区", "PIT-003", "矿区南侧", 10),
        ("4号采区", "PIT-004", "矿区北侧", 8),
    ];
    for (name, code, location, capacity) in &pits {
        sqlx::query(
            "INSERT INTO pits (name, code, location_text, queue_capacity) \
             VALUES ($1, $2, $3, $4) ON CONFLICT (name) DO NOTHING",
        )
        .bind(name)
        .bind(code)
        .bind(location)
        .bind(capacity)
        .execute(&state.db)
        .await?;
    }
    println!("  created {} pits", pits.len());

    // Create sample drivers
    let drivers = vec![
        ("张大山", "13800001001", "贵A10001", "dump_truck", 30.0),
        ("李四牛", "13800001002", "贵A10002", "dump_truck", 35.0),
        ("王老五", "13800001003", "贵A10003", "dump_truck", 30.0),
        ("赵铁柱", "13800001004", "贵A10004", "trailer", 40.0),
        ("陈大力", "13800001005", "贵A10005", "dump_truck", 30.0),
        ("刘强东", "13800001006", "贵A10006", "dump_truck", 35.0),
        ("孙大圣", "13800001007", "贵A10007", "trailer", 40.0),
        ("周武松", "13800001008", "贵A10008", "dump_truck", 30.0),
        ("吴用仁", "13800001009", "贵A10009", "dump_truck", 35.0),
        ("郑成功", "13800001010", "贵A10010", "dump_truck", 30.0),
    ];
    for (name, phone, plate, vtype, capacity) in &drivers {
        sqlx::query(
            "INSERT INTO drivers (name, phone, license_plate, vehicle_type, capacity_ton) \
             VALUES ($1, $2, $3, $4::vehicle_type, $5) \
             ON CONFLICT (phone) DO NOTHING",
        )
        .bind(name)
        .bind(phone)
        .bind(plate)
        .bind(vtype)
        .bind(capacity)
        .execute(&state.db)
        .await?;
    }
    println!("  created {} drivers", drivers.len());

    // Create shifts
    let shifts = vec![
        ("早班", "SHIFT-AM", "08:00", "20:00", false),
        ("晚班", "SHIFT-PM", "20:00", "08:00", true),
    ];
    for (name, code, start, end, crosses) in &shifts {
        sqlx::query(
            "INSERT INTO shifts (name, code, starts_at, ends_at, crosses_day) \
             VALUES ($1, $2, $3::time, $4::time, $5) ON CONFLICT (code) DO NOTHING",
        )
        .bind(name)
        .bind(code)
        .bind(start)
        .bind(end)
        .bind(crosses)
        .execute(&state.db)
        .await?;
    }
    println!("  created shifts");

    println!("seed complete!");
    Ok(())
}
