SELECT space_id, coach_id, profile, created_at
FROM user_space
WHERE space_id = $1