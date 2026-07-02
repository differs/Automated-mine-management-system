-- ═══════════════════════════════════════════════════════════════
-- Migration 0005: 电子围栏
-- ═══════════════════════════════════════════════════════════════

-- 电子围栏表
CREATE TABLE IF NOT EXISTS geo_fences (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  pit_id UUID NOT NULL REFERENCES pits(id) ON DELETE CASCADE,
  name VARCHAR(100) NOT NULL,
  fence_type VARCHAR(20) NOT NULL DEFAULT 'arrival',
  -- arrival: 到场围栏（进入自动签到）
  -- geofence: 通用围栏
  -- restricted: 禁入区域
  shape VARCHAR(20) NOT NULL DEFAULT 'circle',
  -- circle: 圆形（center_lat + center_lng + radius）
  -- polygon: 多边形（polygon_points）
  center_lat DOUBLE PRECISION NOT NULL,
  center_lng DOUBLE PRECISION NOT NULL,
  radius_meters DOUBLE PRECISION NOT NULL DEFAULT 100,
  polygon_points JSONB,
  is_active BOOLEAN NOT NULL DEFAULT true,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_geo_fences_pit ON geo_fences(pit_id);
CREATE INDEX IF NOT EXISTS idx_geo_fences_active ON geo_fences(pit_id, is_active);

-- 司机围栏状态
CREATE TABLE IF NOT EXISTS driver_fence_states (
  driver_id UUID NOT NULL REFERENCES drivers(id) ON DELETE CASCADE,
  fence_id UUID NOT NULL REFERENCES geo_fences(id) ON DELETE CASCADE,
  inside BOOLEAN NOT NULL DEFAULT false,
  entered_at TIMESTAMPTZ,
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  PRIMARY KEY (driver_id, fence_id)
);

-- 围栏事件日志
CREATE TABLE IF NOT EXISTS fence_event_logs (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  driver_id UUID NOT NULL REFERENCES drivers(id),
  fence_id UUID NOT NULL REFERENCES geo_fences(id),
  event_type VARCHAR(20) NOT NULL,  -- enter / exit
  lat DOUBLE PRECISION NOT NULL,
  lng DOUBLE PRECISION NOT NULL,
  occurred_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_fence_events_driver ON fence_event_logs(driver_id, occurred_at);
CREATE INDEX IF NOT EXISTS idx_fence_events_fence ON fence_event_logs(fence_id, occurred_at);

-- 位置上报记录（轨迹）
CREATE TABLE IF NOT EXISTS location_reports (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  driver_id UUID NOT NULL REFERENCES drivers(id) ON DELETE CASCADE,
  lat DOUBLE PRECISION NOT NULL,
  lng DOUBLE PRECISION NOT NULL,
  accuracy REAL DEFAULT 0,
  speed REAL DEFAULT 0,
  bearing REAL,
  reported_at TIMESTAMPTZ NOT NULL,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX IF NOT EXISTS idx_location_reports_driver ON location_reports(driver_id, reported_at);
-- 定期清理 7 天前的轨迹数据
