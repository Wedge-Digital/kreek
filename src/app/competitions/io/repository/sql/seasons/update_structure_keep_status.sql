-- Écrit la structure **sans toucher au statut**, contrairement à
-- `update_structure.sql` qui pose `status = 'structure_selected'`.
--
-- Ce dernier sert le magicien de création, où réécrire la structure fait
-- avancer la saison d'une étape. Ici la saison est **en cours** : la ramener à
-- `structure_selected` la ferait régresser sous `ready`, et la carte 407
-- interdit la création d'équipe sur une saison qui ne l'est pas. Modifier une
-- poule aurait cassé l'inscription de la compétition entière, sans un mot.
UPDATE competition_seasons
SET    structure = $1::jsonb
WHERE  id        = $2
RETURNING id
