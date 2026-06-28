ALTER TABLE match_report_projection
    ADD COLUMN IF NOT EXISTS home_temp_players JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN IF NOT EXISTS away_temp_players JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE TABLE IF NOT EXISTS match_report_actions (
    action_id           TEXT        PRIMARY KEY,
    match_report_id     TEXT        NOT NULL,
    team_side           TEXT        NOT NULL,
    turn_number         SMALLINT    NOT NULL,
    player_id           TEXT        NOT NULL,
    player_type         TEXT        NOT NULL,
    action_json         JSONB       NOT NULL,
    player_display_name TEXT        NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_deleted          BOOLEAN     NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_mr_actions_mr_side
    ON match_report_actions (match_report_id, team_side)
    WHERE NOT is_deleted;
