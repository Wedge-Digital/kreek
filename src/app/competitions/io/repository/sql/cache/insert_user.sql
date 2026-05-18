INSERT INTO competitions__user_cache (id, coach_name, coach_icon, email)
VALUES ($1, $2, $3, $4)
ON CONFLICT (id) DO NOTHING