-- L'état de chaque journée de la saison, en une requête.
--
-- `LEFT JOIN` et non `JOIN` : une journée sans campagne doit apparaître dans la
-- barre latérale, avec ses compteurs à zéro. La faire disparaître laisserait
-- croire à un trou dans le calendrier.
--
-- Aucun statut n'est calculé ici : R23 le déduit de `deadline` et `close_le`,
-- que cette requête rend bruts. Le produire en SQL mettrait la règle à deux
-- endroits.
SELECT d.id                                             AS round_id,
       d.name                                           AS round_name,
       d.position                                       AS round_position,
       (d.day_type = 'rest')                            AS is_rest,
       s.deadline                                       AS deadline,
       s.close_le                                       AS close_le,
       s.appariee                                       AS appariee,
       COUNT(a.id)                                      AS attendues,
       COUNT(a.id) FILTER (WHERE a.presence <> 'sans_reponse') AS reponses,
       COUNT(a.id) FILTER (WHERE a.presence = 'presente')      AS presents
FROM competition_match_days d
LEFT JOIN competition_presence_surveys s ON s.round_id = d.id
LEFT JOIN competition_presence_answers a ON a.survey_id = s.id
WHERE d.season_id = $1
GROUP BY d.id, d.name, d.position, d.day_type, s.deadline, s.close_le, s.appariee
ORDER BY d.position
