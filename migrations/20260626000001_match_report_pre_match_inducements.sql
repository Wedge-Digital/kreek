ALTER TABLE match_report_projection
    ADD COLUMN IF NOT EXISTS home_team_value  INTEGER,
    ADD COLUMN IF NOT EXISTS away_team_value  INTEGER,
    ADD COLUMN IF NOT EXISTS home_inducements JSONB,
    ADD COLUMN IF NOT EXISTS away_inducements JSONB;
