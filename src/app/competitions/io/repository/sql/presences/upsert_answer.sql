-- Une réponse. Le conflit porte sur `(survey_id, team_id)` et non sur `id` :
-- c'est la clé métier de R1, et c'est elle qu'une réécriture doit retrouver.
--
-- Le jeton n'est **pas** réécrit : le réattribuer invaliderait un lien déjà
-- parti par e-mail, alors que R7 veut qu'il réponde tant que la campagne vit.
INSERT INTO competition_presence_answers
    (id, survey_id, team_id, coach_id, token, presence, repondu_le, saisi_par_admin)
VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
ON CONFLICT (survey_id, team_id) DO UPDATE SET
    presence        = EXCLUDED.presence,
    repondu_le      = EXCLUDED.repondu_le,
    saisi_par_admin = EXCLUDED.saisi_par_admin
