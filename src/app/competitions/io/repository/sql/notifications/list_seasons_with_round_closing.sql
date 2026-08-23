-- Les saisons ayant une journée à **fenêtre temporelle** qui clôt à la date
-- donnée. Le type commande, pas la présence d'une `date_end` : les deux
-- coïncident aujourd'hui, mais s'appuyer sur la seconde ferait dépendre une
-- règle métier d'un invariant de persistance que rien ne garantit.
SELECT DISTINCT s.id AS season_id, c.space_id, c.name AS competition_name, s.name AS season_name
FROM   competition_match_days d
JOIN   competition_seasons s ON s.id = d.season_id
JOIN   competitions c        ON c.id = s.competition_id
WHERE  d.date_end = $1
  AND  d.day_type = 'time_frame'
