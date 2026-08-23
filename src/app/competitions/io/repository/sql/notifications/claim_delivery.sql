-- Réserve le créneau d'envoi. Zéro ligne rendue signifie « déjà envoyé » :
-- c'est la base qui tranche, pas le code, et c'est tout R3.
--
-- `ON CONFLICT` sans nommer de contrainte mais en répétant l'expression de
-- l'index : `COALESCE(round_id, '')` en fait un index sur expression, que
-- PostgreSQL ne rattache à aucun nom de colonne.
INSERT INTO competition_notification_deliveries
       (notification_type, season_id, round_id, target_date, coach_id)
VALUES ($1, $2, $3, $4, $5)
ON CONFLICT (notification_type, season_id, COALESCE(round_id, ''), target_date, coach_id)
DO NOTHING
RETURNING 1
