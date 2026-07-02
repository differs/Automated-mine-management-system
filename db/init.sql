CREATE EXTENSION IF NOT EXISTS "pgcrypto";

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'driver_status') THEN
    CREATE TYPE driver_status AS ENUM ('idle', 'working', 'offline');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'vehicle_type') THEN
    CREATE TYPE vehicle_type AS ENUM ('dump_truck', 'trailer', 'other');
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'waybill_status') THEN
    CREATE TYPE waybill_status AS ENUM (
      'pending_dispatch',
      'dispatched',
      'arrived',
      'queueing',
      'loading',
      'loaded',
      'weighing',
      'completed',
      'cancelled'
    );
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'user_role') THEN
    CREATE TYPE user_role AS ENUM (
      'super_admin',
      'dispatcher',
      'pit_operator',
      'weigh_operator',
      'finance',
      'ops_analyst'
    );
  END IF;

  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'exception_type') THEN
    CREATE TYPE exception_type AS ENUM (
      'late_arrival',
      'queue_jump',
      'loading_timeout',
      'weight_deviation',
      'left_without_weighing',
      'manual_override',
      'other'
    );
  END IF;
END
$$;

CREATE TABLE IF NOT EXISTS users (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  username VARCHAR(64) NOT NULL UNIQUE,
  password_hash TEXT NOT NULL,
  display_name VARCHAR(64) NOT NULL,
  role user_role NOT NULL,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS drivers (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL,
  phone VARCHAR(32) NOT NULL UNIQUE,
  license_plate VARCHAR(32) NOT NULL UNIQUE,
  vehicle_type vehicle_type NOT NULL DEFAULT 'dump_truck',
  capacity_ton NUMERIC(10,2) NOT NULL DEFAULT 0,
  tare_weight_ton NUMERIC(10,2),
  status driver_status NOT NULL DEFAULT 'idle',
  wechat_openid VARCHAR(128),
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  last_active_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS pits (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  code VARCHAR(32) UNIQUE,
  location_text VARCHAR(255),
  queue_capacity INTEGER,
  current_queue_count INTEGER NOT NULL DEFAULT 0,
  avg_wait_minutes INTEGER NOT NULL DEFAULT 0,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS waybills (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  serial_no VARCHAR(40) NOT NULL UNIQUE,
  driver_id UUID NOT NULL REFERENCES drivers(id),
  pit_id UUID NOT NULL REFERENCES pits(id),
  status waybill_status NOT NULL DEFAULT 'pending_dispatch',
  queue_number INTEGER,
  estimated_weight_ton NUMERIC(10,2),
  actual_weight_ton NUMERIC(10,2),
  dispatch_time TIMESTAMPTZ,
  arrive_time TIMESTAMPTZ,
  queue_enter_time TIMESTAMPTZ,
  queue_exit_time TIMESTAMPTZ,
  load_start_time TIMESTAMPTZ,
  load_end_time TIMESTAMPTZ,
  weigh_start_time TIMESTAMPTZ,
  completed_time TIMESTAMPTZ,
  cancelled_time TIMESTAMPTZ,
  cancelled_reason TEXT,
  created_by UUID REFERENCES users(id),
  cancelled_by UUID REFERENCES users(id),
  version INTEGER NOT NULL DEFAULT 1,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT waybills_queue_number_non_negative CHECK (
    queue_number IS NULL OR queue_number > 0
  ),
  CONSTRAINT waybills_cancelled_reason_required CHECK (
    status <> 'cancelled' OR cancelled_reason IS NOT NULL
  )
);

CREATE TABLE IF NOT EXISTS queue_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pit_id UUID NOT NULL REFERENCES pits(id),
  driver_id UUID NOT NULL REFERENCES drivers(id),
  waybill_id UUID NOT NULL REFERENCES waybills(id),
  enter_queue_time TIMESTAMPTZ NOT NULL,
  exit_queue_time TIMESTAMPTZ,
  queue_position INTEGER NOT NULL,
  is_manual_adjustment BOOLEAN NOT NULL DEFAULT FALSE,
  adjustment_reason TEXT,
  created_by UUID REFERENCES users(id),
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT queue_logs_position_positive CHECK (queue_position > 0)
);

CREATE TABLE IF NOT EXISTS loading_records (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  waybill_id UUID NOT NULL UNIQUE REFERENCES waybills(id),
  pit_id UUID NOT NULL REFERENCES pits(id),
  operator_id UUID REFERENCES users(id),
  start_time TIMESTAMPTZ NOT NULL,
  end_time TIMESTAMPTZ,
  loader_name VARCHAR(64),
  notes TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS weigh_records (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  waybill_id UUID NOT NULL UNIQUE REFERENCES waybills(id),
  gross_weight_ton NUMERIC(10,2) NOT NULL,
  tare_weight_ton NUMERIC(10,2),
  net_weight_ton NUMERIC(10,2) NOT NULL,
  weigh_time TIMESTAMPTZ NOT NULL,
  operator_id UUID REFERENCES users(id),
  source VARCHAR(32) NOT NULL DEFAULT 'manual',
  note TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  CONSTRAINT weigh_records_weight_positive CHECK (
    gross_weight_ton >= 0 AND
    COALESCE(tare_weight_ton, 0) >= 0 AND
    net_weight_ton >= 0
  )
);

CREATE TABLE IF NOT EXISTS exception_records (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  waybill_id UUID NOT NULL REFERENCES waybills(id),
  type exception_type NOT NULL,
  severity SMALLINT NOT NULL DEFAULT 1,
  description TEXT NOT NULL,
  status VARCHAR(32) NOT NULL DEFAULT 'open',
  reported_by UUID REFERENCES users(id),
  resolved_by UUID REFERENCES users(id),
  resolved_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS operation_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  entity_type VARCHAR(32) NOT NULL,
  entity_id UUID NOT NULL,
  action VARCHAR(64) NOT NULL,
  before_data JSONB,
  after_data JSONB,
  operator_id UUID REFERENCES users(id),
  operator_name VARCHAR(64),
  reason TEXT,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE UNIQUE INDEX IF NOT EXISTS ux_waybills_driver_active
  ON waybills(driver_id)
  WHERE status IN (
    'pending_dispatch',
    'dispatched',
    'arrived',
    'queueing',
    'loading',
    'loaded',
    'weighing'
  );

CREATE INDEX IF NOT EXISTS idx_waybills_pit_status
  ON waybills(pit_id, status);

CREATE INDEX IF NOT EXISTS idx_waybills_driver_status
  ON waybills(driver_id, status);

CREATE INDEX IF NOT EXISTS idx_waybills_dispatch_time
  ON waybills(dispatch_time);

CREATE INDEX IF NOT EXISTS idx_queue_logs_pit_enter_time
  ON queue_logs(pit_id, enter_queue_time);

CREATE INDEX IF NOT EXISTS idx_exception_records_waybill
  ON exception_records(waybill_id, type, status);

-- ═══════════════════════════════════════════════════════════════
-- 无人矿卡任务系统
-- ═══════════════════════════════════════════════════════════════

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'mission_status') THEN
    CREATE TYPE mission_status AS ENUM (
      'pending', 'claimed', 'in_progress', 'completed', 'failed', 'cancelled'
    );
  END IF;
END
$$;

-- 无人矿卡任务表
CREATE TABLE IF NOT EXISTS missions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  waybill_id UUID REFERENCES waybills(id) ON DELETE SET NULL,
  vehicle_id VARCHAR(64) NOT NULL,
  mission_type VARCHAR(32) NOT NULL,          -- loading / hauling / dumping
  source_pit_id UUID NOT NULL REFERENCES pits(id),
  destination VARCHAR(256) NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  status mission_status NOT NULL DEFAULT 'pending',
  estimated_weight_ton DOUBLE PRECISION,
  actual_weight_ton DOUBLE PRECISION,
  params JSONB,                                -- 无人驾驶系统自定义参数
  error_message TEXT,
  claimed_by VARCHAR(64),                      -- 无人车标识
  claimed_at TIMESTAMPTZ,
  started_at TIMESTAMPTZ,
  completed_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_missions_status ON missions(status);
CREATE INDEX IF NOT EXISTS idx_missions_vehicle ON missions(vehicle_id);
CREATE INDEX IF NOT EXISTS idx_missions_waybill ON missions(waybill_id);
CREATE INDEX IF NOT EXISTS idx_missions_pit ON missions(source_pit_id);

-- 无人车状态日志表（轨迹+载重+电量）
CREATE TABLE IF NOT EXISTS mission_status_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  mission_id UUID NOT NULL REFERENCES missions(id) ON DELETE CASCADE,
  status VARCHAR(32) NOT NULL,
  position_lng DOUBLE PRECISION,
  position_lat DOUBLE PRECISION,
  payload_weight DOUBLE PRECISION,
  battery_level REAL,
  error_message TEXT,
  estimated_completion TIMESTAMPTZ,
  reported_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_mission_logs_mission ON mission_status_logs(mission_id, reported_at);

-- ═══════════════════════════════════════════════════════════════
-- 离线调度支持
-- ═══════════════════════════════════════════════════════════════

-- 幂等键表（离线操作去重）
CREATE TABLE IF NOT EXISTS idempotency_keys (
  key VARCHAR(64) PRIMARY KEY,
  result TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires ON idempotency_keys(expires_at);

-- 在 waybills 表上增加版本号（离线乐观锁）
ALTER TABLE waybills ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;

-- 增加到场来源字段（用于区分手动/离线/车牌识别/电子围栏）
ALTER TABLE waybills ADD COLUMN IF NOT EXISTS arrival_source VARCHAR(32) DEFAULT 'manual';

-- 注意: drivers 表已包含 license_plate 字段，无需单独的 vehicles 表
