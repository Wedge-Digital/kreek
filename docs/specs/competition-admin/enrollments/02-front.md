# Inscriptions — Phase 2 : Architecture front ✅

Page d'assemblage à widgets. Les données d'inscription (équipes pending / enrolled) viennent du BC `teams`. Les données d'invitation (coachs sans équipe) viennent du BC `competitions`.

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Stats bar | competitions | inline | — | — | Lecture |
| Pending list | teams | `GET /app/{space_id}/team/widgets/pending?competition_id=...&season_id=...` | `load, enrollmentChanged from:body` | — | Lecture + action |
| Enrolled list | teams | `GET /app/{space_id}/team/widgets/enrolled?competition_id=...&season_id=...` | `load, enrollmentChanged from:body` | — | Lecture + action |

### Événements

- `enrollmentChanged` — émis par les widgets `teams` (approve/reject/dismiss/approve-all) et par le handler `close` (competitions). Tous les widgets se rechargent.

### Actions

| Action | BC | Endpoint |
|---|---|---|
| Valider une inscription | teams | `POST /app/{space_id}/team/{team_id}/enrollment/approve` → `HX-Trigger: enrollmentChanged` |
| Refuser une inscription | teams | `POST /app/{space_id}/team/{team_id}/enrollment/reject` → `HX-Trigger: enrollmentChanged` |
| Renvoyer une équipe | teams | `POST /app/{space_id}/team/{team_id}/enrollment/dismiss` → `HX-Trigger: enrollmentChanged` |
| Tout valider | teams | `POST /app/{space_id}/team/widgets/pending/approve-all?competition_id=...&season_id=...` → `HX-Trigger: enrollmentChanged` |
| Clôturer inscriptions | competitions | `POST .../admin/enrollments/close` → `HX-Trigger: enrollmentChanged` |

### JS côté front

Aucun — tout est déclaratif HTMX.
