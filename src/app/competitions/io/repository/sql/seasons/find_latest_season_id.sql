SELECT id
FROM   competition_seasons
WHERE  competition_id = $1
ORDER BY created_at DESC
LIMIT 1