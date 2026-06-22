CREATE TABLE IF NOT EXISTS competition_match_days (
    id          TEXT PRIMARY KEY,
    season_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    day_type    TEXT NOT NULL DEFAULT 'time_frame',
    date_start  TEXT,
    date_end    TEXT,
    position    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS competition_match_day_pairings (
    id              TEXT PRIMARY KEY,
    match_day_id    TEXT NOT NULL REFERENCES competition_match_days(id) ON DELETE CASCADE,
    home_team_id    TEXT NOT NULL,
    away_team_id    TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_match_days_season
    ON competition_match_days (season_id);

CREATE INDEX IF NOT EXISTS idx_pairings_match_day
    ON competition_match_day_pairings (match_day_id);
