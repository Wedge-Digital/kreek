SELECT u.id, u.coach_name, u.coach_icon, u.email
FROM   spaces__user_cache u
JOIN   spaces__user_space m ON m.coach_id = u.id
WHERE  m.space_id = $1
ORDER BY u.coach_name
