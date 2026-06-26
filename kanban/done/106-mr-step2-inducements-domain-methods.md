# BC match_report — Domain : méthodes agrégat MatchReportPreMatch

**Priorité : haute**
**Dépend de :** 105
**Contexte :** match_report step2-inducements — couche domaine

## Objectif

Ajouter les champs et méthodes métier sur `MatchReportPreMatch` pour gérer les TeamValues et les achats d'inducements.

## Conception

Cf. `docs/specs/match-report/step2-inducements/06-domaine.md`

### Nouveaux champs (`domain/match_report_pre_match.rs`)

```rust
pub home_team_value:  Option<TeamValue>,
pub away_team_value:  Option<TeamValue>,
pub home_inducements: Option<Vec<InducementPurchase>>,
pub away_inducements: Option<Vec<InducementPurchase>>,
```

### Nouvelles méthodes

| Méthode | Règle |
|---|---|
| `record_team_values(home_tv, away_tv, recorded_by) -> (Self, MatchReportDomainEvent)` | Stocke les deux TV, émet `TeamValuesRecorded` |
| `topdog_team_id() -> &TeamId` | TV plus haute = TopDog ; égalité = home team |
| `underdog_team_id() -> &TeamId` | L'autre équipe |
| `topdog_spending() -> u32` | Σ qty × unit_cost des achats TopDog ; 0 si None ou vide |
| `inducement_budget_for(team_id, treasury) -> u32` | TopDog : treasury ; Underdog : \|diff TV\| + topdog_spending + min(treasury, 50_000) |
| `record_inducements(team_id, purchases, budget, allowed_specs, opponent_star_uids) -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError>` | Valide budget, maxQty, ≤ 2 stars, pas de star adverse ; émet `InducementsRecorded` + N × `StarPlayerEngaged` |
| `is_inducements_phase_complete() -> bool` | `home_inducements.is_some() && away_inducements.is_some()` |

### Règles de validation dans `record_inducements` (dans cet ordre)

1. `qty ≤ max_qty` pour chaque achat → `MaxQtyExceeded`
2. Nombre de star players achetés ≤ 2 → `StarPlayerLimitExceeded`
3. Pas de star player en commun avec `opponent_star_uids` → `StarPlayerConflict`
4. `total_cost ≤ budget` → `BudgetExceeded`

## Checklist

- [ ] Champs `home_team_value`, `away_team_value`, `home_inducements`, `away_inducements`
- [ ] `record_team_values()`
- [ ] `topdog_team_id()` / `underdog_team_id()`
- [ ] `topdog_spending()`
- [ ] `inducement_budget_for()` — TopDog et Underdog, cap 50 000 trésorerie Underdog
- [ ] `record_inducements()` — toutes les validations + émission `Vec<MatchReportDomainEvent>`
- [ ] `is_inducements_phase_complete()`
- [ ] Tests unitaires (cf. tableau Phase 6)
