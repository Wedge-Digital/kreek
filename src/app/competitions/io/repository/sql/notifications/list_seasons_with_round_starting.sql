-- Les saisons ayant une journée non `rest` qui démarre à la date donnée.
--
-- Bornée par la date, jamais parcourue en entier : le coût du cron reste
-- indépendant du nombre de saisons historiques.
SELECT DISTINCT s.id AS season_id, c.id AS competition_id, c.space_id,
                sp.space_name, c.name AS competition_name, s.name AS season_name
FROM   competition_match_days d
JOIN   competition_seasons s ON s.id = d.season_id
JOIN   competitions c        ON c.id = s.competition_id
JOIN   spaces sp             ON sp.id = c.space_id
WHERE  d.date_start = $1
  AND  d.day_type <> 'rest'
