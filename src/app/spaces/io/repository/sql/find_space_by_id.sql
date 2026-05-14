SELECT
    s.id              AS space_id,
    s.space_name,
    s.space_icon_path,
    u.id              AS coach_id,
    u.coach_name,
    u.coach_icon,
    us.profile
FROM spaces s
LEFT JOIN spaces__user_space us ON us.space_id = s.id
LEFT JOIN auth__users u ON u.id = us.coach_id
WHERE s.id = $1