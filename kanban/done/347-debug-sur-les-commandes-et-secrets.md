# `app` — `Debug` sur les commandes, et trois secrets à masquer

**Priorité : haute** — la carte 348 en dépend, et la revue de secrets vaut par
elle-même
**Dépend de :** rien
**Fichiers :** les 62 structures `…Command` de `src/app/*/use_cases/`, dont
trois dans `auth` · `src/app/shared_kernel/identity/secret.rs`

## Le problème

**Aucune des commandes ne dérive `Debug`.** Zéro — elles n'ont aucune
dérivation du tout, à une exception près (`PerformLoginCommand`, qui dérive
`Deserialize` parce qu'elle est construite par `Form<…>` depuis le formulaire
de connexion). Or la carte 348 journalise la commande reçue par chaque use
case, ce qui l'exige.

Ajouter `#[derive(Debug)]` partout est mécanique. Le faire sans regarder ce
qu'on rend imprimable ne l'est pas : **trois commandes portent des secrets en
clair.**

| Commande | Champs |
|---|---|
| `PerformLoginCommand` (`auth/use_cases/perform_login.rs`) | `password` |
| `RegisterCommand` (`auth/use_cases/register_new_acount.rs`) | `email`, `password`, `password_confirm` |
| `ResetPasswordCommand` (`auth/use_cases/reset_password.rs`) | `token`, `password`, `password_confirm` |

Un `#[derive(Debug)]` posé sans réfléchir sur ces trois-là mettrait **les mots
de passe des coachs dans `docker logs`**. Le `token` n'est pas moins grave : il
autorise la réinitialisation d'un mot de passe, c'est un identifiant de
connexion à durée de vie limitée. L'adresse e-mail, elle, est une donnée
personnelle qui n'a rien à faire dans un journal de diagnostic.

## Elles sont 62, pas 56

Le compte initial de la carte venait d'un glob sur `src/app/*/use_cases/`. Il
en manquait six, pour une raison qui vaut d'être écrite : **`spaces` avait son
dossier orthographié `uses_cases`**, avec un `s` de trop. Aucun outil ne s'en
plaignait, et tout script qui vise `use_cases/` le ratait en silence —
`check-arch` compris.

Le dossier est renommé dans cette carte plutôt que dans une autre : c'est lui
qui a causé l'angle mort, et le laisser aurait garanti que le prochain
inventaire soit faux de la même façon.

## Ce qu'il faut faire

**Les 59 autres** : `#[derive(Debug)]`, sans autre forme de procès. Trois types
de champs ont dû le dériver au passage — `InducementPurchaseCmd`,
`MercenaryPurchaseCmd`, `TeamMatchStats` — le compilateur les a nommés.

**Les trois de `auth`** : leurs champs sensibles passent par un newtype
`Secret<T>` (`shared_kernel/identity/secret.rs`) dont le `Debug` rend
`[masqué]`. Les structs dérivent alors `Debug` comme les autres.

### Pourquoi un type et pas trois `impl Debug` écrits à la main

C'était la solution d'origine de la carte, et elle a été écartée en cours de
route. Un `Debug` manuel protège les champs **qu'on a pensé à masquer, le jour
où on l'a écrit** : ajouter un champ un an plus tard ne casse rien et ne
prévient personne — le nouveau champ est simplement absent du rendu, ou pire,
ajouté au `debug_struct` par réflexe.

Le type masque par construction. Un secret ne fuit que si quelqu'un écrit
`expose()`, ce qui se voit à la relecture et s'énumère en un `grep`.

`Secret` n'implémente délibérément ni `Display`, ni `Deref`, ni `AsRef` :
chacun rendrait un secret interpolable dans un `{}`. Ni `Serialize` : un secret
sérialisable finirait tôt ou tard dans une charge d'événement, donc dans
l'event store, d'où on ne l'efface plus.

Écrire ça plutôt qu'exempter ces commandes du journal, c'est le point important
de la carte : **le risque existe déjà et ne vient pas du journal.**
Aujourd'hui, n'importe quel `{:?}` égaré — dans un message d'erreur, un `dbg!`
de débogage oublié, une variante d'erreur qui embarque la commande — produit la
même fuite. Corriger la représentation la supprime partout à la fois, y compris
aux endroits qu'on n'a pas prévus.

## Ce qui a été vérifié au passage

Le tableau ci-dessus venait d'une recherche sur les noms de champs. Les 62
commandes ont été relues **par leurs 105 champs distincts**, type par type :
`email` n'apparaît qu'une fois, `token` une fois, `password` trois fois. Le
reste est fait d'identifiants, de noms d'affichage, de numéros de version et
d'un drapeau — rien de confidentiel. `SendResetPasswordEmailCommand` ressort
d'une recherche naïve à cause de `host_domain`, mais elle ne porte qu'un nom de
coach et un nom de domaine.

## Ce qui reste hors périmètre, et pourquoi

**Les payloads de formulaire** (`RegisterFormPayload`, `UpdatePasswordPayload`)
gardent leurs `String`. Ils ne dérivent pas `Debug`, donc ne fuient pas, et la
frontière du secret est la commande — c'est elle que la 348 journalise. Les
envelopper ajouterait des `expose()` jusque dans le re-rendu du formulaire.

**Les primitives nues des 59 autres commandes** (`space_id: String`,
`expected_version: u32`, …) violent la règle CQRS de `CLAUDE.md`. C'est un
chantier réel, sans rapport avec les secrets, et le mêler à celui-ci rendrait
les deux illisibles.

## Checklist

- [x] Les 62 commandes dérivent `Debug` — 62 sur 62 vérifiées
- [x] `spaces/uses_cases/` renommé en `use_cases/`, imports repris
- [x] `Secret<T>` dans `shared_kernel/identity/`, sans `Display`, `Deref`,
      `AsRef` ni `Serialize`
- [x] `PerformLoginCommand`, `RegisterCommand`, `ResetPasswordCommand` portent
      leurs secrets dans `Secret<String>`
- [x] Test unitaire : le rendu `{:?}` de ces trois commandes ne contient ni le
      mot de passe, ni le jeton, ni l'e-mail — le seul garde-fou contre un
      champ repassé en `String` par distraction
- [x] Test unitaire : une struct qui dérive `Debug` masque son `Secret` sans
      rien écrire, et le champ de diagnostic reste lisible
- [x] Les 62 commandes ont été relues une par une, par leurs 105 champs
      distincts, pas seulement filtrées par nom
- [x] `make lint`, `make test` et `make check-arch` passent
