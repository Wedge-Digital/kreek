-- Confirme l'envoi. Tant que ce `UPDATE` n'a pas eu lieu, la ligne réserve le
-- créneau sans attester de rien : c'est un échec constaté, pas un envoi.
UPDATE competition_notification_deliveries
SET    sent_at = now()
WHERE  notification_type = $1
  AND  season_id         = $2
  AND  COALESCE(round_id, '') = COALESCE($3, '')
  AND  target_date       = $4
  AND  coach_id          = $5
