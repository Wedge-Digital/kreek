-- Les réponses d'une campagne, pour la réhydratation de l'agrégat.
--
-- Ordonnées par équipe : sans `ORDER BY`, PostgreSQL rend les lignes dans
-- l'ordre où il les trouve, et deux lectures de la même campagne donneraient
-- deux ordres d'affichage.
SELECT id, team_id, coach_id, token, presence, repondu_le, saisi_par_admin
FROM competition_presence_answers
WHERE survey_id = $1
ORDER BY team_id
