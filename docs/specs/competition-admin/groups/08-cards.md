# Poules — Phase 8 : Cartes kanban ✅

## Condition d'affichage

L'onglet Poules n'apparaît que si la compétition comporte plusieurs poules (`use_ranking_groups == true && ranking_groups.len() > 1` dans `CompetitionStructure`). Le flag `has_groups` est passé au template `admin-page.html` pour conditionner l'affichage du tab.

## Cartes produites

| # | Carte | Dépend de |
|---|---|---|
| 93 | Migration tables groups + repository IGroupRepository | — |
| 94 | Port ITeamInfoPort + adapter | 93 |
| 95 | Use cases (random_draw, reset_groups, assign_team) + tests unitaires | 93, 94 |
| 96 | Widgets poules (unassigned pool + group cards) | 94, 95 |
| 97 | Fragment onglet poules + câblage page hôte + actions + condition affichage tab | 96 |
| 98 | Tests E2E poules | 97 |

## Ordre d'implémentation

```
93 → 94 → 95 → 96 → 97 → 98
```
