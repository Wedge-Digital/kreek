# Phase 8 — Cartes kanban — team-detail

6 cartes, ordonnées par dépendance (domaine → persistance → use case →
front → endpoint → e2e). Toutes dans `kanban/ready_to_be_done/`.

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 290 | `players-roster-edit-domain` | — | VOs (`PersonalName`, `DisplayOrder`), `JerseyVo` resserré, champs `Player`, événements, méthodes `rename`/`change_jersey`/`reorder`, `DomainError::PlayerNotActive` |
| 291 | `players-roster-edit-persistence` | 290 | Migration `display_order`, `append_batch` (port + override transactionnel Pg), branches projection, tri `find_by_team_id` |
| 292 | `players-roster-edit-use-case` | 290, 291 | `update_roster_use_case` : appartenance, unicité sur l'effectif actif complet, diff par champ, persistance atomique |
| 293 | `players-roster-edit-widget` | 290 | Widget GET étendu (mode édition, repris de la maquette validée), renommage vers `widgets/`, route déclarée |
| 294 | `players-roster-edit-save-endpoint` | 292, 293 | Endpoint POST, autorisation réutilisée, réponses succès/échec |
| 295 | `players-roster-edit-e2e` | 293, 294 | 8 scénarios Playwright |

Chaque carte est compilable/testable/commitable indépendamment dans cet
ordre — 293 (front) n'a qu'une dépendance légère sur 290 (aucune sur 291/292)
et peut donc avancer en parallèle du couple 291/292 si besoin.
