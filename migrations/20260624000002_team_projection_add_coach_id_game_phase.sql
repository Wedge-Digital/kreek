ALTER TABLE team_projection ADD COLUMN IF NOT EXISTS coach_id TEXT NOT NULL DEFAULT '';
ALTER TABLE team_projection ADD COLUMN IF NOT EXISTS game_phase TEXT;

UPDATE team_projection SET coach_id = '' WHERE coach_id = '';
