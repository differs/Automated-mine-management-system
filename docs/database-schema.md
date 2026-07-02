# 数据库 Schema 文档

## 概述

系统使用 PostgreSQL 16+ 作为主数据库，所有业务数据持久化存储。数据库设计遵循以下原则：

- 使用 UUID 作为主键，避免自增 ID 冲突
- 所有表包含 `created_at` 和 `updated_at` 审计字段
- 状态使用 ENUM 类型保证数据完整性
- 关键业务操作记录到 `operation_logs` 表
- 运单使用乐观锁（`version` 字段）控制并发

## ER 图（文字描述）

```
users ──────┬── creates ──→ waybills
            ├── operates ─→ loading_records
            ├── operates ─→ weigh_records
            └── reports ──→ exception_records

drivers ────┬── has ──────→ waybills
            └── belongs ──→ fleets (可选)

pits ───────┬── has ──────→ waybills
            └── has ──────→ queue_logs

waybills ───┬── has ──────→ queue_logs
            ├── has ──────→ loading_records
            ├── has ──────→ weigh_records
            ├── has ──────→ exception_records
            └── references → shifts / haul_routes (可选)

fleets ─────── has ──────→ drivers (可选)

shifts ─────── references → waybills (可选)

haul_routes ── references → waybills (可选)
```

## 表结构

### 1. users（用户）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK, DEFAULT gen_random_uuid() | 用户 ID |
| username | VARCHAR(64) | NOT NULL, UNIQUE | 用户名 |
| password_hash | TEXT | NOT NULL | 密码哈希 (bcrypt) |
| display_name | VARCHAR(64) | NOT NULL | 显示名称 |
| role | user_role | NOT NULL | 角色枚举 |
| is_active | BOOLEAN | NOT NULL, DEFAULT TRUE | 是否启用 |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | 更新时间 |

**user_role 枚举：** `super_admin`, `dispatcher`, `pit_operator`, `weigh_operator`, `finance`, `ops_analyst`

### 2. drivers（司机）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 司机 ID |
| name | VARCHAR(64) | NOT NULL | 姓名 |
| phone | VARCHAR(32) | NOT NULL, UNIQUE | 手机号 |
| license_plate | VARCHAR(32) | NOT NULL, UNIQUE | 车牌号 |
| vehicle_type | vehicle_type | NOT NULL | 车型 |
| capacity_ton | NUMERIC(10,2) | NOT NULL, DEFAULT 0 | 额定载重(吨) |
| tare_weight_ton | NUMERIC(10,2) | 可空 | 皮重(吨) |
| status | driver_status | NOT NULL, DEFAULT 'idle' | 状态 |
| fleet_id | UUID | FK → fleets(id), 可空 | 所属车队 |
| identity_no | VARCHAR(64) | 可空 | 身份证号 |
| safety_acknowledged_at | TIMESTAMPTZ | 可空 | 最近安全确认时间 |
| wechat_openid | VARCHAR(128) | 可空 | 微信 OpenID |
| is_active | BOOLEAN | NOT NULL, DEFAULT TRUE | 是否启用 |
| last_active_at | TIMESTAMPTZ | 可空 | 最后活跃时间 |
| created_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL, DEFAULT NOW() | 更新时间 |

**driver_status 枚举：** `idle` (空闲), `working` (工作中), `offline` (离线)
**vehicle_type 枚举：** `dump_truck` (自卸车), `trailer` (挂车), `other` (其他)

### 3. pits（坑口）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 坑口 ID |
| name | VARCHAR(64) | NOT NULL, UNIQUE | 坑口名称 |
| code | VARCHAR(32) | UNIQUE | 坑口编号 |
| location_text | VARCHAR(255) | 可空 | 位置描述 |
| queue_capacity | INTEGER | 可空 | 排队容量上限 |
| current_queue_count | INTEGER | NOT NULL, DEFAULT 0 | 当前排队数 |
| avg_wait_minutes | INTEGER | NOT NULL, DEFAULT 0 | 平均等待时间(分钟) |
| is_active | BOOLEAN | NOT NULL, DEFAULT TRUE | 是否启用 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

### 4. waybills（运单 - 核心表）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 运单 ID |
| serial_no | VARCHAR(40) | NOT NULL, UNIQUE | 运单编号 |
| driver_id | UUID | FK → drivers(id) | 司机 |
| pit_id | UUID | FK → pits(id) | 坑口 |
| status | waybill_status | NOT NULL | 运单状态 |
| queue_number | INTEGER | 可空 | 排队序号 |
| estimated_weight_ton | NUMERIC(10,2) | 可空 | 预估重量(吨) |
| actual_weight_ton | NUMERIC(10,2) | 可空 | 实际重量(吨) |
| dispatch_time | TIMESTAMPTZ | 可空 | 派单时间 |
| arrive_time | TIMESTAMPTZ | 可空 | 到场时间 |
| queue_enter_time | TIMESTAMPTZ | 可空 | 入队时间 |
| queue_exit_time | TIMESTAMPTZ | 可空 | 离队时间 |
| load_start_time | TIMESTAMPTZ | 可空 | 开始装车时间 |
| load_end_time | TIMESTAMPTZ | 可空 | 结束装车时间 |
| weigh_start_time | TIMESTAMPTZ | 可空 | 称重时间 |
| completed_time | TIMESTAMPTZ | 可空 | 完成时间 |
| cancelled_time | TIMESTAMPTZ | 可空 | 取消时间 |
| cancelled_reason | TEXT | 可空 | 取消原因 |
| created_by | UUID | FK → users(id) | 创建人 |
| cancelled_by | UUID | FK → users(id) | 取消人 |
| shift_id | UUID | FK → shifts(id), 可空 | 班次 |
| route_id | UUID | FK → haul_routes(id), 可空 | 运输路线 |
| priority | waybill_priority | NOT NULL, DEFAULT 'normal' | 优先级 |
| manual_override_reason | TEXT | 可空 | 手工干预原因 |
| unload_site_name | VARCHAR(128) | 可空 | 卸货点名称 |
| version | INTEGER | NOT NULL, DEFAULT 1 | 乐观锁版本号 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

**waybill_status 枚举（9 种状态）：**

```
pending_dispatch → dispatched → arrived → queueing → loading → loaded → weighing → completed
                                                                                       ↓
                                                                                  cancelled
```

**waybill_priority 枚举：** `normal`, `urgent`, `vip_override`

**唯一约束：** 同一司机同一时间只能有一个活动运单（部分状态上创建唯一索引）

### 5. queue_logs（排队日志）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 日志 ID |
| pit_id | UUID | FK → pits(id) | 坑口 ID |
| driver_id | UUID | FK → drivers(id) | 司机 ID |
| waybill_id | UUID | FK → waybills(id) | 运单 ID |
| enter_queue_time | TIMESTAMPTZ | NOT NULL | 入队时间 |
| exit_queue_time | TIMESTAMPTZ | 可空 | 离队时间（NULL 表示仍在队列） |
| queue_position | INTEGER | NOT NULL, CHECK > 0 | 排队序号 |
| is_manual_adjustment | BOOLEAN | NOT NULL, DEFAULT FALSE | 是否手工调整 |
| adjustment_reason | TEXT | 可空 | 调整原因 |
| created_by | UUID | FK → users(id) | 操作人 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |

### 6. loading_records（装车记录）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 记录 ID |
| waybill_id | UUID | FK, UNIQUE | 运单（一对一） |
| pit_id | UUID | FK → pits(id) | 坑口 ID |
| operator_id | UUID | FK → users(id) | 操作人 |
| start_time | TIMESTAMPTZ | NOT NULL | 开始时间 |
| end_time | TIMESTAMPTZ | 可空 | 结束时间 |
| loader_name | VARCHAR(64) | 可空 | 装载机编号 |
| notes | TEXT | 可空 | 备注 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

### 7. weigh_records（称重记录）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 记录 ID |
| waybill_id | UUID | FK, UNIQUE | 运单（一对一） |
| gross_weight_ton | NUMERIC(10,2) | NOT NULL, CHECK >= 0 | 毛重(吨) |
| tare_weight_ton | NUMERIC(10,2) | 可空, CHECK >= 0 | 皮重(吨) |
| net_weight_ton | NUMERIC(10,2) | NOT NULL, CHECK >= 0 | 净重(吨) |
| weigh_time | TIMESTAMPTZ | NOT NULL | 称重时间 |
| operator_id | UUID | FK → users(id) | 操作人 |
| source | VARCHAR(32) | NOT NULL, DEFAULT 'manual' | 来源（manual/auto/corrected） |
| note | TEXT | 可空 | 备注 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

### 8. exception_records（异常记录）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 记录 ID |
| waybill_id | UUID | FK → waybills(id) | 运单 ID |
| type | exception_type | NOT NULL | 异常类型 |
| severity | SMALLINT | NOT NULL, DEFAULT 1 | 严重级别(1-5) |
| description | TEXT | NOT NULL | 异常描述 |
| status | VARCHAR(32) | NOT NULL, DEFAULT 'open' | 状态(open/resolved) |
| reported_by | UUID | FK → users(id) | 报告人 |
| resolved_by | UUID | FK → users(id) | 解决人 |
| resolved_at | TIMESTAMPTZ | 可空 | 解决时间 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

**exception_type 枚举：** `late_arrival`, `queue_jump`, `loading_timeout`, `weight_deviation`, `left_without_weighing`, `manual_override`, `other`

### 9. operation_logs（操作审计日志）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 日志 ID |
| entity_type | VARCHAR(32) | NOT NULL | 实体类型（waybill/driver/pit） |
| entity_id | UUID | NOT NULL | 实体 ID |
| action | VARCHAR(64) | NOT NULL | 操作（created/dispatched/cancelled） |
| before_data | JSONB | 可空 | 操作前数据快照 |
| after_data | JSONB | 可空 | 操作后数据快照 |
| operator_id | UUID | FK → users(id) | 操作人 |
| operator_name | VARCHAR(64) | 可空 | 操作人姓名 |
| reason | TEXT | 可空 | 操作原因 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |

### 10. fleets（车队）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 车队 ID |
| name | VARCHAR(64) | NOT NULL, UNIQUE | 车队名称 |
| company_name | VARCHAR(128) | 可空 | 所属公司 |
| contact_name | VARCHAR(64) | 可空 | 联系人 |
| contact_phone | VARCHAR(32) | 可空 | 联系电话 |
| is_active | BOOLEAN | NOT NULL, DEFAULT TRUE | 是否启用 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

### 11. shifts（班次）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 班次 ID |
| name | VARCHAR(64) | NOT NULL | 班次名称 |
| code | VARCHAR(32) | NOT NULL, UNIQUE | 班次编号 |
| starts_at | TIME | NOT NULL | 开始时间 |
| ends_at | TIME | NOT NULL | 结束时间 |
| crosses_day | BOOLEAN | NOT NULL, DEFAULT FALSE | 是否跨天 |
| is_active | BOOLEAN | NOT NULL, DEFAULT TRUE | 是否启用 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

### 12. haul_routes（运输路线）

| 字段 | 类型 | 约束 | 说明 |
|------|------|------|------|
| id | UUID | PK | 路线 ID |
| code | VARCHAR(32) | NOT NULL, UNIQUE | 路线编号 |
| name | VARCHAR(128) | NOT NULL | 路线名称 |
| pit_id | UUID | FK → pits(id) | 起点坑口 |
| unload_site_name | VARCHAR(128) | NOT NULL | 卸货点名称 |
| distance_km | NUMERIC(10,2) | 可空 | 距离(公里) |
| unit_price | NUMERIC(10,2) | 可空 | 单价(元/吨) |
| is_active | BOOLEAN | NOT NULL, DEFAULT TRUE | 是否启用 |
| created_at | TIMESTAMPTZ | NOT NULL | 创建时间 |
| updated_at | TIMESTAMPTZ | NOT NULL | 更新时间 |

## 索引策略

| 表 | 索引 | 类型 | 说明 |
|----|------|------|------|
| waybills | ux_waybills_driver_active | UNIQUE, 部分索引 | 同一司机不可有多个活动运单 |
| waybills | idx_waybills_pit_status | BTREE | 按坑口和状态查询 |
| waybills | idx_waybills_driver_status | BTREE | 按司机和状态查询 |
| waybills | idx_waybills_dispatch_time | BTREE | 按派单时间排序 |
| queue_logs | idx_queue_logs_pit_enter_time | BTREE | 队列按时间排序 |
| exception_records | idx_exception_records_waybill | BTREE | 按运单查询异常 |

## 迁移管理

迁移文件位于 `db/migrations/` 目录，按序号命名：

```
0001_mvp_init.sql    -- 一期核心表（users, drivers, pits, waybills 等）
0002_real_scene_extensions.sql  -- 真实场景扩展（fleets, shifts, haul_routes）
```

### 编写迁移规范

```sql
-- 0003_feature_name.sql
-- Description: 添加功能描述

DO $$
BEGIN
  -- 使用 IF NOT EXISTS 保证幂等
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'new_enum') THEN
    CREATE TYPE new_enum AS ENUM ('value1', 'value2');
  END IF;
END
$$;

CREATE TABLE IF NOT EXISTS new_table (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  -- ...
);

ALTER TABLE existing_table
  ADD COLUMN IF NOT EXISTS new_column VARCHAR(64);
```

## 数据保留策略

| 表 | 保留期限 | 说明 |
|----|---------|------|
| waybills | 永久 | 业务核心数据 |
| queue_logs | 永久 | 审计需要 |
| loading_records | 永久 | 审计需要 |
| weigh_records | 永久 | 结算依据 |
| exception_records | 3 年 | 异常追溯 |
| operation_logs | 3 年 | 操作审计 |
