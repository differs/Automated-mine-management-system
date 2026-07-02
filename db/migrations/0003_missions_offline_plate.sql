-- ═══════════════════════════════════════════════════════════════
-- Migration 0003: 无人矿卡任务 + 离线调度 + 车牌识别
-- ═══════════════════════════════════════════════════════════════

-- 无人矿卡任务
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'mission_status') THEN
    CREATE TYPE mission_status AS ENUM (
      'pending', 'claimed', 'in_progress', 'completed', 'failed', 'cancelled'
    );
  END IF;
END
$$;

CREATE TABLE IF NOT EXISTS missions (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  waybill_id UUID REFERENCES waybills(id) ON DELETE SET NULL,
  vehicle_id VARCHAR(64) NOT NULL,
  mission_type VARCHAR(32) NOT NULL,
  source_pit_id UUID NOT NULL REFERENCES pits(id),
  destination VARCHAR(256) NOT NULL,
  priority INTEGER NOT NULL DEFAULT 0,
  status mission_status NOT NULL DEFAULT 'pending',
  estimated_weight_ton DOUBLE PRECISION,
  actual_weight_ton DOUBLE PRECISION,
  params JSONB,
  error_message TEXT,
  claimed_by VARCHAR(64),
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

-- 离线调度
CREATE TABLE IF NOT EXISTS idempotency_keys (
  key VARCHAR(64) PRIMARY KEY,
  result TEXT NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  expires_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_idempotency_keys_expires ON idempotency_keys(expires_at);

ALTER TABLE waybills ADD COLUMN IF NOT EXISTS version INTEGER NOT NULL DEFAULT 1;
ALTER TABLE waybills ADD COLUMN IF NOT EXISTS arrival_source VARCHAR(32) DEFAULT 'manual';
-- 注意: drivers 表已包含 license_plate 字段，无需单独的 vehicles 表
