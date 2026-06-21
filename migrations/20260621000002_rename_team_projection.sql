ALTER TABLE IF EXISTS team_enrollment_projection RENAME TO team_projection;

ALTER TABLE team_projection ADD COLUMN IF NOT EXISTS logo_url TEXT;
ALTER TABLE team_projection ADD COLUMN IF NOT EXISTS team_value INTEGER NOT NULL DEFAULT 0;

DROP INDEX IF EXISTS idx_team_enrollment_season_status;
CREATE INDEX IF NOT EXISTS idx_team_projection_season_status
    ON team_projection (season_id, status);
