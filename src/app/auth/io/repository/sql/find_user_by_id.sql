SELECT id, coach_name, coach_icon, email, password_hash
FROM users
WHERE id = $1