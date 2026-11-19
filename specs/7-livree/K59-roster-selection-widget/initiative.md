---
id: K59
title: "Widget de sélection de roster avec tiers — et la série qui en dépend"
---

# K59 — Widget de sélection de roster avec tiers — et la série qui en dépend

Initiative créée à la migration vers le kanban de la spec factory (2026-11-19), à partir de une série de cartes livrées sans épic, reliées entre elles par leurs lignes « Dépend de ». Ce regroupement est déduit des dépendances, pas nommé dans le dépôt.

## Tickets

- `19-roster-selection-widget` — Widget de sélection de roster avec tiers (livrée à la migration)
- `20-build-team-roster-wiring` — Câblage widget roster → table joueurs dans build-team (livrée à la migration)
- `21-hire-player` — Engager un joueur (bouton + de la table) (livrée à la migration)
- `22-fire-player` — Renvoyer un joueur (bouton − de la table) (livrée à la migration)
- `23-staff-table-roster-update` — Mise à jour du tableau staff en fonction du roster sélectionné (livrée à la migration)
- `24-buy-remove-staff` — Achat et retrait de staff (table section 4) (livrée à la migration)
- `25-rules-panel` — Câblage du cartouche "Règles de la compétition" — build-team (livrée à la migration)
- `26-submit-team` — Validation finale et soumission de l'équipe (livrée à la migration)
- `27-bc-teams-structure` — BC `teams` — Structure et scaffolding (livrée à la migration)
- `28-teams-aggregate` — BC `teams` — Agrégat `Team` event sourcé (livrée à la migration)
- `29-teams-repository` — BC `teams` — Event store + port `ITeamRepository` (livrée à la migration)
- `30-team-created-app-event` — Publication de l'app event `TeamCreated` depuis le BC `team_creation` (livrée à la migration)
- `31-team-created-listener` — BC `teams` — Consommation `TeamCreated` → équipe "En attente d'inscription" (livrée à la migration)
- `34-team-detail-wired` — BC `teams` — Fiche d'équipe câblée avec données réelles (livrée à la migration)
- `51-team-created-staff-transport` — BC `teams` — Transport du staff initial depuis `team_creation` (livrée à la migration)
- `52-team-staff-tracking` — BC `teams` — Tracking du staff sur événements domaine internes (livrée à la migration)
- `53-team-detail-staff-display` — BC `teams` — Affichage du staff dans la fiche d'équipe (livrée à la migration)
