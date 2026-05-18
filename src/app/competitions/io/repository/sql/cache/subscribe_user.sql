INSERT INTO competitions__user_space_cache (space_id, coach_id, profile)
VALUES ($1, $2, $3)
ON CONFLICT (space_id, coach_id) DO UPDATE SET profile = EXCLUDED.profile