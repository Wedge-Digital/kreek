SELECT c.id,
       c.name,
       c.logo,
       s.id     AS season_id,
       s.status AS status,
       (SELECT COUNT(*) FROM competition_seasons WHERE competition_id = c.id) AS season_count
FROM   competitions c
LEFT JOIN LATERAL (
    SELECT id, status
    FROM   competition_seasons
    WHERE  competition_id = c.id
    ORDER  BY created_at DESC
    LIMIT  1
) s ON true
WHERE  c.space_id = $1
ORDER  BY c.created_at DESC
