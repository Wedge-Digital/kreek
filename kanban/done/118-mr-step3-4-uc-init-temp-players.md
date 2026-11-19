# BC match_report — Use case init_temp_players + extension record_inducements

**Priorité : haute**
**Dépend de :** 115, 117
**Contexte :** match_report step3-4-actions — use cases

## Objectif

Implémenter le use case `init_temp_players` et étendre `record_inducements_use_case` pour qu'il le déclenche séquentiellement.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/05-use-cases.md`

### Nouveau fichier `use_cases/init_temp_players_use_case.rs`

```rust
pub async fn execute(
    cmd: InitTempPlayersCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<(), InitTempPlayersError>

pub struct InitTempPlayersCommand {
    pub match_report_id: MatchReportId,
    pub team_id:         TeamId,
}

pub enum InitTempPlayersError {
    NotFound,
    NotInPreMatchPhase,
    JournalierPositionUnavailable(String),
    PlayerCountUnavailable(String),
    Repository(String),
}
```

### Orchestration

1. Charge agrégat → `PreMatch`, sinon `NotInPreMatchPhase`
2. Si `pm.temp_players_for(side)` non vide → `pm.reset_temp_players(team_id)` + `repo.append(reset_event)`
3. Stars : `pm.star_player_uids_for(&cmd.team_id)` → crée `TempPlayer { kind: StarPlayer, display_name: Some(ref_uid) }` pour chacun
4. Mercs : filtre `pm.purchases_for(&cmd.team_id)` sur `uid == "MERCENARY_PLAYER"` → crée `TempPlayer { kind: Mercenary { position_uid: "" }, display_name: None }` × `qty`
5. Journaliers : `player_data.count_available_players(&cmd.team_id)` → `n = max(0, 11 - count)` ; `team_data.find_journalier_position(&cmd.team_id)` → `JournalierPositionDto` ; crée `n × TempPlayer { kind: Journalier { position_uid } }`
6. Assemble `stars + mercs + journaliers`, génère un `TempPlayerId` (Ulid) pour chacun
7. `pm.init_temp_players(cmd.team_id, players)` → event `TempPlayersInitialized`
8. `repo.append(init_event)`

### Extension `record_inducements_use_case.rs`

Après `repo.append_many(events)` (succès), appelle séquentiellement :

```rust
init_temp_players_use_case::execute(
    InitTempPlayersCommand { match_report_id: cmd.match_report_id, team_id: cmd.team_id },
    repo, team_data, player_data,
).await?;
```

Le handler `inducements_controller::post_inducements` reçoit `player_data` en paramètre depuis le contexte.

## Checklist

- [ ] `InitTempPlayersCommand` + `InitTempPlayersError`
- [ ] Collecte stars (display_name = ref_uid en V1)
- [ ] Collecte mercs (`MERCENARY_PLAYER`, qty × TempPlayer, position_uid vide)
- [ ] Collecte journaliers (`max(0, 11 - count)`, via `IPlayerDataPort` + `ITeamDataPort`)
- [ ] Reset conditionnel si TempPlayers existants
- [ ] Génération des `TempPlayerId` (Ulid)
- [ ] Extension de `record_inducements_use_case` — appel séquentiel à `init_temp_players`
- [ ] Tests unitaires du use case
