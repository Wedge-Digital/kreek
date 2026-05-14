SELECT s.id, s.space_name, s.space_icon_path
FROM spaces s
INNER JOIN spaces__user_space us ON us.space_id = s.id
WHERE us.coach_id = $1
ORDER BY s.space_name