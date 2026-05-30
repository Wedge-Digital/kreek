SELECT
    c.id    AS competition_id,
    c.name  AS competition_name,
    s.id    AS season_id,
    s.name  AS season_name,
    s.status
FROM competitions c
JOIN competition_seasons s ON s.competition_id = c.id
WHERE c.space_id = $1
ORDER BY c.name ASC, s.created_at DESC