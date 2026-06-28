# BC match_report — Use cases record_action + delete_action

**Priorité : haute**
**Dépend de :** 115, 117
**Contexte :** match_report step3-4-actions — use cases

## Objectif

Implémenter les deux use cases de gestion des actions match : enregistrement et suppression.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/05-use-cases.md`

### Nouveau fichier `use_cases/record_action_use_case.rs`

```rust
pub async fn execute(
    cmd: RecordActionCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
) -> Result<RecordActionOutcome, RecordActionError>

pub struct RecordActionCommand {
    pub match_report_id: MatchReportId,
    pub team_side:       TeamSide,
    pub turn:            TurnNumber,   // déjà validé par le handler
    pub player:          ActionPlayer,
    pub action:          MatchActionType,
    pub recorded_by:     CoachId,
}

pub struct RecordActionOutcome {
    pub action_id: String,  // renvoyé dans HX-Trigger: actionRecorded
}

pub enum RecordActionError {
    NotFound,
    NotInPreMatchPhase,
    PlayerNotFound(String),
    TempPlayerNotFound(String),
    Domain(DomainError),
    Repository(String),
}
```

### Orchestration `record_action`

1. Charge agrégat → `PreMatch`, sinon `NotInPreMatchPhase`
2. Résout `player_display_name` :
   - `Regular(player_id)` → `player_data.find_player_display(player_id)` → `"{name} (#{jersey})"` ; `None` → `PlayerNotFound`
   - `Temp(temp_id)` → cherche dans `pm.temp_players_for(cmd.team_side)` ; `None` → `TempPlayerNotFound`
3. Génère `action_id = ActionId(Ulid::new().to_string())`
4. `pm.record_action(team_side, turn, player, action, display_name, action_id)` → event
5. `repo.append(event)`
6. Retourne `RecordActionOutcome { action_id }`

### Nouveau fichier `use_cases/delete_action_use_case.rs`

```rust
pub async fn execute(
    cmd: DeleteActionCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<(), DeleteActionError>

pub struct DeleteActionCommand {
    pub match_report_id: MatchReportId,
    pub action_id:       ActionId,
    pub deleted_by:      CoachId,
}

pub enum DeleteActionError {
    NotFound,
    NotInPreMatchPhase,
    Domain(DomainError),
    Repository(String),
}
```

### Orchestration `delete_action`

1. Charge agrégat → `PreMatch`, sinon `NotInPreMatchPhase`
2. `pm.delete_action(&cmd.action_id, cmd.deleted_by)` → event ou `DomainError::ActionNotFound`
3. `repo.append(event)`

## Checklist

- [ ] `RecordActionCommand`, `RecordActionOutcome`, `RecordActionError`
- [ ] Résolution `display_name` pour Regular et Temp
- [ ] Génération `ActionId` (Ulid)
- [ ] `DeleteActionCommand`, `DeleteActionError`
- [ ] Gestion `DomainError::ActionNotFound` dans le delete
- [ ] Tests unitaires des deux use cases
