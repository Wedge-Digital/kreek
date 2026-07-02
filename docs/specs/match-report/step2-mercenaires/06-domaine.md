# Step 2 — Mercenaires — Domaine

## Récapitulatif des règles métier

| # | Règle | Responsable |
|---|-------|-------------|
| BR1 | `position_id` doit exister dans le roster de l'équipe | Use case |
| BR2 | La position ne peut pas être un journalier | Use case |
| BR3 | Coût = `base_cost + level.extra_cost()` (30 kPo Base / 80 kPo Lvl1) | Use case |
| BR4 | Max 3 mercenaires par équipe par match (toutes positions confondues) | **Domaine** |
| BR5 | Limite roster par position : `count_in_team + qty_in_request ≤ max_qty` | **Domaine** |
| BR6 | Budget total (inducements classiques + mercenaires) ≤ budget inducement | **Domaine** |

BR1–BR3 sont déjà spécifiés en Phase 5 (use case). Ce fichier couvre uniquement BR4–BR6.

---

## Stratégie — Synthetic AllowedInducementSpec

Le domaine ne change pas de signature. Le use case crée des `AllowedInducementSpec` synthétiques pour les mercenaires et les ajoute à la liste existante avant d'appeler `pm.record_inducements`.

```
Use case:
  allowed_specs = classic_specs + merco_specs_synthetics
  purchases     = classic_purchases + merco_purchase_tuples

Domain:
  validate_max_qty(purchases, allowed_specs)      → BR5 couvert
  validate_budget(purchases, allowed_specs, budget) → BR6 couvert
  validate_mercenary_limit(purchases)              → BR4 NOUVEAU
```

**Pourquoi cette approche :**
- Pas de nouveau paramètre sur `record_inducements` — interface domaine stable
- BR5 et BR6 sont automatiquement couverts par les validations existantes
- Un seul ajout minimal dans le domaine pour BR4

---

## Construction des specs synthétiques (use case)

Pour chaque `ValidatedMercenary`, le use case construit un `AllowedInducementSpec` :

```rust
AllowedInducementSpec {
    uid:           InducementId(format!("MERCO:{}:{}", merc.position_id, merc.level.as_str())),
    max_qty:       InducementQty::try_new(max_available).expect("available slots >= 0"),
    unit_cost:     InducementCost::try_new(merc.cost).expect("cost validated at use case boundary"),
    is_star_player: IsStarPlayer(false),
}
```

Avec `max_available = merc.max_qty - count_in_team_for_position` (calculé à partir des `PositionCountDto`).

Le purchase tuple correspondant : `(InducementId("MERCO:{pos}:{level}"), qty)` où `qty` = nombre de fois cette combinaison apparaît dans `cmd.mercenary_purchases`.

---

## Modification domaine — validate_mercenary_limit (UNIQUE ajout)

### Fichier : `src/app/match_report/domain/match_report_pre_match.rs`

Ajout d'une fonction privée `validate_mercenary_limit` et appel dans `validate_purchases` :

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
    validate_mercenary_limit(purchases)    // NOUVEAU — en dernier, après budget
}

fn validate_mercenary_limit(
    purchases: &[(InducementId, u8)],
) -> Result<(), DomainError> {
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

---

## Nouveau variant DomainError

### Fichier : `src/app/match_report/domain/error.rs`

```rust
#[error("trop de mercenaires : {requested} demandés, max {max}")]
TooManyMercenaries { requested: u8, max: u8 },
```

Pas d'autre variant — BR5 remonte `MaxQtyExceeded` et BR6 remonte `BudgetExceeded`, déjà existants.

---

## Pas de nouveau value object domaine

`MercenaryLevel` est défini dans `use_cases/record_inducements_use_case.rs` (Phase 4). Il n'est pas utilisé par le domaine — le domaine ne voit que les UIDs encodés `"MERCO:{pos}:{level}"`. Aucun déplacement nécessaire.

---

## Tests à ajouter

Tous dans le module `#[cfg(test)]` de `match_report_pre_match.rs` :

### BR4 — Limite globale mercenaires

```rust
#[test]
fn record_inducements_fails_when_more_than_3_mercos() {
    // 4 mercos toutes positions confondues → TooManyMercenaries { requested: 4, max: 3 }
}

#[test]
fn record_inducements_with_exactly_3_mercos_succeeds() {
    // 3 mercos → Ok
}

#[test]
fn record_inducements_merco_count_is_sum_of_qtys() {
    // 1 MERCO:pos-a:base qty=2 + 1 MERCO:pos-b:base qty=2 = 4 → TooManyMercenaries
}
```

### BR5 — Limite roster par position (exercice via MaxQtyExceeded existant)

```rust
#[test]
fn record_inducements_merco_respects_position_max_qty() {
    // spec synthétique avec max_qty=1, purchase qty=2 → MaxQtyExceeded
}
```

### BR6 — Budget (exercice via BudgetExceeded existant)

```rust
#[test]
fn record_inducements_merco_cost_counts_toward_budget() {
    // merco cost=180 kPo, budget=100 kPo → BudgetExceeded
}
```

### Cas nominal

```rust
#[test]
fn record_inducements_with_mercos_and_classic_succeed() {
    // 1 inducement classique + 2 mercos, budget suffisant → Ok, liste purchases correcte
}
```

---

## Résumé des fichiers modifiés

| Fichier | Nature |
|---------|--------|
| `src/app/match_report/domain/match_report_pre_match.rs` | Ajout `validate_mercenary_limit` + appel dans `validate_purchases` + tests |
| `src/app/match_report/domain/error.rs` | Ajout `TooManyMercenaries { requested, max }` |
