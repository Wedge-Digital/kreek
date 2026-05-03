SELECT token, coach_name, created_at
FROM lost_login_token
WHERE token = $1