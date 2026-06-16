SELECT id, coach_name, coach_icon, email, password_hash
FROM auth__users
WHERE legacy_id = $1
