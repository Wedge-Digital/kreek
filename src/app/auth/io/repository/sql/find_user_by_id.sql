SELECT id, coach_name, coach_icon, email, password_hash
FROM auth__users
WHERE id = $1