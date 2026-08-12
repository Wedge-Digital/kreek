-- `lower()` des deux côtés : le nom de coach identifie un compte sans
-- distinction de casse. L'index unique users_coach_name_lower_uq garantit
-- l'unicité du résultat et sert cette comparaison.
SELECT id, coach_name, coach_icon, email, password_hash
FROM auth__users
WHERE lower(coach_name) = lower($1)