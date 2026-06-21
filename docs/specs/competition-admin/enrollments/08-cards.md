# Inscriptions — Phase 8 : Cartes kanban ✅

## Cartes produites

| # | Carte | Dépend de | Statut |
|---|---|---|---|
| 86 | Domaine : reject_enrollment + tests unitaires | — | Done |
| 87 | Use cases approve/reject enrollment | 86 | Done |
| 88 | Projection enrollment + find_by_season_and_status | — | Done |
| 89 | Widgets teams (pending + enrolled) + actions handlers | 87, 88 | Done |
| 90 | Fragment onglet inscriptions + câblage page hôte | 89 | Done |
| 91 | Listeners app events (activité récente) | 87 | Reportée |
| 92 | Tests E2E inscriptions | 90 | Done |

## Note

La carte 91 (listeners app events) est reportée. Le mécanisme de publication d'events du BC teams n'existe pas encore. Non bloquant pour l'onglet inscriptions.
