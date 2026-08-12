-- Le nom de coach identifie un compte sans distinction de casse : « Bagouze »
-- et « bagouze » désignent le même coach. Un coach qui ne se souvient plus de
-- la casse exacte de son nom pouvait sinon ni se connecter, ni demander la
-- réinitialisation de son mot de passe.
--
-- L'index unique fonctionnel remplace la contrainte exacte, qu'il subsume
-- (deux noms uniques après `lower()` le sont a fortiori tels quels), et sert
-- d'index à la recherche `WHERE lower(coach_name) = lower($1)`.
--
-- Son nom contient `coach_name` : `user_repository::create` s'appuie dessus
-- pour traduire une violation 23505 en `CoachNameAlreadyTaken` plutôt qu'en
-- `EmailAlreadyTaken`.
ALTER TABLE auth__users
    DROP CONSTRAINT users_coach_name_uq;

CREATE UNIQUE INDEX users_coach_name_lower_uq
    ON auth__users (lower(coach_name));