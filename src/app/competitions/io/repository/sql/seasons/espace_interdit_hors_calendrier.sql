-- Une compétition de l'espace interdit-elle les matchs hors calendrier ?
--
-- `DISTINCT ON` retient **la dernière saison de chaque compétition**, au même
-- critère que `find_latest_season_id` : sans cela, une saison archivée qui
-- interdisait ferait disparaître l'entrée de menu pour toujours.
--
-- La comparaison est faite sur le texte JSON et non sur un booléen converti :
-- `options` est `NULL` sur toutes les saisons antérieures à la carte 550, et
-- `NULL ->> 'clé'` vaut `NULL`, qui n'est pas `'false'`. Une saison jamais
-- réglée n'interdit donc rien — exactement le défaut du domaine.
SELECT EXISTS (
    SELECT 1
    FROM (
        SELECT DISTINCT ON (s.competition_id) s.options
        FROM   competition_seasons s
        JOIN   competitions c ON c.id = s.competition_id
        WHERE  c.space_id = $1
        ORDER  BY s.competition_id, s.created_at DESC
    ) derniere
    WHERE derniere.options ->> 'autorise_hors_calendrier' = 'false'
)
