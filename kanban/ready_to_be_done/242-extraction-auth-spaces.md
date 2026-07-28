# Extraction d'Auth et Spaces — carte chapeau

**Priorité : moyenne**
**Dépend de :** —
**Fichiers :** aucun (carte de pilotage)

## Objectif

Rendre les BCs `auth` et `spaces` **réutilisables dans un autre projet** : à
l'issue de la série, copier `src/app/auth/`, `src/app/spaces/` et le noyau
d'identité dans un nouveau projet doit suffire, sans avoir à démêler des
dépendances vers le reste de kreek.

Cette carte ne contient pas de code à écrire. Elle porte l'objectif, la
séquence, les arbitrages et surtout le **périmètre exclu**.

## Critère de sortie

Aucun fichier de `src/app/auth/` ni de `src/app/spaces/` ne référence :

- `crate::state::AppState`
- `crate::app::routes::AppRoutes`
- `crate::web::` (layout, extracteurs, middlewares)
- un autre BC que lui-même
- un type de `shared_kernel` propre au métier Blood Bowl

Et réciproquement, `shared_kernel` ne référence plus `auth`.

Le verrou est posé par la carte 248 (nouvel axe dans `check-arch.sh`).

## Ce qui est déjà propre — à ne pas casser

L'audit préalable a montré que le métier n'est pas le problème :

- `auth/domain/` et `spaces/domain/` n'importent aucun framework (ni `axum`,
  ni `sqlx`, ni `askama`) — la règle de pureté est respectée
- les ports et les repositories suivent le pattern du projet, avec des fakes
  utilisables en test unitaire
- les tables sont préfixées (`auth__users`, `auth__lost_login_token`,
  `spaces__user_space`, `spaces__user_cache`)
- **aucune clé étrangère inter-BC en base** — le schéma est déjà séparable
- le couple domain event → app event passe bien par un publisher
- `auth` embarque son propre layout (`auth-layout.html`) : il ne dépend pas du
  chrome de kreek

Ce qui bloque, c'est le câblage applicatif : l'état, les routes, le layout et
un cycle dans le kernel.

## Séquence

L'ordre compte : 243 est un prérequis strict de 244, et 248 ne peut être posé
qu'en dernier.

| Carte | Chantier | Bloque |
|---|---|---|
| 243 | Casser le cycle `shared_kernel` → `auth` | 244 |
| 244 | Scinder `shared_kernel` : identité / Blood Bowl | — |
| 245 | Sous-états `FromRef` — contextes autoportants | — |
| 246 | Routes propres au BC (sortir `AppRoutes`) | — |
| 247 | Chrome web — layout et rapatriement des morceaux égarés | — |
| 248 | Verrou `check-arch` | toutes |

Les cartes 245, 246 et 247 sont indépendantes entre elles et peuvent être
prises dans n'importe quel ordre une fois 243 et 244 faites.

## Dépendance externe

Le dépôt `rust-htmx-scaffold` (générateur de projets Rust/Axum/HTMX dérivé de
cette base) attend les cartes **243 à 247** pour ses paliers `--with-auth` et
`--with-spaces`. Copier `auth` ou `spaces` avant, c'est dupliquer dans chaque
projet généré la dette que cette série supprime.

Cette information n'appelle aucune action ici : le pilotage de ce dépôt vit
dans son propre kanban.

## Périmètre exclu — décisions du 2026-07-28

**Pas de workspace cargo.** L'extraction en crates séparés (`kernel` / `auth` /
`spaces` / `kreek-app`) serait la seule preuve vérifiable par le compilateur :
un import croisé deviendrait une erreur de compilation. Écarté comme trop
intrusif pour le bénéfice attendu — touche `Cargo.toml`, les chemins de
templates Askama (`askama.toml` liste onze dossiers) et le build. La
compensation est la carte 248 : un axe `check-arch` qui joue le rôle du
compilateur, en moins fiable.

**Auth et Spaces s'extraient en couple.** Rendre `spaces` utilisable *sans*
`auth` supposait de supprimer le `LEFT JOIN auth__users` de
`src/app/spaces/io/repository/sql/space/find_space_by_id.sql` (le cache
`spaces__user_cache`, alimenté par `user_created_listener`, contient déjà la
donnée). Écarté : les deux BCs partent ensemble. **Conséquence assumée** :
cette violation de souveraineté reste dans kreek, et `check-arch` ne la voit
pas — c'est du SQL, pas un import Rust.

**Adhérences entrantes hors périmètre.** Trois SQL de `news` joignent
`auth__users`, un SQL de `competitions` joint `spaces__user_cache`, et
`src/cli/seed_e2e.rs` écrit directement dans les deux. Ce sont des violations
de souveraineté du projet actuel, pas des BCs extraits : les paquets emportent
leur schéma, donc kreek continue de fonctionner. À traiter dans une autre
série si le sujet remonte.

## Verrues assumées, non traitées

Elles sont documentées ici pour que le projet qui réutilise ces BCs sache ce
qu'il achète — aucune n'empêche l'extraction :

- **`legacy_id`** — colonne et méthode `find_by_legacy_id()`, reliquat de la
  reprise de données depuis l'ancienne appli kreek. Inutile ailleurs, mais la
  supprimer touche la migration : hors sujet ici.
- **`coach_name` comme identifiant de connexion**, `CoachId`, `CoachIcon` —
  vocabulaire Blood Bowl. Un autre projet renommera ou assumera « coach »
  comme synonyme d'utilisateur.
- **`CloudinaryImage`** valide une URL `res.cloudinary.com` par expression
  régulière : le fournisseur de stockage d'images est imposé au noyau.
- **Messages en français en dur** dans `RepositoryError`, les handlers et les
  templates — aucune infrastructure d'i18n.
- **`axum-login` + `tower-sessions` + `argon2`** sont non négociables :
  `AuthnBackend` est l'API publique du BC.
- **La table `spaces`** n'est pas préfixée `spaces__`, contrairement à toutes
  les autres tables du BC.

## Checklist

- [ ] Cartes 243 à 248 écrites et prises dans l'ordre de la séquence
- [ ] Critère de sortie vérifié à la main avant de poser le verrou de la 248
- [ ] `make check-arch` au vert sur l'ensemble du projet
- [ ] `make test` au vert
- [ ] CLAUDE.md mis à jour là où la règle générale et le découplage divergent
      (cf. carte 246 sur `AppRoutes`)
