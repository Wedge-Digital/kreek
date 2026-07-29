# Verrou `check-arch` — empêcher le recouplage d'auth et spaces

**Priorité : haute** (sans elle, toute la série se défait en quelques sessions)
**Dépend de :** 243, 244, 245, 246, 247 — c'est la dernière carte de la série
**Fichiers :** `scripts/check-arch.sh`, `CLAUDE.md`

## Problème

La carte 242 a écarté le découpage en crates cargo, qui aurait fait du
découplage une propriété vérifiée par le compilateur : un `use crate::state::`
dans un crate `auth` séparé est une erreur de compilation, point final.

Sans ce verrou, rien ne signale une régression. Le précédent est documenté dans
la carte 237 : `recap_controller.rs` a atteint `state.spaces` directement
**pendant des mois** sans que rien ne le relève, parce que `match_report`
n'était pas dans la liste des BCs de l'axe 3. La violation a été trouvée à la
main. Une session de bonne foi qui ajoute `State<AppState>` dans un handler
d'auth annule le travail de la carte 245, et personne ne le voit.

## Action

Ajouter un **axe 8 bloquant** à `scripts/check-arch.sh` : « BCs extractibles ».

Une liste explicite en tête de script (ici, ces deux-là seulement — c'est un
statut qu'on accorde, pas une propriété qu'on découvre) :

```sh
# BCs maintenus extractibles vers un autre projet (cf. kanban/242).
# Contraintes plus strictes que les autres BCs : aucune adhérence au host.
EXTRACTABLE_BCS="auth spaces"
```

Pour chaque fichier de production de ces BCs (réutiliser `strip_test_code`,
déjà en place), interdire :

| Motif | Ce qu'il empêche | Carte |
|---|---|---|
| `crate::state::` / `State<AppState>` / `state\.` | le retour de l'objet dieu | 245 |
| `crate::app::routes::AppRoutes` | l'agrégat de routes de l'app | 246 |
| `crate::web::` | layout, extracteurs et middlewares du host | 247 |
| `{% extends "app-layout.html" %}` et les autres layouts de `src/web/templates/` (sur les `.html` du BC) | le chrome de kreek | 247 |
| `use crate::app::<autre_bc>::` sans exception `::routes::` | les liens inter-BC | 246 |

Deux points d'attention :

- L'exemption générale `auth_backend::AuthSession` de l'axe 3 **ne doit pas**
  s'appliquer à ce nouvel axe dans le sens spaces → auth : elle existe pour que
  les BCs de kreek consomment la session, pas pour autoriser des allers-retours
  entre les deux BCs extraits. Les deux partant en couple (décision 242),
  `spaces` a le droit de consommer `AuthSession` et `AuthAppEvent` — mais rien
  d'autre, et surtout pas `auth::routes`.
- L'axe doit être **bloquant**, pas en avertissement. Un avertissement dans une
  sortie de script est un axe mort.

Documenter l'axe dans l'en-tête du script (la liste des axes y est maintenue)
et ajouter une ligne dans CLAUDE.md sur le statut « BC extractible » et ce
qu'il implique.

## Limite connue

`check-arch.sh` est un ensemble de `grep`, pas un analyseur syntaxique : il ne
distingue ni les commentaires ni les chaînes littérales (cf. le faux positif
traité dans la carte 237). Ce verrou est plus faible que le compilateur — c'est
le prix de la décision de la carte 242, à assumer tel quel.

Il ne voit pas non plus le SQL : le `LEFT JOIN auth__users` de
`spaces/io/repository/sql/space/find_space_by_id.sql` reste invisible, comme
noté dans le périmètre exclu de la carte 242.

## Checklist

- [ ] Axe 8 ajouté, bloquant, avec `EXTRACTABLE_BCS="auth spaces"` en tête de script
- [ ] En-tête du script mis à jour (liste des axes)
- [ ] L'axe est vérifié **en échec** avant d'être vérifié au vert : introduire
      volontairement un `State<AppState>` dans un handler d'auth, confirmer que
      le script échoue, puis annuler
- [ ] `make check-arch` au vert sur l'ensemble du projet
- [ ] CLAUDE.md : statut « BC extractible » documenté
- [ ] Carte 242 relue : critère de sortie atteint, série close
