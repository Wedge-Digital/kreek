INSERT INTO spaces__user_cache (id, coach_name, coach_icon, email)
VALUES ($1, $2, NULL, $3)
ON CONFLICT (id) DO NOTHING