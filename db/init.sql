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
