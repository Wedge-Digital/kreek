# BC match_report — Domaine mercenaires (BR4)

**Priorité : haute**
**Dépend de :** —
**Contexte :** `docs/specs/match-report/step2-mercenaires/06-domaine.md`

## Objectif

Ajouter la règle métier "max 3 mercenaires par équipe" dans le domaine, avec son test unitaire.

## Conception

### 1. DomainError — src/app/match_report/domain/error.rs

Ajouter le variant :

```rust
#[error("trop de mercenaires : {requested} demandés, max {max}")]
TooManyMercenaries { requested: u8, max: u8 },
```

### 2. validate_mercenary_limit — src/app/match_report/domain/match_report_pre_match.rs

Ajouter la fonction privée et l'appeler dans `validate_purchases` :

```rust
fn validate_purchases(
    purchases: &[(InducementId, u8)],
    allowed_specs: &[AllowedInducementSpec],
    opponent_star_uids: &[InducementId],
    budget: u32,
) -> Result<(), DomainError> {
    validate_max_qty(purchases, allowed_specs)?;
    validate_star_player_limit(purchases, allowed_specs)?;
    validate_star_player_conflict(purchases, allowed_specs, opponent_star_uids)?;
    validate_budget(purchases, allowed_specs, budget)?;
    validate_mercenary_limit(purchases)    // NOUVEAU
}

fn validate_mercenary_limit(purchases: &[(InducementId, u8)]) -> Result<(), DomainError> {
    let total: u8 = purchases
        .iter()
        .filter(|(uid, _)| uid.0.starts_with("MERCO:"))
        .map(|(_, qty)| *qty)
        .sum();
    if total > 3 {
        Err(DomainError::TooManyMercenaries { requested: total, max: 3 })
    } else {
        Ok(())
    }
}
```

### 3. Tests unitaires (même fichier, module tests)

- `record_inducements_fails_when_more_than_3_mercos` — qty sum > 3 → `TooManyMercenaries`
- `record_inducements_with_exactly_3_mercos_succeeds` — qty sum = 3 → Ok
- `record_inducements_merco_count_is_sum_of_qtys` — 2 positions × qty 2 = 4 → `TooManyMercenaries`
- `record_inducements_merco_respects_position_max_qty` — spec max_qty=1, qty=2 → `MaxQtyExceeded`
- `record_inducements_merco_cost_counts_toward_budget` — coût merco dépasse budget → `BudgetExceeded`
- `record_inducements_with_mercos_and_classic_succeed` — mercos + classiques, budget ok → Ok

Utiliser des UIDs `"MERCO:pos-a:base"` avec des `AllowedInducementSpec` synthétiques pour les tests.

## Checklist

- [ ] `TooManyMercenaries` ajouté à `DomainError`
- [ ] `validate_mercenary_limit` implémentée
- [ ] Appel ajouté dans `validate_purchases`
- [ ] 6 tests écrits et passants
- [ ] `cargo test` passe
