CREATE TABLE IF NOT EXISTS team_enrollment_projection (
    team_id         TEXT PRIMARY KEY,
    space_id        TEXT NOT NULL,
    team_name       TEXT NOT NULL,
    coach_name      TEXT NOT NULL,
    roster_name     TEXT NOT NULL,
    competition_id  TEXT,
    season_id       TEXT,
    status          TEXT NOT NULL DEFAULT 'PendingEnrollment',
    updated_at      TIMESTAMPTZ NOT NULL DEFAULT now()
);

CREATE INDEX IF NOT EXISTS idx_team_enrollment_season_status
    ON team_enrollment_projection (season_id, status);
