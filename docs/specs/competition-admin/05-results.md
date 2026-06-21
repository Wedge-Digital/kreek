# Résultats — Admin compétition

## Phase 2 — Architecture front ✅

Layout split : sidebar journées (gauche) + détail résultats (droite).

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Round sidebar | competitions | `GET .../admin/results/rounds` | `load, resultsChanged from:body` | `roundSelected` | Lecture + sélection |
| Round detail | competitions | `GET .../admin/results/round?round_id={id}` | `roundSelected from:body, resultsChanged from:body` | — | Lecture + action |

### Événements

- `roundSelected` — émis au clic dans la sidebar. Payload : `{ round_id }`
- `resultsChanged` — émis après : valider un match, rejeter un match, valider la journée

### Actions

- `POST .../admin/results/rounds/{round_id}/validate` → valider toute la journée → `HX-Trigger: resultsChanged`
- `POST .../admin/results/matches/{match_id}/validate` → valider un match → `HX-Trigger: resultsChanged`
- `POST .../admin/results/matches/{match_id}/reject` → rejeter un match → `HX-Trigger: resultsChanged`

### JS côté front

Aucun — tout est déclaratif HTMX.

### Règles métier

- Les résultats sont des rapports de match groupés par journée
- Valider une journée rend visibles les modifications liées aux matchs (classement, stats)
- Un match peut être soumis par un coach, puis validé/rejeté par un admin
- On ne peut valider une journée que si tous les matchs sont soit joués soit non joués (pas de match "soumis non validé")

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
