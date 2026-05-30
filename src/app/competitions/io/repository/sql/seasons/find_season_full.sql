SELECT
    s.id         AS season_id,
    s.name       AS season_name,
    s.status,
    s.rules,
    s.structure,
    c.id         AS competition_id,
    c.name       AS competition_name
FROM competition_seasons s
JOIN competitions c ON c.id = s.competition_id
WHERE s.id = $1