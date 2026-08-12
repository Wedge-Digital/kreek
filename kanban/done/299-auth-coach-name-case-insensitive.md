# Recherche de coach insensible à la casse

**Priorité : haute**
**Contexte :** `auth` — connexion, inscription, mot de passe oublié

## Objectif

Un coach qui ne se souvient plus s'il s'est inscrit sous « Bagouze » ou
« bagouze » ne pouvait ni se connecter, ni demander la réinitialisation de son
mot de passe : `find_user_by_coach_name.sql` comparait en `=`, donc en
respectant la casse. Rien dans l'interface ne lui indiquait laquelle essayer.

Le nom de coach doit identifier un compte **sans distinction de casse**, sur
les trois parcours qui le prennent en saisie.

---

## Décisions

1. **Un index unique fonctionnel plutôt qu'une simple recherche `lower()`.**
   `users_coach_name_uq` est remplacée par
   `CREATE UNIQUE INDEX users_coach_name_lower_uq ON auth__users (lower(coach_name))`.
   Sans cette unicité, deux comptes ne différant que par la casse rendraient la
   recherche ambiguë — le correctif appelle donc le durcissement, il n'en est
   pas séparable. L'index subsume l'ancienne contrainte et sert la comparaison.

2. **L'inscription refuse un nom déjà pris dans une autre casse.** Corollaire
   assumé du point précédent, validé par l'utilisateur : le projet n'est pas
   encore en production, et les 852 comptes du corpus legacy importé ne
   contiennent aucune collision de casse.

3. **La casse saisie à l'inscription est conservée à l'affichage.** Seule la
   comparaison est insensible, pas le stockage.

4. **L'expéditeur d'emails devient configurable** (`EMAIL__PROVIDER`,
   `resend` par défaut). Le test e2e du mot de passe oublié appellerait sinon
   l'API Resend à chaque exécution, en local comme en CI — où une
   `EMAIL__API_KEY=dummy-key-ci` partait pour de bon.

---

## Pièges rencontrés

- **Trois consommateurs s'appuyaient sur la contrainte supprimée.** Les deux
  seeds (`ON CONFLICT (coach_name)`, que Postgres ne sait plus inférer une fois
  la contrainte partie) et le contrôle anti-doublon de `scripts/import_users.py`,
  dont la comparaison exacte laissait passer une collision de casse que
  l'`INSERT` rejetait ensuite : l'utilisateur était perdu au lieu d'être renommé
  avec le suffixe `_N` prévu.
- **`update_user_password.sql` n'était pas en cause** — son nom vient du token
  de réinitialisation, donc de la base. Aligné quand même : une comparaison
  exacte n'échouerait pas bruyamment le jour où ce nom viendrait d'une saisie,
  elle mettrait à jour zéro ligne en silence.
- **La demande de réinitialisation ne dit jamais si le compte existe** (même
  page de confirmation dans les deux cas, pour ne pas révéler qui est inscrit).
  Le token créé en base est donc le seul témoin observable en e2e.

---

## Checklist

- [x] Migration : index unique fonctionnel sur `lower(coach_name)`
- [x] `find_user_by_coach_name.sql` — comparaison insensible à la casse
- [x] `update_user_password.sql` — alignement
- [x] `seed_accounts.rs` et `seed_e2e.rs` — `ON CONFLICT (lower(coach_name))`
- [x] `scripts/import_users.py` — dédoublonnage insensible à la casse
- [x] `EMAIL__PROVIDER` (`console` | `resend`), clé d'API vérifiée au démarrage
- [x] `ConsoleEmailService` sorti de `fakes/` — ce n'est plus un fake de test
- [x] `make dev-demo` et job e2e de la CI en `console`
- [x] Tests d'intégration repository (vraie base) : recherche, unicité, update
- [x] Tests unitaires : sélection de l'expéditeur, refus de démarrer sans clé
- [x] Test e2e `test_auth_coach_name_casse.py` — 4 scénarios, contrôle négatif
      compris
- [x] Carte d'impact tests↔bounded-contexts mise à jour
