SELECT id, coach_name, email, password_hash
FROM users
WHERE coach_name = $1