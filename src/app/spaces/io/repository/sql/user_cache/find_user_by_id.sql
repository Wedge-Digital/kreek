SELECT id, coach_name, coach_icon, email, created_at
FROM space__user_cache
WHERE coach_id = :coach_id
ORDER BY coach_name