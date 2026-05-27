DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_type WHERE typname = 'waybill_priority') THEN
    CREATE TYPE waybill_priority AS ENUM ('normal', 'urgent', 'vip_override');
  END IF;
END
$$;

CREATE TABLE IF NOT EXISTS fleets (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL UNIQUE,
  company_name VARCHAR(128),
  contact_name VARCHAR(64),
  contact_phone VARCHAR(32),
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS shifts (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  name VARCHAR(64) NOT NULL,
  code VARCHAR(32) NOT NULL UNIQUE,
  starts_at TIME NOT NULL,
  ends_at TIME NOT NULL,
  crosses_day BOOLEAN NOT NULL DEFAULT FALSE,
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS haul_routes (
  id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
  code VARCHAR(32) NOT NULL UNIQUE,
  name VARCHAR(128) NOT NULL,
  pit_id UUID REFERENCES pits(id),
  unload_site_name VARCHAR(128) NOT NULL,
  distance_km NUMERIC(10,2),
  unit_price NUMERIC(10,2),
  is_active BOOLEAN NOT NULL DEFAULT TRUE,
  created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
  updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

ALTER TABLE drivers
  ADD COLUMN IF NOT EXISTS fleet_id UUID REFERENCES fleets(id),
  ADD COLUMN IF NOT EXISTS identity_no VARCHAR(64),
  ADD COLUMN IF NOT EXISTS safety_acknowledged_at TIMESTAMPTZ;

ALTER TABLE waybills
  ADD COLUMN IF NOT EXISTS shift_id UUID REFERENCES shifts(id),
  ADD COLUMN IF NOT EXISTS route_id UUID REFERENCES haul_routes(id),
  ADD COLUMN IF NOT EXISTS priority waybill_priority NOT NULL DEFAULT 'normal',
  ADD COLUMN IF NOT EXISTS manual_override_reason TEXT,
  ADD COLUMN IF NOT EXISTS unload_site_name VARCHAR(128);
