# BC match_report — Action-panel + action-log + record_action_controller

**Priorité : haute**
**Dépend de :** 119, 121
**Contexte :** match_report step3-4-actions — frontend BC match_report (2/2)

## Objectif

Implémenter le widget action-panel, le widget action-log, et le controller POST/DELETE pour l'enregistrement et la suppression d'actions.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/02-front.md`, `03-back.md`

### Widget action-panel

Handler : `src/app/match_report/io/web/widgets/action_panel_widget.rs`
Template : `src/app/match_report/io/web/widgets/action_panel_widget.html`

- Affiché après `turnSelected` ET `playerSelected` (les deux reçus)
- Paramètres URL : `turn`, `player_id`, `player_type`, `team_side`
- Rend la grille des boutons d'action (TD, Passe, Interception, Agression, Lancer, Sortie, MVP, Blessé)
- **Blessé** : clic → affiche inline un sélecteur de blessure (Commotion, Amoché, BlessureSerieuse, Sequel, Mort) ; `Sequel` → affiche sélecteur de stat (-AV, -MA, -PA, -AG, -ST) → alors POST
- Tous les autres : POST immédiat au clic
- Isolation : `hx-disinherit="*"`

### Widget action-log

Handler : `src/app/match_report/io/web/widgets/action_log_widget.rs`
Template : `src/app/match_report/io/web/widgets/action_log_widget.html`

- Paramètre URL : `team_side`
- Charge depuis `match_report_actions` (via `find_actions_by_match_and_side`)
- Rend la liste des actions avec : tour, nom joueur, libellé action
- Chaque ligne a un bouton Supprimer → `DELETE /app/{space_id}/match-report/{mr_id}/actions/{action_id}`
- Re-chargé après `actionRecorded` et `actionDeleted` via `hx-trigger="actionRecorded from:body, actionDeleted from:body"`
- Isolation : `hx-disinherit="*"`

### `record_action_controller.rs`

```
POST /app/{space_id}/match-report/{mr_id}/step3/actions
POST /app/{space_id}/match-report/{mr_id}/step4/actions
DELETE /app/{space_id}/match-report/{mr_id}/actions/{action_id}
```

**POST** : parse `RecordActionForm` → construit `RecordActionCommand` → `record_action_use_case::execute` → répond avec `HX-Trigger: {"actionRecorded": {"action_id": "..."}}` (déclenche le rechargement du turn-selector et de l'action-log)

**DELETE** : parse `action_id` → construit `DeleteActionCommand` → `delete_action_use_case::execute` → répond avec `HX-Trigger: {"actionDeleted": {}}` (déclenche le rechargement)

`RecordActionForm` :

```rust
pub struct RecordActionForm {
    pub turn:         u8,
    pub player_id:    String,
    pub player_type:  String,  // "regular" | "temp"
    pub action_type:  String,
    pub injury_type:  Option<String>,
    pub sequel_stat:  Option<String>,
    pub team_side:    String,  // "home" | "away"
}
```

Le handler construit `TurnNumber::try_new(form.turn)?`, `ActionPlayer`, `MatchActionType` avant d'appeler le use case.

### Wiring routes

Constantes et câblage pour :
- `MATCH_REPORT_STEP3_ACTION_PANEL`, `MATCH_REPORT_STEP4_ACTION_PANEL`
- `MATCH_REPORT_STEP3_LOG`, `MATCH_REPORT_STEP4_LOG`
- `MATCH_REPORT_STEP3_ACTIONS`, `MATCH_REPORT_STEP4_ACTIONS`
- `MATCH_REPORT_ACTION` (DELETE)

## Checklist

- [ ] `action_panel_widget::get` + template avec boutons action et flux Blessé deux étapes
- [ ] `action_log_widget::get` + template avec liste + bouton supprimer
- [ ] `record_action_controller::post_action` — parse form, construit command, appelle use case, répond `HX-Trigger`
- [ ] `record_action_controller::delete_action` — parse action_id, appelle use case, répond `HX-Trigger`
- [ ] `RecordActionForm` avec validation handler (TurnNumber, ActionPlayer, MatchActionType)
- [ ] Constantes de routes + méthodes builder
- [ ] Câblage router + `mod.rs`
- [ ] CSS pour action-panel et action-log
