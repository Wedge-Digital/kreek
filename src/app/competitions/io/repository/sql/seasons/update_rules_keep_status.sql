-- Écrit le nom et les règles **sans toucher au statut**, contrairement à
-- `update_rules.sql` qui pose `status = 'rules_selected'`.
--
-- Ce dernier sert le magicien de création, où enregistrer les règles fait
-- avancer la saison d'une étape. Ici la saison est **en cours** : la ramener à
-- `rules_selected` la ferait régresser sous `ready`. La carte de la compétition
-- mènerait alors à l'étape 2 du magicien au lieu de la compétition, et la carte
-- 407 interdirait la création d'équipe — un clic sur « Enregistrer » aurait mis
-- la compétition hors service, sans un mot (carte 485).
--
-- Troisième occurrence du même piège, après `update_structure_keep_status.sql`
-- (carte 423) et `save_visibility` (carte 426).
UPDATE competition_seasons
SET    name   = $1,
       rules  = $2::jsonb
WHERE  id     = $3
RETURNING id
