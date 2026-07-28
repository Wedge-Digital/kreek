# Casser le cycle `shared_kernel` → `auth`

**Priorité : haute**
**Dépend de :** 242 (chapeau)
**Fichiers :** `src/app/shared_kernel/coach_name.rs`, `src/app/shared_kernel/email.rs`,
`src/app/shared_kernel/user.rs`, `src/app/auth/domain/error.rs`,
`src/app/all_domain_events.rs`, `src/app/auth/app_event.rs`, `scripts/check-arch.sh`

## Problème

`shared_kernel` est censé être le socle dont tout le monde dépend. Il dépend
d'un BC :

```rust
// src/app/shared_kernel/coach_name.rs:1
use crate::app::auth::domain::error::AuthDomainError;
// src/app/shared_kernel/email.rs:1
use crate::app::auth::domain::error::AuthDomainError;
```

Et `auth` dépend massivement de `shared_kernel` (14 imports de `coach_name`,
6 d'`email`, 8 de `user`). **Le cycle rend toute séparation impossible** : il
n'existe aucun ordre dans lequel copier les deux dossiers.

Second problème dans le même dossier : `shared_kernel/user.rs` est en réalité
l'agrégat d'`auth` mal placé, et il implémente `axum_login::AuthUser` — un
type présenté comme « noyau partagé » est donc couplé à un framework web.

## Action

### 1. Rendre les value objects du kernel autonomes

`CoachName` et `Email` ne doivent pas connaître l'erreur d'un BC. Deux options,
à trancher à l'implémentation :

- une erreur propre au kernel (`KernelError` ou l'erreur générée par `nutype`),
  `auth` convertissant vers `AuthDomainError` via `From` — préféré ;
- ou faire remonter l'erreur `nutype` telle quelle, si aucun appelant n'a
  besoin d'un type d'erreur métier.

### 2. Déplacer `User` dans `auth`

`shared_kernel/user.rs` → `auth/domain/user.rs`, **avec** son
`impl axum_login::AuthUser`. Le type et son implémentation de trait framework
vivent au même endroit, dans le BC qui en est propriétaire.

Quatre BCs consomment ce type aujourd'hui, via la session :

- `src/app/competitions/io/web/calendrier_tab_controller.rs`
- `src/app/competitions/io/web/resultats_view.rs`
- `src/app/match_report/io/web/recap_controller.rs`
- `src/app/players/io/web/purchase_skill_controller.rs`
- `src/app/players/io/web/widgets/spp_spending_widget.rs`

**Ce déplacement ne crée aucun couplage nouveau** : ces fichiers importent déjà
`auth::auth_backend::AuthSession`, dont `user` est le champ. Ils passent
simplement d'un import `shared_kernel` à un import `auth` sur la même donnée.

### 3. Étendre l'exemption de l'axe 3

`scripts/check-arch.sh` exempte déjà la surface publique d'auth :

```sh
grep -vE "::routes::|auth_backend::AuthSession"
```

Ajouter le type `User` à cette exemption. L'exemption est légitime : dans une
extraction, tout le monde dépend du paquet d'authentification — c'est la
définition d'un fournisseur d'identité.

### 4. Supprimer deux fichiers morts

`src/app/all_domain_events.rs` et `src/app/auth/app_event.rs` référencent
`crate::lib::services::event_bus`, un chemin qui n'existe plus. Ils ne sont
déclarés dans aucun `mod.rs` — donc jamais compilés, jamais vus par personne.
Copiés tels quels dans un autre projet, ils cassent le build dès qu'on les
déclare.

Vérifier avant suppression (règle CLAUDE.md n°4) : `AllDomainEvents`,
`AllDomainEventKind` et `AuthAppEvent` (celui de `auth/app_event.rs`, à ne pas
confondre avec `shared_kernel/app_events/auth_app_events.rs` qui est bien vivant)
n'ont aucun consommateur.

## Checklist

- [ ] `shared_kernel/coach_name.rs` et `email.rs` n'importent plus rien de `app::auth`
- [ ] `grep -rn "app::auth" src/app/shared_kernel/` ne remonte rien
- [ ] `User` déplacé dans `auth/domain/user.rs` avec son `impl AuthUser`
- [ ] Les 5 fichiers consommateurs mis à jour
- [ ] Exemption `User` ajoutée à l'axe 3 de `check-arch.sh`, avec un commentaire
      expliquant pourquoi auth est un cas particulier
- [ ] `all_domain_events.rs` et `auth/app_event.rs` supprimés après vérification
      qu'ils n'ont aucun consommateur
- [ ] `make check-arch` au vert, `make test` au vert
