-- Sans `status`, contrairement à `update_structure` et `update_invitations` :
-- autoriser ou non les matchs hors calendrier n'est pas une étape du magicien.
-- Y écrire un statut ferait retomber dans le parcours de création une
-- compétition en cours de saison, dont l'organisateur ne fait que changer un
-- réglage.
--
-- Et seule la colonne `options` est touchée : les quatre autres blobs se
-- lisent-modifient-réécrivent en entier, donc les inclure ici écraserait une
-- modification faite au même moment dans un autre onglet.
UPDATE competition_seasons
SET    options = $1::jsonb
WHERE  id      = $2
RETURNING id
