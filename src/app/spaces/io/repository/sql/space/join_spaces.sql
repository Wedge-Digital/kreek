INSERT INTO spaces__user_space (space_id, coach_id, profile)
SELECT UNNEST($1::text[]), $2, 'SpaceUser'
ON CONFLICT (space_id, coach_id) DO NOTHING