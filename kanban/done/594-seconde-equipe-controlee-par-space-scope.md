# La seconde équipe d'une URL est contrôlée par `space_scope`

**Priorité : haute** — préalable de sécurité de toute route d'impression à deux équipes
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** rien
**Spec :** `docs/specs/impression-feuille-equipe/team-print/03-back.md` (décision 1), `07-integration.md` (« Sécurité »)
**Fichiers :** `src/infrastructure/teams/space_ownership.rs`, `src/main.rs`

## Objectif

`space_scope` ne contrôle que les paramètres de chemin qu'il connaît
(`team_id`, `player_id`, `season_id`…). L'impression reçoit une seconde
équipe : passée en query, ou sous un nom inconnu, elle ne serait vérifiée par
personne — c'est la forme exacte de la fuite de `team-selection/json`.

Toute route d'impression à deux équipes déclare donc `{other_team_id}` dans son
chemin, et ce paramètre a son résolveur.

## Conception

- Un `impl ISpaceOwnership` dont `param()` rend `"other_team_id"`, et dont
  `space_of` résout comme celui de `team_id` (`team_proj`, puis brouillons) :
  réutiliser la résolution existante plutôt que la recopier.
- Enregistré dans le `vec!` de `main.rs` ; `verifier_unicite_des_parametres`
  garantit qu'il n'existe qu'une fois.
- Aucune route ne l'utilise encore : la carte livre le verrou avant les portes.

## Checklist

- [x] Résolveur `other_team_id`, sans duplication de la requête de `team_id`
- [x] Enregistrement dans `main.rs`
- [x] Test unitaire : une équipe de l'espace passe, une équipe d'un autre espace rend `None` ou un autre espace, un identifiant inconnu rend `None`
- [x] `make check-arch`, `make lint`, `make test`
