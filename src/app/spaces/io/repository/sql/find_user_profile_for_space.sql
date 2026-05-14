SELECT space_id, coach_id, profile, created_at
FROM spaces__user_space
WHERE  coach_id= $1 and space_id = $2