# BC match_report — Domain : value objects + events inducements

**Priorité : haute**
**Dépend de :** 99, 100
**Contexte :** match_report step2-inducements — couche domaine

## Objectif

Ajouter les value objects et domain events nécessaires à la phase d'achat des inducements.

## Conception

Cf. `docs/specs/match-report/step2-inducements/06-domaine.md`

### Value objects (`domain/value_objects.rs`)

- `TeamValue(u32)` — dérive `PartialOrd + Ord` pour comparaison TopDog/Underdog
- `InducementPurchase { uid: InducementId, qty: u8, unit_cost: u32 }` — `unit_cost` conservé pour `topdog_spending()` sans recalcul

### Domain events (`domain/events.rs`)

```rust
TeamValuesRecorded {
    home_team_value: TeamValue,
    away_team_value: TeamValue,
    recorded_by:     CoachId,
},
InducementsRecorded {
    team_id:     TeamId,
    purchases:   Vec<InducementPurchase>,
    recorded_by: CoachId,
},
StarPlayerEngaged {
    team_id:         TeamId,
    star_player_uid: InducementId,
    recorded_by:     CoachId,
},
```

### Erreurs domaine (`domain/error.rs`)

```rust
BudgetExceeded { spent: u32, budget: u32 },
MaxQtyExceeded { uid: String, qty: u8, max_qty: u8 },
StarPlayerLimitExceeded,
StarPlayerConflict { uid: String },
TeamValuesNotRecorded,
InducementsAlreadyRecorded,
```

## Checklist

- [ ] `TeamValue(u32)` avec derives `PartialOrd + Ord + Serialize + Deserialize`
- [ ] `InducementPurchase { uid, qty, unit_cost }` avec `Serialize + Deserialize`
- [ ] Events `TeamValuesRecorded`, `InducementsRecorded`, `StarPlayerEngaged` dans l'enum `MatchReportDomainEvent`
- [ ] Variants d'erreur dans `DomainError`
- [ ] Tests unitaires : `TeamValue` — validation, comparaison
