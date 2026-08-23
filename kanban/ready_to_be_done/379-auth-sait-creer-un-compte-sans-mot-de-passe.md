# `auth` sait créer un compte sans mot de passe

**Priorité : haute** — indépendante du reste de la fonctionnalité
**Dépend de :** rien
**Conception :** `docs/specs/space-admin/ajout-direct/05-use-cases.md`
**Fichiers :** `src/app/auth/use_cases/create_account_without_password.rs`

## Pourquoi un use case de plus

`RegisterCommand` exige `password` et `password_confirm`, et refuse en dessous
de huit caractères. Un compte créé par un administrateur d'espace n'en a pas.

Lui en inventer un serait pire : un mot de passe que personne ne connaît et que
rien n'oblige à changer.

## Orchestration

1. mêmes vérifications d'unicité et de format que l'inscription publique —
   pseudo et email
2. créer le compte **sans hachage de mot de passe**
3. engendrer un jeton de réinitialisation et envoyer l'email de définition
4. `emettre(bus, AuthDomainEvent::AccountCreated { … })`

**Le même événement que l'inscription publique.** C'est le même fait : un compte
existe. Le chemin par lequel il a été créé n'intéresse aucun BC d'à côté, et
`spaces::user_created_listener` continue d'alimenter son cache sans rien savoir.

## L'email est une étape, pas une option

Décidé en phase 2 : la case de la maquette est retirée, l'email part toujours.
Son échec **fait donc échouer le use case** — un compte sans mot de passe et sans
email reçu est un compte auquel personne ne peut accéder.

C'est l'inverse du choix de la carte 378, où l'email n'est qu'une courtoisie, et
les deux sont cohérents : là-bas un agrément, ici l'unique porte d'entrée.

## Le point délicat

**Si l'envoi échoue, le compte ne doit pas rester derrière.** Soit l'envoi
précède le point de non-retour, soit l'échec le défait.

Un compte créé dont l'unique accès n'a pas été livré, c'est un pseudo et un
email pris, et une seconde tentative qui échouera sur `PseudoDejaPris` sans que
l'administrateur comprenne pourquoi.

## Checklist

- [ ] `CreateAccountWithoutPasswordCommand` sans primitive nue, aucun champ
      sensible à protéger par `Secret<T>` puisqu'il n'y a pas de mot de passe
- [ ] Colonne `password_hash` : décider ce qu'on y met — `auth__users` la
      déclare `NOT NULL`. Chaîne vide, sentinelle, ou migration vers nullable :
      à trancher et à commenter, pas à subir
- [ ] Vérifications d'unicité pseudo et email, mêmes messages que l'inscription
- [ ] Jeton de réinitialisation engendré, email de définition envoyé
- [ ] `AuthDomainEvent::AccountCreated` émis par `emettre()`
- [ ] `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]`
- [ ] Tests unitaires :
  - [ ] création nominale → `Ok(coach_id)`, événement émis, email envoyé
  - [ ] pseudo déjà pris → `PseudoDejaPris`, aucun compte créé
  - [ ] email déjà pris → `EmailDejaPris`, aucun compte créé
  - [ ] **l'envoi échoue → erreur, et le compte n'est pas laissé derrière**
- [ ] `make lint`, `make check-arch`, `make test` passent
