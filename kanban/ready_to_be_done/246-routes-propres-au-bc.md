# Routes propres au BC — sortir `AppRoutes` d'auth et spaces

**Priorité : moyenne**
**Dépend de :** 242 (chapeau) — indépendante de 243/244/245
**Fichiers :** 5 controllers spaces, 5 templates spaces,
`src/app/spaces/io/web/register_space.rs`, `CLAUDE.md`

## Problème

`src/app/routes.rs` agrège les onze modules de routes de l'application. Cinq
controllers de spaces l'utilisent comme type de champ dans leurs structs Askama :

- `io/web/all_spaces.rs:20`
- `io/web/register_space.rs:22,41`
- `io/web/widget_tester_controller.rs:15`
- `io/web/controllers/widgets/coach_search.rs:23`
- `io/web/controllers/widgets/coach_search_results.rs:26`

Résultat : sortir spaces embarque au niveau du type les routes de `teams`,
`match_report`, `players`, `competitions`, `ranking`, `references`,
`team_creation`, `news`.

## La bonne nouvelle

**Aucun de ces templates ne référence la route d'un autre BC.** L'inventaire
exhaustif des appels de route dans les templates d'auth et de spaces :

```
6 × app_routes.spaces.register_space
1 × app_routes.spaces.join
1 × routes.spaces.coach_search_results
1 × routes.spaces.coach_search_widget
1 × routes.spaces.coach_select_widget
```

Tout pointe vers le BC lui-même. `AppRoutes` n'y sert à rien d'autre qu'à être
le type imposé par la convention. Le remplacement est donc mécanique : champ
`AppRoutes` → champ `SpacesRoutes`, et `app_routes.spaces.x()` → `routes.x()`
dans les templates.

## Le seul lien sortant réel

`src/app/spaces/io/web/register_space.rs:104` redirige vers
`auth_path::AUTH_LAYOUT` après création d'un espace. C'est un vrai lien
inter-BC, et le seul.

Le traiter par injection plutôt que par import : le contexte (ou la config du
BC) reçoit du host la destination de redirection post-création — une `String`
ou un petit type dédié. Le BC ne connaît plus `auth::routes`.

## Divergence avec CLAUDE.md — à formuler explicitement

La section « Accès aux routes — règle obligatoire » impose aujourd'hui :

> Les routes des autres BCs sont **toujours** accédées via `AppRoutes`, jamais
> par un import direct du module de routes d'un autre BC.

Cette règle reste juste pour les BCs non extraits — elle empêche justement le
type d'import direct qu'on trouve dans `register_space.rs`. Mais elle est
incompatible avec l'objectif d'extraction pour auth et spaces.

Ajouter l'exception dans CLAUDE.md : **un BC destiné à l'extraction n'utilise
que ses propres `Routes` ; ses liens sortants sont injectés par le host.** Ne
pas se contenter de le faire dans le code — la prochaine session recâblerait
`AppRoutes` en toute bonne foi.

## Checklist

- [ ] Les 5 controllers de spaces utilisent `SpacesRoutes` et non `AppRoutes`
- [ ] Les templates appellent `routes.x()` au lieu de `app_routes.spaces.x()`
- [ ] Redirection post-création d'espace injectée depuis le host,
      `use crate::app::auth::routes` supprimé de `register_space.rs`
- [ ] `grep -rn "app::routes::AppRoutes" src/app/auth src/app/spaces` ne remonte rien
- [ ] Exception ajoutée à la section « Accès aux routes » de CLAUDE.md
- [ ] `make check-arch` au vert — vérifier en particulier l'axe 4 (aucune route
      en dur n'a été introduite en remplacement)
- [ ] `make test` au vert
