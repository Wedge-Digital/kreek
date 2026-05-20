SELECT u.id, u.coach_name, u.coach_icon, u.email
FROM competitions__user_cache u
JOIN competitions__user_space_cache m ON m.coach_id = u.id
WHERE m.space_id = $1
ORDER BY u.coach_name