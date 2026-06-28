# Step 3 & 4 — Actions match — Use cases

---

## Use case 1 : `record_action_use_case` (nouveau)

### Signature

```rust
pub async fn execute(
    cmd: RecordActionCommand,
    repo: &dyn IMatchReportRepository,
    player_data: &dyn IPlayerDataPort,
) -> Result<RecordActionOutcome, RecordActionError>

pub struct RecordActionOutcome {
    pub action_id: String,  // renvoyé dans HX-Trigger: actionRecorded
}
```

### Orchestration

1. Charge l'agrégat via `repo.find_by_id()` → doit être `PreMatch`, sinon `NotInPreMatchPhase`
2. Résout `player_display_name` selon le type de joueur :
   - `ActionPlayer::Regular(player_id)` → appelle `player_data.find_player_display(player_id)` → construit `"{personal_name} (#{jersey})"` ; `None` → `PlayerNotFound`
   - `ActionPlayer::Temp(temp_player_id)` → cherche dans `pm.home_temp_players` / `pm.away_temp_players` selon `cmd.team_side` ; `None` → `TempPlayerNotFound`
3. Génère un `ActionId` (Ulid)
4. Appelle `pm.record_action(cmd.team_side, cmd.turn, cmd.player, cmd.action, player_display_name, action_id)` → event `ActionRecorded` ou `DomainError`
5. Persiste via `repo.append()`
6. Retourne `RecordActionOutcome { action_id }`

### Erreurs

```rust
pub enum RecordActionError {
    NotFound,
    NotInPreMatchPhase,
    PlayerNotFound(String),
    TempPlayerNotFound(String),
    Domain(DomainError),
    Repository(String),
}
```

---

## Use case 2 : `delete_action_use_case` (nouveau)

### Signature

```rust
pub async fn execute(
    cmd: DeleteActionCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<(), DeleteActionError>
```

### Orchestration

1. Charge l'agrégat → `PreMatch`, sinon `NotInPreMatchPhase`
2. Appelle `pm.delete_action(cmd.action_id)` → event `ActionDeleted` ou `DomainError::ActionNotFound`
3. Persiste via `repo.append()`

### Erreurs

```rust
pub enum DeleteActionError {
    NotFound,
    NotInPreMatchPhase,
    Domain(DomainError),
    Repository(String),
}
```

---

## Use case 3 : `init_temp_players_use_case` (nouveau)

Crée (ou recrée) les joueurs temporaires d'une équipe après l'enregistrement de ses inducements.

### Signature

```rust
pub async fn execute(
    cmd: InitTempPlayersCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    player_data: &dyn IPlayerDataPort,
) -> Result<(), InitTempPlayersError>
```

### Orchestration

1. Charge l'agrégat → `PreMatch`, sinon `NotInPreMatchPhase`
2. Détermine `team_side` depuis `cmd.team_id` (home ou away)
3. Si des TempPlayers existent déjà pour cette équipe :
   - Appelle `pm.reset_temp_players(cmd.team_id)` → event `TempPlayersReset`
   - Persiste immédiatement via `repo.append()`
4. Collecte les **star players** depuis l'event store :
   - Filtre les events `StarPlayerEngaged { team_id }` de l'agrégat (champ `pm.star_player_uids_for(&cmd.team_id)`)
   - Pour chaque UID : crée `TempPlayer { kind: StarPlayer { ref_uid, position_uid: "" }, display_name: Some(star_name) }`
   - Le nom de la star n'est pas disponible ici sans appel au référentiel. **Le `display_name` est le `ref_uid` en V1** ; l'adapter BC References pourra enrichir via `ITeamDataPort` dans une future itération.
5. Collecte les **mercenaires** :
   - Filtre `pm.purchases_for(&cmd.team_id)` où `uid == "MERCENARY_PLAYER"`
   - Pour chaque unité achetée : crée `TempPlayer { kind: Mercenary { position_uid: String::new() }, display_name: None }`
   - La position du mercenaire n'est pas capturée en V1 (le choix de position est une future carte).
6. Collecte les **journaliers** :
   - Appelle `player_data.count_available_players(&cmd.team_id.to_string())` → `MatchPlayerCountDto`
   - `n_journaliers = max(0, 11 - dto.available) as u8`
   - Appelle `team_data.find_journalier_position(&cmd.team_id.to_string())` → `JournalierPositionDto` ; `None` → `JournalierPositionUnavailable`
   - Crée `n_journaliers × TempPlayer { kind: Journalier { position_uid }, display_name: None }`
7. Assemble la liste complète : `stars + mercs + journaliers`
8. Appelle `pm.init_temp_players(cmd.team_id, players)` → event `TempPlayersInitialized`
9. Persiste via `repo.append()`

### Erreurs

```rust
pub enum InitTempPlayersError {
    NotFound,
    NotInPreMatchPhase,
    JournalierPositionUnavailable(String),
    PlayerCountUnavailable(String),
    Repository(String),
}
```

### Déclenchement

Appelé depuis `inducements_controller::post_inducements()` **après** `record_inducements_use_case::execute()`, dans la même requête HTTP. Le handler appelle les deux use cases séquentiellement — ce n'est pas de la logique métier mais de l'orchestration de flux HTTP.

---

## Règles ne relevant pas des use cases

Les décisions suivantes appartiennent au domaine (Phase 6) :

- Vérifier que `turn` est dans 1..=16 → `DomainError::InvalidTurn`
- Vérifier que `player_type` et `player_id` sont cohérents avec la liste connue → `DomainError::UnknownPlayer`
- Valider qu'une `InjuryType::Sequel` porte bien un `SequelStat` → `DomainError::MissingSequelStat`
- Valider que l'`ActionId` à supprimer existe → `DomainError::ActionNotFound`
- Générer les `TempPlayerId` (Ulid) → responsabilité du use case (service d'ID)

---

## Limites V1 documentées

| Limitation | Effet | Future carte |
|---|---|---|
| Nom star player = `ref_uid` dans `display_name` | Affiché tel quel dans le log | Résolution via référentiel dans `init_temp_players` |
| Mercenaire sans `position_uid` | Affiché "Mercenaire" sans position | Choix de position en step 2 (carte inducements V2) |
| Tous les joueurs réguliers marqués `is_available: true` | Aucun joueur grisé dans le player-selector | Tracking disponibilité dans BC Players |
