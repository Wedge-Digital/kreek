-- Quatre espaces peuplés n'avaient aucun administrateur.
--
-- `is_admin()` y était donc faux pour tous leurs membres : personne ne pouvait
-- les administrer, et la page d'administration leur aurait été inaccessible de
-- façon définitive. Constaté le 2026-08-23, sur trente-quatre espaces — les huit
-- autres sans administrateur n'ont aucun membre, ce qui est sans conséquence.
--
-- Bagouze était `SpaceUser` dans exactement ces quatre espaces, et dans aucun
-- autre espace cassé. Le promouvoir là où il est **déjà membre** suffit donc à
-- tous les réparer, sans le faire entrer dans des espaces qu'il n'a jamais
-- rejoints.
--
-- Par `coach_name` et non par identifiant : le ULID diffère d'un environnement
-- à l'autre. La requête est idempotente, et ne fait rien là où le compte
-- n'existe pas.
--
-- Ce que cette migration ne fait pas : garantir la propriété dans le temps.
-- Elle répare l'état du jour. Ce qui empêche la régression est l'invariant posé
-- au même moment sur l'agrégat `Space` (carte 365), qui refuse désormais de
-- retirer ou de rétrograder le dernier administrateur d'un espace.
UPDATE spaces__user_space m
SET    profile = 'SpaceAdmin'
FROM   auth__users u
WHERE  u.id = m.coach_id
AND    u.coach_name = 'Bagouze'
AND    m.profile <> 'SpaceAdmin';
