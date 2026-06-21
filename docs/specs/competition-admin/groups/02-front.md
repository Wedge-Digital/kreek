# Poules — Admin compétition

## Phase 2 — Architecture front ✅

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Actions bar | — | inline | — | — | Action |
| Unassigned pool | competitions | `GET .../admin/groups/unassigned` | `load, groupsChanged from:body` | — | Lecture + drag |
| Group cards | competitions | `GET .../admin/groups/cards` | `load, groupsChanged from:body` | — | Lecture + drop |

### Événements

- `groupsChanged` — émis après : tirage aléatoire, vider les poules, enregistrer, déplacer une équipe

### Actions

- `POST .../admin/groups/random-draw` → tirage aléatoire → `HX-Trigger: groupsChanged`
- `POST .../admin/groups/reset` → vider les poules → `HX-Trigger: groupsChanged`
- `POST .../admin/groups/assign` → assigner une équipe à une poule (body: `{team_id, group_id}`) → `HX-Trigger: groupsChanged`
- `POST .../admin/groups/save` → enregistrer l'état courant

### JS côté front

Drag & drop : JS inline scoped dans le widget (Alpine `init()`/`destroy()`). Au drop, un POST `assign` est envoyé au serveur.
