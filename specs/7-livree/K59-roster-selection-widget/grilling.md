# K59 — Grilling

> Reconstitué à la migration (2026-11-19), à partir des cartes elles-mêmes. Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K59-B1** — Widget de sélection de roster avec tiers (ticket `19-roster-selection-widget`)
- **K59-B2** — Câblage widget roster → table joueurs dans build-team (ticket `20-build-team-roster-wiring`)
- **K59-B3** — Engager un joueur (bouton + de la table) (ticket `21-hire-player`)
- **K59-B4** — Renvoyer un joueur (bouton − de la table) (ticket `22-fire-player`)
- **K59-B5** — Mise à jour du tableau staff en fonction du roster sélectionné (ticket `23-staff-table-roster-update`)
- **K59-B6** — Achat et retrait de staff (table section 4) (ticket `24-buy-remove-staff`)
- **K59-B7** — Câblage du cartouche "Règles de la compétition" — build-team (ticket `25-rules-panel`)
- **K59-B8** — Validation finale et soumission de l'équipe (ticket `26-submit-team`)
- **K59-B9** — BC `teams` — Structure et scaffolding (ticket `27-bc-teams-structure`)
- **K59-B10** — BC `teams` — Agrégat `Team` event sourcé (ticket `28-teams-aggregate`)
- **K59-B11** — BC `teams` — Event store + port `ITeamRepository` (ticket `29-teams-repository`)
- **K59-B12** — Publication de l'app event `TeamCreated` depuis le BC `team_creation` (ticket `30-team-created-app-event`)
- **K59-B13** — BC `teams` — Consommation `TeamCreated` → équipe "En attente d'inscription" (ticket `31-team-created-listener`)
- **K59-B14** — BC `teams` — Fiche d'équipe câblée avec données réelles (ticket `34-team-detail-wired`)
- **K59-B15** — BC `teams` — Transport du staff initial depuis `team_creation` (ticket `51-team-created-staff-transport`)
- **K59-B16** — BC `teams` — Tracking du staff sur événements domaine internes (ticket `52-team-staff-tracking`)
- **K59-B17** — BC `teams` — Affichage du staff dans la fiche d'équipe (ticket `53-team-detail-staff-display`)

## Hors périmètre

Non tracé dans le dépôt.
