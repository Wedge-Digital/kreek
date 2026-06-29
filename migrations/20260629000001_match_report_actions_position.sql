ALTER TABLE match_report_actions
    ADD COLUMN IF NOT EXISTS player_position TEXT NOT NULL DEFAULT '';
