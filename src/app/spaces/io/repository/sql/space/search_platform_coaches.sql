-- Coachs de l'annuaire de la plateforme, marqués selon qu'ils sont déjà membres
-- de l'espace.
--
-- `m.space_id = $1` est dans la **condition de jointure**, jamais dans le
-- `WHERE`. L'y déplacer transformerait la jointure externe en jointure interne :
-- la recherche ne rendrait plus que les membres, c'est-à-dire l'exact inverse du
-- besoin — sans erreur, sans exception, avec une liste qui a l'air d'une liste.
--
-- Les membres sont **rendus, pas exclus**. Les exclure laisserait croire qu'un
-- coach n'existe pas alors qu'il est déjà là, et l'administrateur chercherait à
-- créer un compte qui existe.
SELECT u.id                        AS coach_id,
       u.coach_name,
       u.email,
       u.coach_icon                AS icon,
       (m.coach_id IS NOT NULL)    AS est_membre
FROM   spaces__user_cache u
LEFT   JOIN spaces__user_space m
       ON m.coach_id = u.id AND m.space_id = $1
WHERE  u.coach_name ILIKE $2 OR u.email ILIKE $2
ORDER  BY u.coach_name
LIMIT  $3
