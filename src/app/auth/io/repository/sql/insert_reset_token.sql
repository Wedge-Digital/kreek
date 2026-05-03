INSERT INTO lost_login_token (token, coach_name, created_at)
VALUES ($1, $2, NOW())
ON CONFLICT (token) DO UPDATE SET created_at = NOW()