-- Sans `status`, contrairement à `update_structure` et `update_invitations` :
-- régler une notification n'est pas une étape du magicien. Y écrire un statut
-- ferait retomber dans le parcours de création une compétition en cours de
-- saison, dont l'organisateur ne fait que changer un réglage.
UPDATE competition_seasons
SET    notifications = $1::jsonb
WHERE  id            = $2
RETURNING id
