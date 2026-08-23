-- Les membres d'un espace, avec leur profil.
--
-- C'est `list_members_for_space.sql` plus `m.profile`. Une requête distincte
-- plutôt qu'une colonne ajoutée à l'autre : son appelant, le sélecteur de
-- coachs, n'a que faire du profil.
--
-- Les coachs viennent du cache de ce BC, jamais de la table des comptes du BC
-- d'authentification — `spaces` est extractible.
SELECT u.id AS coach_id, u.coach_name, u.email, u.coach_icon AS icon, m.profile
FROM   spaces__user_cache u
JOIN   spaces__user_space m ON m.coach_id = u.id
WHERE  m.space_id = $1
ORDER BY u.coach_name
