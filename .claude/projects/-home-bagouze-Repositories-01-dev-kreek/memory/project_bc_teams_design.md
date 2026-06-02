---
name: bc-teams-design
description: Décisions architecturales du BC teams — event sourcing, machine d'états, composition frontend, cartes kanban 27-43
metadata:
  type: project
---

## BC `teams` — décisions clés

**Why:** Session de conception complète du BC teams (juin 2026), toutes les décisions validées avec le product owner.

**How to apply:** Ces décisions sont structurantes — ne pas revenir dessus sans discussion explicite.

---

### Périmètre

- Le BC `teams` consomme des app events d'autres BCs, il ne produit pas ses propres données sources
- Frontières : `team_creation` (TeamCreated) → `teams` ; `match_report` (MatchPlayed) → `teams` ; `competitions` (TeamEnrolled, SeasonEnded) → `teams`
- Le BC `players` (non encore créé) sera consommé via widgets HTMX uniquement — pas de projection locale

### Machine d'états

**Statut de participation :** `PendingEnrollment` → `Enrolled` → `Dismissed` (admin)

**Phase de jeu (si Enrolled) :**
`ReadyToPlay` → *(MatchPlayed)* → `PlayerImprovement` → `Recruitment` → `Dismissals` → `TemporaryRetirement` → *(auto: CostlyMistakes)* → `ReadyToPlay`

**Fin de saison :** `ReadyToPlay` → *(SeasonEnded)* → `OffSeason` → *(OffSeasonCompleted)* → `PendingEnrollment`

**Multi-saisons :** l'agrégat traverse plusieurs saisons dans le même flux d'événements. Snapshot prévu en fin de saison (non prioritaire).

### Retraite temporaire

- Dure jusqu'à la fin de la saison (pas juste le prochain match)
- Le joueur compte dans les quotas tout au long de la saison
- Peut être renvoyé depuis la phase Dismissals pour libérer son slot
- En repos hors-saison : coach choisit de réengager ou non → si non réengagé, quitte l'effectif

### Event sourcing

- Persistance : `team_event_store` (event_type, event_version défaut "1.0", payload JSONB internally tagged, version séquence par équipe)
- Hydratation : rejeu de tous les événements via `Team::hydrate(&events)` = fold sur `apply()`
- Projection `teams_projection` mise à jour dans la même transaction que l'append (règle dans CLAUDE.md)
- La projection sert uniquement aux requêtes de liste — l'état complet se charge toujours par rejeu

### Composition frontend

- Chaque BC expose ses propres fragments HTML — pas de requêtes SQL cross-BC
- Le tableau joueurs est un widget du BC `players` chargé via `hx-get` dans la page du BC `teams`
- Règle inscrite dans CLAUDE.md

### Cartes kanban

- `tbd/` prêtes : 27 (structure), 28 (agrégat), 29 (event store), 30 (app event publisher), 31 (listener TeamCreated), 33 (dismissal admin), 34 (fiche d'équipe), 42 (projection)
- `to_be_refined/` : 32 (enrollment), 35-40 (phases post-match), 43 (off-season), comp-01 (règles accession)
