-- ═══════════════════════════════════════════════════════════════
-- Migration 0004: 地磅自动采集
-- ═══════════════════════════════════════════════════════════════

-- 地磅设备表
CREATE TABLE IF NOT EXISTS scale_devices (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pit_id UUID NOT NULL REFERENCES pits(id),
  device_name VARCHAR(100) NOT NULL,
  device_type VARCHAR(20) NOT NULL DEFAULT 'serial',
  -- serial / bluetooth / network
  connection_config JSONB NOT NULL DEFAULT '{}',
  -- serial: {port: "/dev/ttyS0", baud: 9600}
  -- bluetooth: {address: "00:11:22:33:44:55", name: "Scale-01"}
  is_active BOOLEAN NOT NULL DEFAULT true,
  last_heartbeat_at TIMESTAMPTZ,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_scale_devices_pit ON scale_devices(pit_id);

-- 称重原始日志（防篡改审计）
CREATE TABLE IF NOT EXISTS weigh_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  weighing_id UUID REFERENCES weigh_records(id) ON DELETE CASCADE,
  device_id UUID REFERENCES scale_devices(id),
  weight DOUBLE PRECISION NOT NULL,
  raw_data TEXT NOT NULL,
  is_stable BOOLEAN NOT NULL DEFAULT false,
  read_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_weigh_logs_weighing ON weigh_logs(weighing_id);

-- 称重记录增加来源字段
ALTER TABLE weigh_records ADD COLUMN IF NOT EXISTS source VARCHAR(20) DEFAULT 'manual';
ALTER TABLE weigh_records ADD COLUMN IF NOT EXISTS note TEXT;

-- 运单增加 vehicle_id（引用 drivers 表，车辆即司机）
ALTER TABLE waybills ADD COLUMN IF NOT EXISTS vehicle_id UUID REFERENCES drivers(id);
ALTER TABLE waybills ADD COLUMN IF NOT EXISTS completed_time TIMESTAMPTZ;
