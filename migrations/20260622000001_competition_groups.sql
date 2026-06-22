CREATE TABLE IF NOT EXISTS competition_groups (
    id          TEXT PRIMARY KEY,
    season_id   TEXT NOT NULL,
    name        TEXT NOT NULL,
    position    INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE IF NOT EXISTS competition_group_teams (
    group_id    TEXT NOT NULL REFERENCES competition_groups(id) ON DELETE CASCADE,
    team_id     TEXT NOT NULL,
    PRIMARY KEY (group_id, team_id)
);

CREATE INDEX IF NOT EXISTS idx_competition_groups_season
    ON competition_groups (season_id);
