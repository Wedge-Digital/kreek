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

- [x] `CreateAccountWithoutPasswordCommand` sans primitive nue à protéger — il
      n'y a pas de mot de passe, donc pas de `Secret<T>`
- [x] Colonne `password_hash` : **sentinelle `"!"`**, la convention de Django
      dont vient l'import legacy. `PasswordHash::new` la refuse au parsing, donc
      aucune session ne s'ouvre — exactement l'état des 851 comptes importés. La
      rendre nullable toucherait le chemin de connexion, bien au-delà de cette
      carte. Un test verrouille l'inutilisabilité
- [x] Vérification d'unicité du pseudo **avant l'envoi**
- [ ] ~~Vérification d'unicité de l'adresse~~ — **impossible** : le dépôt n'offre
      pas de recherche par e-mail. Elle n'est connue qu'à l'insertion, donc une
      adresse déjà prise reçoit un lien qui ne mènera nulle part. Écrit dans le
      code, et le jeton est nettoyé
- [x] Jeton engendré, e-mail de définition envoyé
- [x] `AuthDomainEvent::AccountCreated` émis par `emettre()`
- [x] `#[tracing::instrument(skip_all, fields(cmd = ?cmd))]`
- [x] Six tests unitaires, dont **l'envoi qui échoue ne laisse ni compte ni
      jeton** — vu échouer sur l'ordre inverse

## Ce qu'on a appris en la faisant

**La table de jetons n'a aucune clé étrangère** vers les comptes : elle stocke un
pseudo en clair. Le jeton peut donc être créé **avant** l'utilisateur, ce qui
rend possible l'ordre qui satisfait la carte — jeton, envoi, puis création.

Sans cette vérification préalable, il aurait fallu créer le compte d'abord et
chercher à le défaire. Or `IUserRepository` **n'a pas de méthode de
suppression**, et en ajouter une pour un chemin de compensation aurait été plus
dangereux que le problème qu'elle résout.

**Le pseudo est vérifié avant l'envoi**, et ce n'est pas une optimisation : sans
ça, une faute de frappe sur un pseudo existant enverrait un lien de définition de
mot de passe à son titulaire actuel, qui n'a rien demandé.

**Le jeton orphelin est effacé sur chaque chemin d'échec.** Sans ça, un jeton
subsisterait pour un pseudo inexistant — et si quelqu'un enregistrait ce pseudo
plus tard, le lien déjà envoyé lui donnerait la main sur le compte.
