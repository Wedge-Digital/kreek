-- Écrit **la seule colonne `invitations`**, sans statut ni notifications,
-- contrairement à `update_invitations.sql`.
--
-- Ce dernier sert l'étape 4 du magicien, où enregistrer les invitations fait
-- avancer la saison et où les réglages de notification se règlent d'un bloc.
-- Ici la saison est **en cours** : y poser `status = 'invitations_configured'`
-- la ferait régresser sous `ready`, et `competition_rules_adapter` ne la dirait
-- plus prête — la carte 407 interdisant la création d'équipe sur une saison qui
-- ne l'est pas, changer le mode d'accès aurait cassé l'inscription de la
-- compétition entière, sans un mot.
--
-- Ne pas écrire `notifications` est aussi ce qui **dispense de les relire** :
-- une colonne qu'on n'écrit pas ne peut pas être remise à son défaut, et les
-- rappels d'échéance ne peuvent donc pas s'éteindre ici.
UPDATE competition_seasons
SET    invitations = $1::jsonb
WHERE  id          = $2
RETURNING id
