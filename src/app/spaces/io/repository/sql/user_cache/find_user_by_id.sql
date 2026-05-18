SELECT id, coach_name, coach_icon, email, created_at
FROM spaces__user_cache
WHERE id = $1
ORDER BY coach_name