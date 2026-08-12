-- `lower()` comme à la recherche : le nom vient du token de réinitialisation,
-- donc de la base, mais un jour où il viendrait d'une saisie, une comparaison
-- exacte ne lèverait pas d'erreur — elle mettrait à jour zéro ligne en
-- silence.
UPDATE auth__users
SET password_hash = $1
WHERE lower(coach_name) = lower($2)
