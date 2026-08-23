-- Les saisons dont la date limite d'inscription vaut la date donnée.
--
-- `<> ''` autant que `IS NOT NULL` : le champ date du magicien rend la chaîne
-- vide quand on l'efface, et `DateString` l'autorise. Sans cette exclusion, une
-- date limite effacée serait comparée comme une date.
SELECT s.id AS season_id, c.id AS competition_id, c.space_id,
       sp.space_name, c.name AS competition_name, s.name AS season_name
FROM   competition_seasons s
JOIN   competitions c ON c.id = s.competition_id
JOIN   spaces sp      ON sp.id = c.space_id
WHERE  s.invitations->>'registration_deadline' = $1
  AND  s.invitations->>'registration_deadline' <> ''
