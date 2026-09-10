-- De quoi titrer la page publique : la journée, la compétition, l'espace.
--
-- **Trois jointures, toutes dans `competitions`.** Le nom de l'équipe vient
-- d'`ITeamInfoPort` et jamais d'une jointure vers les tables de `teams` : ce
-- serait l'exacte violation que la souveraineté des données entre BCs nomme.
--
-- Les dates sont rendues **brutes**. Composer « Du 12 au 19 octobre » ici mettrait
-- le libellé à deux endroits — la couche web en a déjà un — et un DTO de lecture
-- porte des données, pas des libellés.
SELECT d.name  AS round_name,
       d.date_start AS round_date_start,
       d.date_end   AS round_date_end,
       c.id    AS competition_id,
       c.name  AS competition_name,
       se.id   AS season_id,
       c.space_id
FROM competition_presence_answers a
JOIN competition_presence_surveys s ON s.id = a.survey_id
JOIN competition_match_days d       ON d.id = s.round_id
JOIN competition_seasons se         ON se.id = d.season_id
JOIN competitions c                 ON c.id = se.competition_id
WHERE a.token = $1
