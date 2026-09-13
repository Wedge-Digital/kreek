-- Les réponses de **plusieurs** campagnes, en une requête.
--
-- Le pendant pluriel de `find_answers_by_survey.sql`, et il existe pour une seule
-- raison : sans lui, lire N campagnes coûterait `1 + N` allers-retours, ce que la
-- requête de liste cherchait précisément à éviter. Le déplacer d'un cran n'aurait
-- rien réglé.
--
-- `survey_id` est rendu en plus des colonnes du singulier : c'est lui qui permet
-- de regrouper les lignes par campagne à la réhydratation. Sans cette colonne, il
-- faudrait refaire le lien par un second aller-retour, ou le deviner.
--
-- L'ordre porte sur les deux clés : `survey_id` regroupe, `team_id` donne à
-- chaque campagne le même ordre de réponses que la lecture au singulier — deux
-- chemins qui rendraient le même agrégat dans deux ordres seraient deux agrégats.
SELECT survey_id, id, team_id, coach_id, token, presence, repondu_le, saisi_par_admin
FROM competition_presence_answers
WHERE survey_id = ANY($1)
ORDER BY survey_id, team_id
