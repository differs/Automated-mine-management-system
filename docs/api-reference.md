# API 参考文档

## 概述

本文档描述了 Automated Mine Management System 的所有 API 接口。所有 API 均基于 RESTful 设计，使用 JSON 作为请求和响应格式。

## 基础信息

- **Base URL**: `http://<host>:3000/api/v1`
- **Content-Type**: `application/json`
- **认证方式**: Bearer Token (JWT)

## 通用规范

### 错误响应格式

```json
{
  "code": "error_code",
  "message": "错误描述"
}
```

### HTTP 状态码说明

| 状态码 | 含义 |
|--------|------|
| 200 | 请求成功 |
| 201 | 创建成功 |
| 400 | 请求参数错误 |
| 401 | 未认证或认证过期 |
| 403 | 无权限 |
| 404 | 资源不存在 |
| 409 | 资源冲突（如重复创建、状态冲突） |
| 500 | 服务器内部错误 |

---

## 1. 认证 (Auth)

### 1.1 登录

```
POST /auth/login
```

**Request Body:**

```json
{
  "username": "admin",
  "password": "password123"
}
```

**Response (200):**

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs...",
  "refresh_token": "eyJhbGciOiJIUzI1NiIs...",
  "role": "super_admin",
  "display_name": "管理员"
}
```

**错误码：**
- `400` - 用户名或密码为空
- `401` - 用户名或密码错误

### 1.2 刷新令牌

```
POST /auth/refresh
```

**Request Body:**

```json
{
  "refresh_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

**Response (200):**

```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIs..."
}
```

---

## 2. 司机管理 (Driver)

### 2.1 创建司机

```
POST /drivers
```

**Request Body:**

```json
{
  "name": "张三",
  "phone": "13800138000",
  "license_plate": "贵A12345",
  "vehicle_type": "dump_truck",
  "capacity_ton": 30.0
}
```

**vehicle_type 枚举：** `dump_truck` (自卸车), `trailer` (挂车), `other` (其他)

**Response (200):**

```json
{
  "id": "uuid",
  "name": "张三",
  "phone": "13800138000",
  "license_plate": "贵A12345",
  "vehicle_type": "dump_truck",
  "capacity_ton": 30.0,
  "status": "idle",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

### 2.2 获取司机列表

```
GET /drivers?keyword=张三&status=idle
```

**Query Parameters:**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| keyword | string | 否 | 搜索关键词（匹配姓名、电话、车牌） |
| status | string | 否 | 过滤状态（idle/working/offline） |

**Response (200):**

```json
[
  {
    "id": "uuid",
    "name": "张三",
    "phone": "13800138000",
    "license_plate": "贵A12345",
    "vehicle_type": "dump_truck",
    "status": "idle"
  }
]
```

### 2.3 获取司机详情

```
GET /drivers/{driver_id}
```

**Response (200):**

```json
{
  "id": "uuid",
  "name": "张三",
  "phone": "13800138000",
  "license_plate": "贵A12345",
  "vehicle_type": "dump_truck",
  "capacity_ton": 30.0,
  "status": "idle",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

### 2.4 导入司机

```
POST /drivers/import
```

**Request Body:**

```json
{
  "source": "batch_20260101.csv",
  "total_rows": 100
}
```

**Response (200):**

```json
{
  "accepted": true,
  "source": "batch_20260101.csv",
  "total_rows": 100
}
```

---

## 3. 坑口管理 (Pit)

### 3.1 创建坑口

```
POST /pits
```

**Request Body:**

```json
{
  "name": "1号采区",
  "code": "PIT-001",
  "location_text": "矿区东侧",
  "queue_capacity": 15
}
```

**Response (200):**

```json
{
  "id": "uuid",
  "name": "1号采区",
  "code": "PIT-001",
  "location_text": "矿区东侧",
  "queue_capacity": 15,
  "current_queue_count": 0,
  "avg_wait_minutes": 0,
  "is_active": true
}
```

### 3.2 获取坑口列表

```
GET /pits
```

**Response (200):**

```json
[
  {
    "id": "uuid",
    "name": "1号采区",
    "code": "PIT-001",
    "current_queue_count": 5,
    "avg_wait_minutes": 12,
    "is_active": true
  }
]
```

### 3.3 获取坑口详情

```
GET /pits/{pit_id}
```

**Response (200):** 同创建响应。

---

## 4. 运单管理 (Waybill)

### 4.1 创建运单

```
POST /waybills
```

**Request Body:**

```json
{
  "driver_id": "uuid",
  "pit_id": "uuid",
  "estimated_weight_ton": 35.0
}
```

**Response (200):**

```json
{
  "id": "uuid",
  "serial_no": "WB-20260101-000001",
  "driver_id": "uuid",
  "pit_id": "uuid",
  "status": "pending_dispatch",
  "queue_number": null,
  "estimated_weight_ton": 35.0,
  "actual_weight_ton": null,
  "dispatch_time": null,
  "arrive_time": null
}
```

### 4.2 获取运单列表

```
GET /waybills?status=dispatched&pit_id=uuid
```

**Query Parameters:**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| status | string | 否 | 状态过滤 |
| pit_id | uuid | 否 | 坑口过滤 |

**Response (200):**

```json
[
  {
    "id": "uuid",
    "serial_no": "WB-20260101-000001",
    "driver_id": "uuid",
    "pit_id": "uuid",
    "status": "dispatched",
    "dispatch_time": "2026-01-01T08:00:00Z"
  }
]
```

### 4.3 获取运单详情

```
GET /waybills/{waybill_id}
```

**Response (200):**

```json
{
  "id": "uuid",
  "serial_no": "WB-20260101-000001",
  "driver_id": "uuid",
  "pit_id": "uuid",
  "status": "completed",
  "queue_number": 3,
  "estimated_weight_ton": 35.0,
  "actual_weight_ton": 32.5,
  "dispatch_time": "2026-01-01T08:00:00Z",
  "arrive_time": "2026-01-01T08:15:00Z"
}
```

### 4.4 派单

```
POST /waybills/{waybill_id}/dispatch
```

**Request Body:**

```json
{
  "dispatcher_id": "uuid"
}
```

**Response (200):**

```json
{
  "id": "uuid",
  "status": "dispatched",
  "at": "2026-01-01T08:00:00Z"
}
```

### 4.5 司机到场

```
POST /waybills/{waybill_id}/arrive
```

**Request Body:**

```json
{
  "arrival_source": "driver_app"
}
```

**Response (200):**

```json
{
  "id": "uuid",
  "status": "arrived",
  "at": "2026-01-01T08:15:00Z"
}
```

### 4.6 取消运单

```
POST /waybills/{waybill_id}/cancel
```

**Request Body:**

```json
{
  "cancelled_by": "uuid",
  "reason": "司机临时有事"
}
```

**Response (200):**

```json
{
  "id": "uuid",
  "status": "cancelled",
  "at": "2026-01-01T09:00:00Z"
}
```

### 运单状态机

```
待派车 (pending_dispatch)
  ├── 派单 → 已派车 (dispatched)
  └── 取消 → 已取消 (cancelled)

已派车 (dispatched)
  ├── 到场 → 已到场 (arrived)
  └── 取消 → 已取消 (cancelled)

已到场 (arrived)
  ├── 入队 → 排队中 (queueing)
  └── 取消 → 已取消 (cancelled)

排队中 (queueing)
  ├── 叫号 → 装载中 (loading)
  └── 取消 → 已取消 (cancelled)

装载中 (loading)
  ├── 完成装载 → 已装载 (loaded)
  └── 取消 → 已取消 (cancelled)

已装载 (loaded)
  ├── 称重 → 已完成 (completed)
  └── 取消 → 已取消 (cancelled)
```

---

## 5. 排队管理 (Queue)

### 5.1 查看坑口队列

```
GET /queue/pits/{pit_id}
```

**Response (200):**

```json
[
  {
    "waybill_id": "uuid",
    "driver_id": "uuid",
    "queue_position": 1,
    "entered_at": "2026-01-01T08:20:00Z"
  }
]
```

### 5.2 入队

```
POST /queue/waybills/{waybill_id}/join
```

**Request Body:**

```json
{
  "driver_id": "uuid",
  "pit_id": "uuid",
  "arrival_method": "driver_app_self_checkin"
}
```

**Response (200):**

```json
{
  "waybill_id": "uuid",
  "status": "queueing",
  "queue_position": 3,
  "at": "2026-01-01T08:20:00Z"
}
```

### 5.3 叫号

```
POST /queue/waybills/{waybill_id}/call-next
```

**Request Body:**

```json
{
  "operator_id": "uuid",
  "reason": null
}
```

**Response (200):**

```json
{
  "waybill_id": "uuid",
  "status": "called",
  "queue_position": 1,
  "at": "2026-01-01T08:30:00Z"
}
```

### 5.4 离队

```
POST /queue/waybills/{waybill_id}/leave
```

**Request Body:**

```json
{
  "operator_id": "uuid",
  "reason": "司机弃单"
}
```

**Response (200):**

```json
{
  "waybill_id": "uuid",
  "status": "left_queue",
  "queue_position": 2,
  "at": "2026-01-01T09:00:00Z"
}
```

---

## 6. 装车管理 (Loading)

### 6.1 开始装车

```
POST /loading/waybills/{waybill_id}/start
```

**Request Body:**

```json
{
  "operator_id": "uuid",
  "loader_name": "装载机1号",
  "notes": null
}
```

**Response (200):**

```json
{
  "waybill_id": "uuid",
  "status": "loading",
  "at": "2026-01-01T08:30:00Z"
}
```

### 6.2 结束装车

```
POST /loading/waybills/{waybill_id}/finish
```

**Request Body:**

```json
{
  "operator_id": "uuid",
  "notes": "装车完毕"
}
```

**Response (200):**

```json
{
  "waybill_id": "uuid",
  "status": "loaded",
  "at": "2026-01-01T08:45:00Z"
}
```

---

## 7. 称重管理 (Weighing)

### 7.1 称重完成

```
POST /weighing/waybills/{waybill_id}
```

**Request Body:**

```json
{
  "operator_id": "uuid",
  "gross_weight_ton": 50.0,
  "tare_weight_ton": 17.5,
  "net_weight_ton": 32.5,
  "source": "manual",
  "note": null
}
```

**source 可选值：** `manual` (手工录入), `auto` (地磅自动), `corrected` (更正)

**Response (200):**

```json
{
  "waybill_id": "uuid",
  "status": "completed",
  "net_weight_ton": 32.5,
  "completed_at": "2026-01-01T09:00:00Z"
}
```

---

## 8. 看板统计 (Dashboard)

### 8.1 获取运营概览

```
GET /dashboard/overview
```

**Response (200):**

```json
{
  "today_total_waybills": 45,
  "today_completed": 38,
  "today_cancelled": 2,
  "in_progress": 5,
  "today_total_tonnage": 1234.5,
  "pit_summaries": [
    {
      "pit_id": "uuid",
      "pit_name": "1号采区",
      "current_queue": 5,
      "avg_wait_minutes": 12,
      "today_trips": 15,
      "today_tonnage": 487.5
    }
  ],
  "alerts": [
    {
      "type": "late_arrival",
      "waybill_id": "uuid",
      "description": "运单 WB-20260101-000015 司机超时未到场",
      "severity": 1
    }
  ],
  "date": "2026-01-01"
}
```

### 8.2 获取坑口效率排名

```
GET /dashboard/pit-efficiency
```

**Response (200):**

```json
[
  {
    "pit_id": "uuid",
    "pit_name": "1号采区",
    "today_trips": 15,
    "today_tonnage": 487.5,
    "avg_wait_minutes": 12,
    "avg_loading_minutes": 8
  }
]
```

### 8.3 获取司机排名

```
GET /dashboard/driver-ranking?sort_by=trips&limit=10
```

**Response (200):**

```json
[
  {
    "driver_id": "uuid",
    "driver_name": "张三",
    "license_plate": "贵A12345",
    "today_trips": 8,
    "today_tonnage": 260.0
  }
]
```

---

## 9. 告警管理 (Alert)

### 9.1 获取告警列表

```
GET /alerts?status=open&type=late_arrival
```

**Query Parameters:**
| 参数 | 类型 | 必填 | 说明 |
|------|------|------|------|
| status | string | 否 | 状态过滤 (open/resolved) |
| type | string | 否 | 类型过滤 |

**Response (200):**

```json
[
  {
    "id": "uuid",
    "waybill_id": "uuid",
    "type": "late_arrival",
    "severity": 1,
    "description": "运单 WB-20260101-000015 司机超时未到场",
    "status": "open",
    "reported_at": "2026-01-01T09:00:00Z",
    "resolved_at": null
  }
]
```

### 9.2 解决告警

```
POST /alerts/{alert_id}/resolve
```

**Request Body:**

```json
{
  "resolved_by": "uuid"
}
```

**Response (200):**

```json
{
  "id": "uuid",
  "status": "resolved",
  "resolved_at": "2026-01-01T10:00:00Z"
}
```

---

## 10. WebSocket 实时推送

### 连接地址

```
ws://<host>:3000/ws?token=<jwt_token>
```

### 推送事件类型

| 事件 | 说明 | 推送对象 |
|------|------|---------|
| `waybill.dispatched` | 新派单 | 对应司机 |
| `queue.updated` | 队列变化 | 坑口管理员 / 调度看板 |
| `queue.called` | 叫号通知 | 对应司机 |
| `loading.started` | 开始装车 | 调度看板 |
| `loading.finished` | 装车完成 | 调度看板 |
| `weighing.completed` | 称重完成 | 调度看板 / 财务 |
| `alert.created` | 新告警 | 调度员 |

### 消息格式

```json
{
  "event": "queue.called",
  "data": {
    "waybill_id": "uuid",
    "driver_id": "uuid",
    "pit_id": "uuid",
    "pit_name": "1号采区",
    "queue_position": 1,
    "called_at": "2026-01-01T08:30:00Z"
  }
}
```
