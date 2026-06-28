# BC match_report — Domain : value objects + events step3-4

**Priorité : haute**
**Dépend de :** 113
**Contexte :** match_report step3-4-actions — couche domaine

## Objectif

Ajouter les value objects, events domaine et erreurs domaine nécessaires à l'enregistrement des actions de match (étapes 3 et 4).

## Conception

Cf. `docs/specs/match-report/step3-4-actions/06-domaine.md`

### Value objects (`domain/value_objects.rs`)

- `ActionId(String)` — ULID, généré par le use case, Serialize + Deserialize
- `TempPlayerId(String)` — ULID, généré par le use case, Serialize + Deserialize
- `TurnNumber(u8)` — smart constructor `try_new(value: u8) -> Result<Self, DomainError>` ; valide 1..=16
- `TeamSide { Home, Away }` — Copy, Serialize + Deserialize
- `ActionPlayer { Regular(PlayerId), Temp(TempPlayerId) }` — Serialize + Deserialize
- `MatchActionType` — enum avec variante `Blesse { injury: InjuryType }`, Serialize + Deserialize
- `InjuryType` — enum avec variante `Sequel { stat: SequelStat }`, Serialize + Deserialize
- `SequelStat { MinusAv, MinusMa, MinusPa, MinusAg, MinusSt }` — Copy, Serialize + Deserialize
- `TempPlayer { id, team_id, kind, display_name: Option<String> }` — Serialize + Deserialize
- `TempPlayerKind { StarPlayer { ref_uid, position_uid }, Mercenary { position_uid }, Journalier { position_uid } }` — Serialize + Deserialize
- `MatchAction { id, turn, player, action, player_display_name: String }` — Serialize + Deserialize

### Domain events (`domain/events.rs`)

```rust
TempPlayersInitialized {
    team_id: TeamId,
    players: Vec<TempPlayer>,
},
TempPlayersReset {
    team_id: TeamId,
},
ActionRecorded {
    action_id:           ActionId,
    team_side:           TeamSide,
    turn:                TurnNumber,
    player:              ActionPlayer,
    action:              MatchActionType,
    player_display_name: String,
    recorded_by:         CoachId,
},
ActionDeleted {
    action_id:  ActionId,
    team_side:  TeamSide,
    deleted_by: CoachId,
},
```

### Erreurs domaine (`domain/error.rs`)

```rust
InvalidTurn(u8),        // tour hors 1..=16
ActionNotFound(String), // action_id introuvable
```

## Checklist

- [ ] Tous les value objects avec `Serialize + Deserialize` (et `Copy` pour `TeamSide`, `SequelStat`)
- [ ] `TurnNumber::try_new` — rejette 0 et 17+, accepte 1 et 16
- [ ] 4 nouveaux events dans `MatchReportDomainEvent`
- [ ] 2 nouveaux variants dans `DomainError` avec message `Display`
- [ ] Tests unitaires : `turn_number_rejects_0`, `turn_number_rejects_17`, `turn_number_accepts_1_and_16`
