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

Aucun — formulaires HTMX standards.

### Règles métier

- Les résultats sont des rapports de match groupés par journée
- Valider une journée rend visibles les modifications liées aux matchs (classement, stats)
- Un match peut être soumis par un coach, puis validé/rejeté par un admin
- On ne peut valider une journée que si tous les matchs sont soit joués soit non joués (pas de match "soumis non validé")
