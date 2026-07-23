CREATE TABLE ranking_lines (
    id               TEXT PRIMARY KEY,
    sequence         BIGSERIAL NOT NULL,
    competition_id   TEXT NOT NULL,
    season_id        TEXT NOT NULL,
    round_id         TEXT NOT NULL,
    match_report_id  TEXT NOT NULL,
    team_id          TEXT NOT NULL,
    recorded_at      TIMESTAMPTZ NOT NULL,
    matches_played   INTEGER NOT NULL,
    wins             INTEGER NOT NULL,
    draws            INTEGER NOT NULL,
    losses           INTEGER NOT NULL,
    ranking_points   INTEGER NOT NULL
);

CREATE INDEX idx_ranking_lines_latest
    ON ranking_lines (season_id, team_id, sequence DESC);
