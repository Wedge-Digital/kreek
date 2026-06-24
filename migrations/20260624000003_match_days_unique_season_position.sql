DELETE FROM competition_match_days a
USING competition_match_days b
WHERE a.season_id = b.season_id
  AND a.position = b.position
  AND a.id > b.id;

CREATE UNIQUE INDEX IF NOT EXISTS idx_match_days_season_position
    ON competition_match_days (season_id, position);
