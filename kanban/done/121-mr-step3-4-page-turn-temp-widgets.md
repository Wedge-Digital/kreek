# BC match_report — Page hôte step3/step4 + turn-selector + temp-player-selector

**Priorité : haute**
**Dépend de :** 115, 116, 117, 118, 120
**Contexte :** match_report step3-4-actions — frontend BC match_report (1/2)

## Objectif

Implémenter la page hôte step3 et step4, le widget turn-selector et le widget temp-player-selector.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/02-front.md`, `03-back.md`

### Page hôte step3/step4

Handler : `src/app/match_report/io/web/actions_step_controller.rs`
Templates : `src/app/match_report/io/web/templates/step3.html` et `step4.html` (ou un seul template paramétré par `team_side`)

Paramètres route : `space_id`, `mr_id`
- Détermine `team_side` (Home pour step3, Away pour step4)
- Rend la page hôte avec les 5 placeholders `hx-get` + `hx-trigger="load"`
- La page hôte ne porte aucune logique ni VM complexe — seulement les URLs des widgets

Routes câblées :
- `GET /app/{space_id}/match-report/{mr_id}/step3` → `actions_step_controller::get_step`
- `GET /app/{space_id}/match-report/{mr_id}/step4` → `actions_step_controller::get_step`

### Widget turn-selector

Handler : `src/app/match_report/io/web/widgets/turn_selector_widget.rs`
Template : `src/app/match_report/io/web/widgets/turn_selector_widget.html`

- Charge `pm.actions_for(side)` → sait quels tours ont déjà des actions (badge indicator)
- Rend une grille de 16 boutons (tours 1–16)
- Clic → émet `turnSelected { turn: N }` sur `document.body`
- Re-chargé après chaque `actionRecorded` ou `actionDeleted` (`hx-trigger="actionRecorded from:body, actionDeleted from:body"`)
- Isolation : `hx-disinherit="*"`

Paramètre URL : `team_side` (pour savoir quelle liste d'actions regarder)

### Widget temp-player-selector

Handler : `src/app/match_report/io/web/widgets/temp_player_selector_widget.rs`
Template : `src/app/match_report/io/web/widgets/temp_player_selector_widget.html`

- Charge `pm.temp_players_for(side)` depuis la projection (`home_temp_players` / `away_temp_players` JSONB)
- Rend la liste des joueurs temporaires : `{display_name ?? kind_label} ({position_label})`
- Clic → émet `playerSelected { player_id: temp_player_id, player_type: "temp" }` sur `document.body`
- Affiché uniquement si la liste est non vide
- Isolation : `hx-disinherit="*"`

### Wiring routes (dans `routes.rs` + `router.rs`)

Constantes et câblage pour :
- `MATCH_REPORT_STEP3` (si pas encore câblé), `MATCH_REPORT_STEP4`
- `MATCH_REPORT_STEP3_TURN_SELECTOR`, `MATCH_REPORT_STEP4_TURN_SELECTOR`
- `MATCH_REPORT_STEP3_TEMP_PLAYERS`, `MATCH_REPORT_STEP4_TEMP_PLAYERS`

## Checklist

- [ ] `actions_step_controller::get_step` — détermine `team_side` depuis le path, rend la page hôte
- [ ] Templates step3 / step4 avec 5 conteneurs `hx-get` + `hx-trigger="load"`
- [ ] `turn_selector_widget::get` — charge actions par side, rend les 16 boutons avec badge si action présente
- [ ] Template turn-selector avec `hx-disinherit="*"` + événement `turnSelected`
- [ ] `temp_player_selector_widget::get` — charge JSONB temp players, rend la liste
- [ ] Template temp-player-selector avec `hx-disinherit="*"` + événement `playerSelected`
- [ ] Toutes les constantes de routes + méthodes builder
- [ ] Câblage router + `mod.rs`
