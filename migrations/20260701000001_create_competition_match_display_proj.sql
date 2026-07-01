CREATE TABLE competition_match_display_proj (
    pairing_id          TEXT PRIMARY KEY,
    season_id           TEXT NOT NULL,
    round_id            TEXT NOT NULL,
    round_name          TEXT NOT NULL,
    round_position      INTEGER NOT NULL,
    round_date_start    TEXT,
    round_date_end      TEXT,
    round_day_type      TEXT NOT NULL,
    home_team_id        TEXT NOT NULL,
    home_team_name      TEXT NOT NULL,
    home_roster_name    TEXT NOT NULL,
    home_coach_name     TEXT NOT NULL,
    home_logo_url       TEXT,
    home_initials       TEXT NOT NULL,
    away_team_id        TEXT NOT NULL,
    away_team_name      TEXT NOT NULL,
    away_roster_name    TEXT NOT NULL,
    away_coach_name     TEXT NOT NULL,
    away_logo_url       TEXT,
    away_initials       TEXT NOT NULL,
    match_status        TEXT NOT NULL DEFAULT 'upcoming',
    home_score          INTEGER,
    away_score          INTEGER,
    home_casualties     INTEGER,
    away_casualties     INTEGER,
    match_report_id     TEXT,
    match_report_url    TEXT
);

CREATE INDEX idx_cmdp_season_status
    ON competition_match_display_proj (season_id, match_status);

CREATE INDEX idx_cmdp_season_position
    ON competition_match_display_proj (season_id, round_position);
