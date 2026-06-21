# Inscriptions — Admin compétition

## Phase 2 — Architecture front ✅

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Stats bar | competitions | inline | — | — | Lecture |
| Pending list | competitions | `GET .../admin/enrollments/pending` | `load, enrollmentChanged from:body` | — | Lecture + action |
| Enrolled list | competitions | `GET .../admin/enrollments/enrolled` | `load, enrollmentChanged from:body` | — | Lecture + action |
| Coaches waiting | competitions | `GET .../admin/enrollments/coaches-waiting` | `load, enrollmentChanged from:body` | — | Lecture |

### Événements

- `enrollmentChanged` — émis après : valider, refuser, renvoyer, tout valider, clôturer inscriptions. Tous les widgets se rechargent.

### Actions

- `POST .../admin/enrollments/{team_id}/approve` → valider une inscription → `HX-Trigger: enrollmentChanged`
- `POST .../admin/enrollments/{team_id}/reject` → refuser → `HX-Trigger: enrollmentChanged`
- `POST .../admin/enrollments/{team_id}/dismiss` → renvoyer → `HX-Trigger: enrollmentChanged`
- `POST .../admin/enrollments/approve-all` → tout valider → `HX-Trigger: enrollmentChanged`
- `POST .../admin/enrollments/close` → clôturer les inscriptions → `HX-Trigger: enrollmentChanged`

### JS côté front

Aucun — tout est déclaratif HTMX.

## Phase 3 — Architecture back

_À compléter_

## Phase 4 — DTOs

_À compléter_

## Phase 5 — Use cases

_À compléter_

## Phase 6 — Domaine

_À compléter_

## Phase 7 — Intégration

_À compléter_

## Phase 8 — Cartes kanban

_À produire après les phases 3-7_
