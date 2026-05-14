UPDATE auth__users
SET password_hash = $1
WHERE coach_name = $2