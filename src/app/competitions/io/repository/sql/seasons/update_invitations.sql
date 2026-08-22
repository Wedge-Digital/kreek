-- Les deux colonnes en une seule instruction, depuis la carte 333 : l'étape 4
-- du magicien enregistre d'un bloc, et les réglages de notification y sont
-- désormais rendus par un widget en mode différé. Deux `UPDATE` séparés
-- laisseraient une fenêtre où l'un a réussi et l'autre non.
--
-- Le `status` reste écrit, contrairement à `update_notifications.sql` : ici on
-- est bien dans une étape du magicien, et c'est elle qui fait avancer la saison.
UPDATE competition_seasons
SET    invitations   = $1::jsonb,
       notifications = $2::jsonb,
       status        = 'invitations_configured'
WHERE  id            = $3
RETURNING id
