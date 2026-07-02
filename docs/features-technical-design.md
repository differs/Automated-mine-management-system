# 关键技术方案设计

> 五个急需功能的实现方案 + 无人矿卡趋势应对

---

## 一、⭐⭐⭐ 离线调度（Offline Dispatch）

### 问题

矿山采场、坑口附近 4G/5G 信号弱甚至无信号，司机和坑口管理员无法实时联网。如果系统必须在线才能用，现场根本跑不起来。

### 设计思路

**核心原则：离线可操作，在线自动同步，冲突由服务端裁决。**

### 整体架构

```text
                     ┌──────────────┐
                     │   云服务端    │
                     │  (权威数据源) │
                     └──────┬───────┘
                            │ 在线时同步
              ┌─────────────┼─────────────┐
              │             │             │
         ┌────┴────┐  ┌────┴────┐  ┌────┴────┐
         │司机端    │  │坑口端   │  │调度后台  │
         │Flutter   │  │Flutter  │  │Web      │
         │+ SQLite  │  │+ SQLite │  │(始终在线)│
         └─────────┘  └─────────┘  └─────────┘
```

### 移动端离线架构

```
┌─────────────────────────────────────┐
│            Flutter App              │
│  ┌───────────────────────────────┐  │
│  │       UI Layer (正常使用)      │  │
│  ├───────────────────────────────┤  │
│  │    Offline Manager (核心)     │  │
│  │  ┌─────────┐  ┌───────────┐   │  │
│  │  │ 队列管理  │  │ 冲突检测   │   │  │
│  │  └────┬────┘  └─────┬─────┘   │  │
│  ├───────┴─────────────┴────────┤  │
│  │   Sync Engine (同步引擎)      │  │
│  │  ┌──────────┐ ┌───────────┐   │  │
│  │  │ 本地SQLite│ │ 变更日志   │   │  │
│  │  └──────────┘ └───────────┘   │  │
│  ├───────────────────────────────┤  │
│  │   Connectivity Monitor        │  │
│  │   (检测网络状态)               │  │
│  └───────────────────────────────┘  │
└─────────────────────────────────────┘
```

### 关键设计细节

#### 1. 本地数据库（SQLite）

每个移动端（司机/坑口）维护一个本地 SQLite 数据库：

```sql
-- 本地运单缓存
CREATE TABLE local_waybills (
    id TEXT PRIMARY KEY,          -- UUID
    waybill_no TEXT,
    pit_id TEXT,
    driver_id TEXT,
    status TEXT,                   -- 本地状态
    server_status TEXT,            -- 服务端最新状态
    data JSON,                     -- 完整运单数据（JSON）
    last_synced_at TEXT,           -- 最后同步时间
    updated_at TEXT
);

-- 离线操作队列（待同步）
CREATE TABLE offline_operations (
    id TEXT PRIMARY KEY,
    operation_type TEXT,           -- arrive/start_loading/finish_loading/weigh
    waybill_id TEXT,
    payload JSON,                  -- 操作数据
    created_at TEXT,               -- 本地创建时间
    retry_count INTEGER DEFAULT 0,
    status TEXT DEFAULT 'pending'  -- pending/syncing/synced/conflicted
);

-- 冲突日志
CREATE TABLE sync_conflicts (
    id TEXT PRIMARY KEY,
    waybill_id TEXT,
    local_data JSON,
    server_data JSON,
    resolution TEXT,               -- local_wins/server_wins/manual
    resolved_at TEXT
);
```

#### 2. 同步引擎（Sync Engine）

```dart
// Flutter 端伪代码
class SyncEngine {
  final ConnectivityMonitor _connectivity;
  final LocalDatabase _local;
  final ApiClient _api;
  final ConflictResolver _resolver;

  // 启动同步循环
  Future<void> start() async {
    _connectivity.onOnline.listen((_) => _sync());
  }

  // 同步流程
  Future<void> _sync() async {
    // 1. 获取待同步操作（按时间排序）
    final pending = await _local.getPendingOperations();

    for (final op in pending) {
      try {
        // 2. 提交到服务端（幂等）
        final result = await _api.submitOperation(op);

        if (result.success) {
          // 3. 标记已同步
          await _local.markSynced(op.id);
          // 4. 更新本地数据
          await _local.updateWaybill(op.waybillId, result.data);
        }
      } on ConflictException catch (e) {
        // 5. 冲突处理
        await _resolver.resolve(op, e.serverState);
      } on NetworkException {
        // 6. 网络断开，下次继续
        break;
      }
    }

    // 7. 拉取服务端最新状态（全量或增量）
    final serverWaybills = await _api.getActiveWaybills();
    await _local.mergeServerState(serverWaybills);
  }
}
```

#### 3. 服务端幂等控制

```rust
// Rust 服务端伪代码
async fn handle_offline_operation(
    State(state): State<AppState>,
    Json(op): Json<OfflineOperation>,
) -> Result<Json<OperationResult>, AppError> {
    // 1. 客户端请求去重（idempotency_key）
    let idempotent = sqlx::query!(
        "SELECT result FROM idempotency_keys WHERE key = $1",
        op.idempotency_key
    )
    .fetch_optional(&state.db)
    .await?;

    if let Some(cached) = idempotent {
        return Ok(Json(cached.result));
    }

    // 2. 乐观锁版本检查
    let current = sqlx::query!(
        "SELECT version, status FROM waybills WHERE id = $1",
        op.waybill_id
    )
    .fetch_one(&state.db)
    .await?;

    if op.base_version != current.version {
        // 冲突：返回服务端当前状态，让客户端处理
        return Err(ConflictError {
            waybill_id: op.waybill_id,
            server_state: current,
        });
    }

    // 3. 执行操作
    let new_version = current.version + 1;
    // ... 执行业务逻辑 ...

    // 4. 记录幂等键
    sqlx::query!(
        "INSERT INTO idempotency_keys (key, result, expires_at) VALUES ($1, $2, $3)",
        op.idempotency_key,
        &result,
        now() + INTERVAL '7 days'
    )
    .execute(&state.db)
    .await?;

    Ok(Json(result))
}
```

#### 4. 冲突解决策略

| 冲突场景 | 策略 |
|:--------|:----|
| 司机离线到场，同时调度已取消运单 | **服务端为准**，告知司机运单已取消 |
| 坑口离线叫号，同时系统已重排 | **服务端为准**，通知坑口当前实际队列 |
| 司机离线完单，服务端状态一致 | **自动合并**，正常处理 |
| 双方状态不同但可合并 | **以先到服务端的为准**，后到的做补偿 |

### 数据流示例

```text
司机离线操作 "到场打卡":
  1. Flutter 检查网络 → 离线
  2. 写入本地 SQLite offline_operations (status=pending)
  3. UI 显示 "已提交，待同步"
  4. 司机继续使用（查看缓存的任务和队列）

网络恢复后:
  5. SyncEngine 检测到在线
  6. 按时间顺序提交 offline_operations
  7. 服务端校验版本号，处理冲突
  8. 更新本地数据，UI 切换为 "已同步"
  9. 推送最新状态到司机端
```

### 实施建议

| 阶段 | 内容 | 预估工时 |
|:----|:----|:--------|
| 第一阶段 | Flutter端SQLite集成 + 基础本地缓存 | 3天 |
| 第二阶段 | 离线操作队列 + SyncEngine | 5天 |
| 第三阶段 | 服务端幂等 + 冲突处理 | 3天 |
| 第四阶段 | UI状态提示 + 测试 | 3天 |
| **合计** | | **14天** |

---

## 二、⭐⭐⭐ 车牌识别接入（License Plate Recognition）

### 问题

司机到场时需要核验身份，目前靠手动确认或扫码。车牌识别可以自动完成"车辆到矿即登记"，减少人工干预。

### 方案选型

| 方案 | 成本 | 准确率 | 离线能力 | 推荐 |
|:----|:----|:------|:--------|:---:|
| **海康/大华摄像头SDK** | 中(2000-5000元/路) | 99%+ | 部分支持 | ⭐⭐⭐ |
| **手机摄像头+端侧AI** | 低(软件成本) | 95% | ✅ 完全离线 | ⭐⭐ |
| **第三方API（阿里云等）** | 按调用量计费 | 99%+ | ❌ 需在线 | ⭐⭐ |

**推荐方案：手机端AI识别（免费）+ 可选专业摄像头（付费）**

### 端侧AI识别方案（最低成本）

```
┌─────────────────────────────────────┐
│          Flutter 车牌识别            │
│                                      │
│  1. 司机/门卫打开App摄像头            │
│  2. 屏幕实时显示取景框               │
│  3. 自动检测车牌区域                  │
│  4. Tesseract/MLKit OCR 识别文字     │
│  5. 识别结果自动填入 "到场确认"       │
│  6. 匹配车辆档案中的车牌号            │
└─────────────────────────────────────┘
```

#### Flutter端实现

```dart
// 使用 google_mlkit_text_recognition 进行端侧OCR
import 'package:google_mlkit_text_recognition/binary.dart';

class LicensePlateScanner {
  final TextRecognizer _recognizer = TextRecognizer();

  Future<String?> scanPlate(CameraImage image) async {
    final inputImage = InputImage.fromBytes(...);
    final recognisedText = await _recognizer.processImage(inputImage);

    // 提取车牌号（正则匹配中国车牌）
    for (final block in recognisedText.blocks) {
      for (final line in block.lines) {
        final match = RegExp(r'[京津沪渝冀豫云辽黑湘皖鲁新苏浙赣鄂桂甘晋蒙陕吉闽贵粤川青藏琼][A-Z][A-HJ-NP-Z0-9]{4,5}[A-HJ-NP-Z0-9挂学警港澳]')
            .firstMatch(line.text);
        if (match != null) return match.group(0);
      }
    }
    return null;
  }
}
```

#### 服务端对接

```rust
// 车牌号与运单关联
#[derive(Deserialize)]
struct ArriveWithPlate {
    waybill_id: Uuid,
    plate_number: String,    // 识别的车牌号
    confidence: f32,         // 识别置信度
}

async fn arrive_with_plate(
    State(state): State<AppState>,
    Json(req): Json<ArriveWithPlate>,
) -> Result<Json<WaybillResponse>, AppError> {
    // 1. 校验车牌是否匹配该司机绑定的车辆
    let vehicle = sqlx::query!(
        "SELECT id FROM vehicles WHERE plate_number = $1 AND driver_id = (
            SELECT driver_id FROM waybills WHERE id = $2
        )",
        req.plate_number, req.waybill_id
    )
    .fetch_optional(&state.db)
    .await?;

    if vehicle.is_none() {
        // 车牌不匹配，但记录异常供后续处理
        sqlx::query!(
            "INSERT INTO plate_scan_logs (waybill_id, plate_number, confidence, matched) 
             VALUES ($1, $2, $3, false)",
            req.waybill_id, req.plate_number, req.confidence
        ).execute(&state.db).await?;

        return Err(AppError::PlateNotMatched);
    }

    // 2. 自动完成到场登记
    // ... 原有arrive逻辑...
}
```

### 摄像头硬件方案（可选）

```
拓扑:
  [摄像头] --RTSP--> [边缘盒子(Rust)] --MQTT--> [API服务]
  
  边缘盒子功能:
    - 使用 OpenALPR / HyperLPR 做车牌识别
    - 识别结果通过 MQTT 推送到服务端
    - 缓存最近的100条识别记录
    - 断网时本地存储，恢复后补推
```

### 数据模型

```sql
-- 车牌识别日志
CREATE TABLE plate_scan_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    waybill_id UUID REFERENCES waybills(id),
    plate_number VARCHAR(20) NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.0,
    matched BOOLEAN NOT NULL DEFAULT false,
    scan_source VARCHAR(20) NOT NULL DEFAULT 'manual', -- manual/camera/app
    scanned_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    created_at TIMESTAMPTZ NOT NULL DEFAULT now()
);

-- 车辆档案（扩展）
ALTER TABLE vehicles ADD COLUMN plate_number VARCHAR(20);
CREATE INDEX idx_vehicles_plate ON vehicles(plate_number);
```

### 实施建议

| 阶段 | 内容 | 预估工时 |
|:----|:----|:--------|
| 第一阶段 | 服务端车牌字段+API+日志表 | 1天 |
| 第二阶段 | Flutter端MLKit集成+扫描UI | 3天 |
| 第三阶段 | 可选：边缘盒子+摄像头对接 | 5天 |
| **合计** | | **9天** |

---

## 三、⭐⭐ 地磅自动采集（Auto Scale Collection）

### 问题

目前称重靠地磅操作员手动录入重量，容易出错、造假、效率低。自动采集直接从地磅仪表读取数据。

### 方案一：串口直连（推荐）

```
地磅仪表 --RS232--> 串口服务器 --TCP--> 称重服务(Rust) --API--> 业务系统

流程:
  1. 车辆上磅
  2. 地磅仪表显示重量
  3. 串口服务器读取 RS232 数据
  4. 称重服务解析重量值
  5. 自动关联当前运单
  6. 推送称重结果到操作员确认
```

#### Rust串口读取

```rust
use tokio_serial::SerialPortBuilderExt;
use tokio::io::AsyncReadExt;

struct ScaleReader {
    port: tokio_serial::SerialStream,
}

impl ScaleReader {
    async fn new(path: &str, baud: u32) -> Result<Self> {
        let port = tokio_serial::new(path, baud)
            .open_native_async()?;
        Ok(Self { port })
    }

    // 读取稳定重量（地磅协议通常是连续输出的稳定值）
    async fn read_stable_weight(&mut self) -> Result<f64> {
        let mut buf = [0u8; 1024];
        let mut stable_count = 0;
        let mut last_weight = 0.0;

        for _ in 0..30 {  // 最多等30秒
            let n = self.port.read(&mut buf).await?;
            let data = String::from_utf8_lossy(&buf[..n]);

            // 解析重量（根据地磅协议，常见格式）
            if let Some(weight) = parse_scale_protocol(&data) {
                if (weight - last_weight).abs() < 0.5 {
                    stable_count += 1;
                    if stable_count >= 3 {  // 连续3次稳定
                        return Ok(weight);
                    }
                } else {
                    stable_count = 0;
                }
                last_weight = weight;
            }
            tokio::time::sleep(Duration::from_millis(1000)).await;
        }
        Err(ScaleError::Timeout)
    }
}
```

#### 地磅协议解析

```rust
// 常见地磅协议格式
// 1. 连续稳定格式: ST,GS,+012345kg,ST
// 2. 仪表主动上报: 净重: 12345 kg
// 3. MODBUS RTU: 需要按协议文档解析

fn parse_scale_protocol(data: &str) -> Option<f64> {
    // 尝试匹配常见格式
    let patterns = [
        r"ST,GS,([+-]?\d+)kg,ST",           // 常见地磅协议
        r"净重:\s*([\d.]+)\s*kg",            // 中文格式
        r"NW\s*:\s*([\d.]+)",                // 英文格式
        r"Weight:\s*([\d.]+)",               // 通用格式
    ];

    for pattern in &patterns {
        if let Some(caps) = Regex::new(pattern).ok()?.captures(data) {
            if let Some(weight_str) = caps.get(1) {
                if let Ok(weight) = weight_str.as_str().parse::<f64>() {
                    return Some(weight / 1000.0);  // kg -> t
                }
            }
        }
    }
    None
}
```

### 方案二：平板+蓝牙地磅（最轻量）

```
地磅 --蓝牙--> 坑口平板(Flutter) --网络--> API

Flutter 端:
  - 扫描蓝牙地磅设备
  - 配对连接
  - 读取重量数据
  - 提交到服务端
```

### 防作弊设计

```rust
// 称重异常检测
async fn validate_weighing(
    &self,
    req: &WeighRequest,
    waybill: &Waybill,
) -> Result<(), WeighError> {
    // 1. 皮重历史对比
    if let Some(last_tare) = self.get_last_tare(waybill.vehicle_id).await? {
        let diff = (req.tare_weight - last_tare).abs();
        if diff > 500.0 {  // 皮重偏差超过500kg
            return Err(WeighError::TareDeviation {
                expected: last_tare,
                actual: req.tare_weight,
            });
        }
    }

    // 2. 毛重合理性检查
    let net = req.gross_weight - req.tare_weight;
    let vehicle_capacity = self.get_vehicle_capacity(waybill.vehicle_id).await?;
    if net > vehicle_capacity * 1.1 {  // 超载10%以上
        return Err(WeighError::Overload {
            max: vehicle_capacity,
            actual: net,
        });
    }

    // 3. 称重时长检查（太快可能作弊）
    if req.elapsed_seconds < 10 {
        return Err(WeighError::TooFast);
    }

    Ok(())
}
```

### 数据模型

```sql
-- 称重记录
ALTER TABLE weighings ADD COLUMN source VARCHAR(20) DEFAULT 'manual';
-- source: manual / auto_serial / auto_bluetooth / auto_plate

-- 设备管理
CREATE TABLE scale_devices (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pit_id UUID REFERENCES pits(id),
    device_type VARCHAR(20) NOT NULL,   -- serial/bluetooth/network
    connection_config JSON NOT NULL,     -- {port: "/dev/ttyS0", baud: 9600}
    is_active BOOLEAN DEFAULT true,
    last_heartbeat_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT now()
);

-- 称重原始记录（防篡改）
CREATE TABLE weigh_logs (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    weighing_id UUID REFERENCES weighings(id),
    device_id UUID REFERENCES scale_devices(id),
    raw_data JSON NOT NULL,              -- 地磅原始报文
    weight REAL NOT NULL,
    is_stable BOOLEAN NOT NULL DEFAULT false,
    read_at TIMESTAMPTZ NOT NULL DEFAULT now()
);
```

### 实施建议

| 阶段 | 内容 | 预估工时 |
|:----|:----|:--------|
| 第一阶段 | 蓝牙地磅对接(Flutter) | 3天 |
| 第二阶段 | 串口地磅对接(Rust) | 5天 |
| 第三阶段 | 防作弊+异常检测逻辑 | 3天 |
| **合计** | | **11天** |

---

## 四、⭐⭐ 电子围栏（Geo-fencing）

### 问题

目前"到场"需要司机手动点击"我已到达"，容易忘记或提前点击。电子围栏自动感知车辆进入/离开指定区域。

### 架构

```text
司机App                 服务端
  │                      │
  │──[GPS定位上报]──────→│ 存储位置轨迹
  │                      │──[围栏判断]──→ 触发事件
  │←──[围栏事件推送]────│ 到场/离场通知
```

### Flutter端位置上报

```dart
// 使用 background_locator 实现后台定位
class LocationReporter {
  // 启动后台定位
  Future<void> start() async {
    await BackgroundLocator.registerLocationUpdate(
      _onLocationUpdate,
      initCallback: _onInit,
      disposeCallback: _onDispose,
      iosSettings: IOSSettings(
        accuracy: LocationAccuracy.high,
        distanceFilter: 20.0,  // 20米变化才上报（省电）
      ),
      androidSettings: AndroidSettings(
        interval: 10,           // 秒
        distanceFilter: 20.0,   // 米
        notificationTitle: "运输调度",
        notificationText: "正在获取位置以确认到场",
      ),
    );
  }

  // 上报位置到服务端（带离线缓存）
  Future<void> _onLocationUpdate(Location location) async {
    final payload = {
      'lat': location.latitude,
      'lng': location.longitude,
      'speed': location.speed,
      'accuracy': location.accuracy,
      'timestamp': DateTime.now().toIso8601String(),
    };
    await ApiClient.post('/api/v1/locations/report', payload); // 有离线能力
  }
}
```

### 服务端围栏判定

```rust
use geo::{Point, Polygon, Contains};

#[derive(Debug, sqlx::Type)]
struct GeoFence {
    id: Uuid,
    pit_id: Uuid,
    fence_type: String,           // arrival / departure / restricted
    shape: geo_types::Polygon<f64>,  // 多边形围栏
    radius: Option<f64>,          // 圆形围栏半径(米)
    center_lat: Option<f64>,      // 圆心纬度
    center_lng: Option<f64>,      // 圆心经度
}

async fn check_geo_fence(
    State(state): State<AppState>,
    Json(loc): Json<LocationReport>,
) -> Result<Json<Vec<FenceEvent>>, AppError> {
    let point = Point::new(loc.lng, loc.lat);
    let mut events = vec![];

    // 获取司机相关围栏（所属坑口）
    let fences = sqlx::query_as!(
        GeoFence,
        r#"
        SELECT gf.* FROM geo_fences gf
        JOIN pits p ON gf.pit_id = p.id
        JOIN waybills w ON w.pit_id = p.id
        WHERE w.driver_id = $1 AND w.status IN ('dispatched', 'arrived')
        "#,
        loc.driver_id
    )
    .fetch_all(&state.db)
    .await?;

    for fence in &fences {
        let inside = match &fence.shape {
            Some(polygon) => polygon.contains(&point),
            None => {
                // 圆形围栏：计算距离
                let distance = haversine_distance(
                    fence.center_lat.unwrap(),
                    fence.center_lng.unwrap(),
                    loc.lat, loc.lng
                );
                distance <= fence.radius.unwrap_or(100.0)
            }
        };

        let prev_state = get_driver_fence_state(&state, loc.driver_id, fence.id).await?;

        if inside && !prev_state.inside {
            // 进入围栏 → 自动到场
            if fence.fence_type == "arrival" {
                auto_arrive(&state, loc.driver_id, fence.pit_id).await?;
            }
            events.push(FenceEvent {
                fence_id: fence.id,
                event_type: "enter".into(),
            });
        } else if !inside && prev_state.inside {
            // 离开围栏
            events.push(FenceEvent {
                fence_id: fence.id,
                event_type: "exit".into(),
            });
        }

        // 更新围栏状态
        update_fence_state(&state, loc.driver_id, fence.id, inside).await?;
    }

    Ok(Json(events))
}

// Haversine距离计算
fn haversine_distance(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const R: f64 = 6371000.0; // 地球半径(米)
    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();
    let a = (dlat / 2.0).sin().powi(2)
          + lat1.to_radians().cos()
          * lat2.to_radians().cos()
          * (dlng / 2.0).sin().powi(2);
    R * 2.0 * a.sqrt().asin()
}
```

### 坑口围栏配置界面

```text
调度后台 → 坑口管理 → 设置电子围栏

支持两种模式:
  1. 圆形围栏: 以坑口坐标为中心，设置半径(默认100米)
  2. 多边形围栏: 在地图上画出坑口区域

围栏类型:
  - 到场围栏: 车辆进入自动签到
  - 排队围栏: 车辆进入自动入队
  - 离开围栏: 车辆离开自动标记
```

### 数据模型

```sql
-- 电子围栏
CREATE TABLE geo_fences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    pit_id UUID NOT NULL REFERENCES pits(id),
    name VARCHAR(100) NOT NULL,
    fence_type VARCHAR(20) NOT NULL DEFAULT 'arrival',
    -- arrival: 到场围栏 / departure: 离场 / geofence: 通用
    shape geo_polygon,           -- PostGIS多边形
    center_lat DOUBLE PRECISION, -- 圆形围栏圆心
    center_lng DOUBLE PRECISION,
    radius_meters DOUBLE PRECISION, -- 圆形围栏半径
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ DEFAULT now()
);

-- 司机围栏状态（当前在哪个围栏内）
CREATE TABLE driver_fence_states (
    driver_id UUID NOT NULL REFERENCES drivers(id),
    fence_id UUID NOT NULL REFERENCES geo_fences(id),
    inside BOOLEAN NOT NULL DEFAULT false,
    entered_at TIMESTAMPTZ,
    updated_at TIMESTAMPTZ DEFAULT now(),
    PRIMARY KEY (driver_id, fence_id)
);

-- 位置上报记录（轨迹）
CREATE TABLE location_reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    driver_id UUID NOT NULL REFERENCES drivers(id),
    lat DOUBLE PRECISION NOT NULL,
    lng DOUBLE PRECISION NOT NULL,
    accuracy REAL,
    speed REAL,
    reported_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ DEFAULT now()
);
CREATE INDEX idx_location_reports_driver ON location_reports(driver_id, reported_at);
```

### 实施建议

| 阶段 | 内容 | 预估工时 |
|:----|:----|:--------|
| 第一阶段 | 服务端围栏判断+自动到场逻辑 | 3天 |
| 第二阶段 | Flutter端后台定位+上报 | 3天 |
| 第三阶段 | 后台围栏配置界面 | 2天 |
| **合计** | | **8天** |

---

## 五、⭐ 报表导出（Report Export）

### 问题

财务/运营需要日报、月报、司机结算单、坑口效率报表的对账数据，目前只有实时看板。

### 方案

```text
API                   后台
 │                     │
 │──GET /api/v1/reports/daily────→ 日报预览
 │──GET /api/v1/reports/monthly───→ 月报预览
 │──GET /api/v1/reports/daily?format=xlsx → 导出Excel
 │──GET /api/v1/reports/driver/:id → 司机结算单
```

### Rust导出Excel

```rust
use rust_xlsxwriter::*;

async fn export_daily_report(
    State(state): State<AppState>,
    Query(params): Query<ReportParams>,
) -> Result<Vec<u8>, AppError> {
    let data = sqlx::query_as!(
        DailyRow,
        r#"
        SELECT 
            w.waybill_no,
            d.name as driver_name,
            v.plate_number,
            p.name as pit_name,
            w.status,
            wg.gross_weight,
            wg.tare_weight,
            wg.net_weight,
            w.dispatched_at,
            w.completed_at
        FROM waybills w
        JOIN drivers d ON w.driver_id = d.id
        JOIN vehicles v ON w.vehicle_id = v.id
        JOIN pits p ON w.pit_id = p.id
        LEFT JOIN weighings wg ON w.id = wg.waybill_id
        WHERE w.created_at >= $1 AND w.created_at < $2
        ORDER BY w.created_at
        "#,
        params.date_start, params.date_end + Duration::days(1)
    )
    .fetch_all(&state.db)
    .await?;

    // 生成Excel
    let mut workbook = Workbook::new();
    let sheet = workbook.add_worksheet();

    // 标题行
    let header_format = Format::new().set_bold();
    let headers = ["运单号", "司机", "车牌", "坑口", "状态", 
                   "毛重(t)", "皮重(t)", "净重(t)", "派单时间", "完成时间"];
    for (col, header) in headers.iter().enumerate() {
        sheet.write_string(0, col as u16, header, &header_format)?;
    }

    // 数据行
    for (row, record) in data.iter().enumerate() {
        sheet.write_string((row + 1) as u32, 0, &record.waybill_no)?;
        sheet.write_string((row + 1) as u32, 1, &record.driver_name)?;
        // ...
    }

    // 汇总行
    let total_row = data.len() + 2;
    sheet.write_string(total_row as u32, 0, "合计", &header_format)?;
    let total_net: f64 = data.iter().map(|r| r.net_weight.unwrap_or(0.0)).sum();
    sheet.write_number(total_row as u32, 7, total_net)?;

    workbook.save_to_buffer().map_err(|e| AppError::ExportError(e.to_string()))
}
```

### 报表类型

```rust
#[derive(Deserialize)]
enum ReportType {
    #[serde(rename = "daily")]
    Daily,           // 日报：当天运单明细
    #[serde(rename = "monthly")]
    Monthly,         // 月报：按月汇总
    #[serde(rename = "driver_settlement")]
    DriverSettlement, // 司机结算单：按司机+时间段
    #[serde(rename = "pit_efficiency")]
    PitEfficiency,    // 坑口效率：吞吐量+等待时间
}

#[derive(Deserialize)]
struct ReportParams {
    r#type: ReportType,
    date_start: NaiveDate,
    date_end: Option<NaiveDate>,
    driver_id: Option<Uuid>,
    pit_id: Option<Uuid>,
    format: Option<String>,  // csv / xlsx (默认csv)
}
```

### 数据模型（预聚合）

```sql
-- 日报预聚合（每天凌晨计算）
CREATE TABLE daily_summary (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_date DATE NOT NULL,
    total_waybills INTEGER NOT NULL DEFAULT 0,
    completed_waybills INTEGER NOT NULL DEFAULT 0,
    cancelled_waybills INTEGER NOT NULL DEFAULT 0,
    total_net_weight REAL NOT NULL DEFAULT 0.0,
    avg_wait_minutes REAL,
    avg_loading_minutes REAL,
    pit_breakdown JSON,    -- 各坑口数据
    driver_breakdown JSON, -- 各司机数据
    created_at TIMESTAMPTZ DEFAULT now(),
    UNIQUE(report_date)
);
```

### 实施建议

| 阶段 | 内容 | 预估工时 |
|:----|:----|:--------|
| 第一阶段 | Service层查询+CSV导出 | 1天 |
| 第二阶段 | Excel导出（rust_xlsxwriter） | 1天 |
| 第三阶段 | 后台报表预览+下载UI | 1天 |
| 第四阶段 | 预聚合定时任务+月报 | 2天 |
| **合计** | | **5天** |

---

## 六、无人矿卡趋势应对策略

### 趋势判断

无人驾驶矿卡是**终局**，但不是**现在**。

```
现在 (-2026):   有人驾驶 + 数字化调度   ← 你们在这里
过渡期 (2026-2028): 有人/无人混行调度  ← 需要准备
终局 (2028+):    全无人驾驶 + 自动调度  ← 远期方向
```

### 你们现在应该做的事

#### 1. 兼容有人/无人混行（1年内）

无人矿卡和有人车辆在同一矿区运行将是过渡期常态。你们的系统需要：

```text
调度系统面向:
  - 有人车: 派单→到场→排队→装车→称重→完单（现有流程）
  - 无人车: 系统自动派单→车辆自动到位→自动装载→自动称重→自动完单

差异仅在执行层，调度逻辑不变:
  有人车: 通知 → 司机操作
  无人车: 通知 → 中控平台执行
```

#### 2. 预留API对接（现在）

```rust
// 无人驾驶中控系统对接接口
#[derive(Deserialize)]
struct AutonomousDispatch {
    vehicle_id: String,
    mission_type: String,  // loading / hauling / dumping
    source_pit: Uuid,
    destination: String,
    priority: i32,
}

// 无人车状态上报
#[derive(Deserialize)]
struct AutonomousStatus {
    vehicle_id: String,
    status: String,           // en_route / loading / hauling / dumping / idle
    position: Position,
    battery_level: f32,
    payload_weight: f64,
    estimated_arrival: Option<NaiveDateTime>,
}
```

#### 3. 短期不做，但需要知道的技术

| 技术 | 当前状态 | 建议介入时间 |
|:----|:--------|:-----------|
| V2X车路通信 | 5G正在普及 | 2026-2027 |
| 数字孪生 | 大厂在做 | 2027+ |
| AI动态配矿 | 迪迈已有 | 2026下半年 |
| 自动驾驶矿卡 | 试点阶段 | 2028+ |

### 你们的定位

```
不要试图做无人驾驶，那不是你们的战场。

你们的战场是:
  ✅ 有人驾驶矿山的运输调度数字化（现在）
  ✅ 有人/无人混行的调度协调层（过渡期）
  ❌ 无人驾驶执行层（留给希迪智驾/路凯智行）

在这个定位下，无人驾驶是你们的数据消费者:
  无人驾驶中控 ← 获取调度指令 ← 你们的系统
  无人驾驶中控 → 上报状态 → 你们的系统
```

---

## 七、实施路线图

```
优先级   功能        工时    开始时间
──────────────────────────────────────
⭐⭐⭐    离线调度     14天   第1周  ← 最紧急，没这个不能用
⭐⭐⭐    车牌识别      9天   第3周
⭐⭐     地磅采集     11天   第5周
⭐⭐     电子围栏      8天   第7周
⭐      报表导出      5天   第9周
──────────────────────────────────────
        总计         47天   ~10周
无人驾驶API预留      穿插进行
```
