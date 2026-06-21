# Calendrier — Admin compétition

## Phase 2 — Architecture front ✅

Layout split : sidebar journées (gauche) + détail journée (droite).

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Actions globales | — | inline | — | — | Action |
| Round sidebar | competitions | `GET .../admin/schedule/rounds` | `load, scheduleChanged from:body` | `roundSelected` | Lecture + sélection |
| Round detail | competitions | `GET .../admin/schedule/round?round_id={id}` | `roundSelected from:body, scheduleChanged from:body` | — | Lecture + édition |

### Événements

- `roundSelected` — émis au clic sur une journée dans la sidebar. Payload : `{ round_id }`
- `scheduleChanged` — émis après toute mutation. Les deux widgets se rechargent.

### Actions

- `POST .../admin/schedule/generate` → générer le calendrier → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/clear` → vider le calendrier → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/rounds` → ajouter une journée (body: `{type, date_start, date_end}`) → `HX-Trigger: scheduleChanged`
- `DELETE .../admin/schedule/rounds/{round_id}` → supprimer une journée → `HX-Trigger: scheduleChanged`
- `PUT .../admin/schedule/rounds/{round_id}` → modifier dates → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/rounds/{round_id}/matches/{match_id}/postpone` → reporter un match
- `DELETE .../admin/schedule/rounds/{round_id}/matches/{match_id}` → supprimer un match

### JS côté front

Toggle date fixe/plage : JS minimal inline dans le round detail widget.

### Règles métier

- Une journée a soit une date fixe soit une plage (date_start, date_end)
- Une saison peut avoir jusqu'à ~20 journées
- Les journées de repos n'ont pas de matchs
- Le calendrier est basé sur la structure définie à la création (poules + nb équipes)
