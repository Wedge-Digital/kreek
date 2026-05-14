SELECT token, coach_name, created_at
FROM auth__lost_login_token
WHERE token = $1