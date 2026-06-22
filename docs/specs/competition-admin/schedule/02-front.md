# Calendrier — Phase 2 : Architecture front ✅

Layout split : sidebar journées (gauche) + détail journée (droite).

## Widgets

| Widget | BC | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|---|
| Actions globales | — | inline (page) | — | — | Action |
| Round sidebar | competitions | `GET .../admin/schedule/rounds` | `load, scheduleChanged from:body` | `roundSelected` | Lecture + sélection |
| Round detail | competitions | `GET .../admin/schedule/round?round_id={id}` | `roundSelected from:body, scheduleChanged from:body` | — | Lecture + édition |

## Événements

- `roundSelected` — émis au clic sur une journée dans la sidebar. Payload : `{ round_id }`
- `scheduleChanged` — émis après toute mutation (journée ajoutée/supprimée, match ajouté/supprimé, rencontres générées). Les deux widgets se rechargent.

## Actions globales (barre du haut)

- `POST .../admin/schedule/generate-all` → générer les rencontres de toutes les journées → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/clear-all` → vider toutes les rencontres → `HX-Trigger: scheduleChanged`

## Actions sidebar (bas de la sidebar)

- `POST .../admin/schedule/rounds` → ajouter une journée (body: `{type, name, date_start, date_end}`) → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/rounds/rest` → ajouter une journée de repos (body: `{name, date_start, date_end}`) → `HX-Trigger: scheduleChanged`

## Actions détail journée

### Config date
- `PUT .../admin/schedule/rounds/{round_id}` → modifier dates/type d'une journée → `HX-Trigger: scheduleChanged`
- `DELETE .../admin/schedule/rounds/{round_id}` → supprimer une journée → `HX-Trigger: scheduleChanged`

### Matchs de la journée
- `POST .../admin/schedule/rounds/{round_id}/generate` → générer les rencontres de cette journée → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/rounds/{round_id}/clear` → vider les matchs de cette journée → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/rounds/{round_id}/matches` → ajouter un match (body: `{home_team_id, away_team_id}`) → `HX-Trigger: scheduleChanged`
- `DELETE .../admin/schedule/rounds/{round_id}/matches/{match_id}` → supprimer un match → `HX-Trigger: scheduleChanged`
- `POST .../admin/schedule/rounds/{round_id}/matches/{match_id}/postpone` → reporter un match

## Formulaire ajout match (inline)

Visible au clic sur "➕ Ajouter un match" dans le détail d'une journée. Deux selects TomSelect côte à côte :
- **Domicile** : TomSelect searchable sur nom d'équipe, coach, roster. Rendu custom : nom en gras + coach · roster en note.
- **Extérieur** : idem.
- **Bouton "✓ Ajouter"** : fond bleu, border outline. POST le match.
- **Bouton "Annuler"** : ferme le formulaire.

TomSelect initialisé via Alpine `init()`/`destroy()` lifecycle (dans le template final) ou via script scoped. Les options sont alimentées par le port ITeamInfoPort (équipes enrolled de la saison).

## JS côté front

- Toggle date fixe/plage : JS minimal inline dans le round detail
- Formulaire ajout match : show/hide au clic, TomSelect lifecycle
- Sélection journée dans sidebar : émet `roundSelected` via `htmx.trigger(document.body, 'roundSelected', { round_id })`

## Règles métier

- Une journée a soit une date fixe soit une plage (date_start, date_end)
- Une journée de repos n'a pas de matchs
- Une saison peut avoir jusqu'à ~20 journées
- La génération automatique de rencontres distribue les matchs entre équipes d'une même poule (round-robin)
- Un match a un ID unique (utilisé par le futur BC MatchReport)
- Les infos des matchs passés (scores) viendront d'un futur port vers le BC MatchReport
