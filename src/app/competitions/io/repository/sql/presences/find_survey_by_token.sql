-- La campagne à laquelle un jeton appartient.
--
-- **Aucune colonne d'expiration consultée, parce qu'il n'y en a pas.** R7 refuse
-- au jeton toute échéance propre : sa validité se lit sur l'état de la campagne,
-- que `statut_de` calcule depuis l'échéance et la clôture décidée (R23). Le jeton
-- est un pointeur vers une réponse, rien de plus — et c'est ce qui fait que
-- rouvrir une campagne réarme les anciens liens sans rien réémettre.
SELECT s.id, s.season_id, s.round_id, s.deadline, s.auto_remind, s.opened_at,
       s.close_le, s.exemptee, s.appariee
FROM competition_presence_surveys s
JOIN competition_presence_answers a ON a.survey_id = s.id
WHERE a.token = $1
