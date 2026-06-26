# Step 2.1 — Coups de pouce — Use cases

---

## Use case 1 : `record_fan_factor_use_case` (modifié)

### Signature

```rust
pub async fn execute(
    cmd: RecordFanFactorCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordFanFactorOutcome, RecordFanFactorError>

pub enum RecordFanFactorOutcome {
    RedirectToInducements { topdog_team_id: String },
    RedirectToStep3,  // aucun inducement disponible pour la compétition
}
```

### Orchestration

1. Charge l'agrégat via `repo.find_by_id()` → doit être `PreMatch`, sinon `NotInPreMatchPhase`
2. Appelle `pm.record_fan_factor(home_roll, away_roll, recorded_by)` → event `FanFactorRecorded`
3. Fetch `team_data.find_team_value(home_id)` + `find_team_value(away_id)` en parallèle
4. Si l'un des deux retourne `None` → `TeamValueUnavailable`
5. Appelle `pm.record_team_values(home_tv, away_tv, recorded_by)` → event `TeamValuesRecorded`
6. Persiste les deux events dans la même transaction via `repo.append()`
7. Fetch `competition_data.find_tier_rules_for_roster(season_id, topdog_roster_id)`
8. Si liste vide (aucun inducement configuré) → retourne `RedirectToStep3`
9. Sinon → retourne `RedirectToInducements { topdog_team_id: pm.topdog_team_id().to_string() }`

### Erreurs

```rust
pub enum RecordFanFactorError {
    NotFound,
    NotInPreMatchPhase,
    TeamValueUnavailable(String),
    Repository(String),
}
```

---

## Use case 2 : `record_inducements_use_case` (nouveau)

### Signature

```rust
pub async fn execute(
    cmd: RecordInducementsCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordInducementsOutcome, RecordInducementsError>

pub enum RecordInducementsOutcome {
    RedirectToInducements { next_team_id: String },  // équipe suivante (underdog)
    RedirectToStep3,                                  // les deux équipes ont terminé
}
```

### Orchestration

1. Charge l'agrégat → doit être `PreMatch` avec `home_team_value` et `away_team_value` présents, sinon `TeamValuesNotRecorded`
2. Fetch `competition_data.find_tier_rules_for_roster(season_id, roster_id_of_buying_team)` → liste des UIDs autorisés
3. Si `None` → `TierRulesUnavailable`
4. Vérifie que chaque UID soumis dans `cmd.purchases` figure dans les listes autorisées → sinon `UnauthorizedInducement(uid)`
5. Fetch `team_data.find_team_treasury(cmd.team_id)` → trésorerie de l'équipe
6. Si `None` → `TreasuryUnavailable`
7. Calcule `budget = pm.inducement_budget_for(&cmd.team_id, treasury)` — `topdog_spending()` est calculé en interne par l'agrégat
8. Appelle `pm.record_inducements(&cmd.team_id, cmd.purchases, budget, &tier_rules.allowed_inducements, opponent_star_uids)` → `Vec<MatchReportDomainEvent>` ou `DomainError`
   - Toujours : `InducementsRecorded { team_id, purchases, recorded_by }`
   - Pour chaque star player acheté : `StarPlayerEngaged { team_id, star_player_uid, recorded_by }`
9. Persiste tous les events dans la même transaction via `repo.append_many()`
10. Si `pm.is_inducements_phase_complete()` → retourne `RedirectToStep3`
11. Sinon → retourne `RedirectToInducements { next_team_id: pm.underdog_team_id().to_string() }`

> **"Passer"** : `cmd.purchases` est vide. Le use case suit le même chemin — le domaine accepte un tableau vide, émet `InducementsRecorded { purchases: [] }`, l'agrégat marque l'équipe comme ayant terminé avec 0 dépensé.

### Erreurs

```rust
pub enum RecordInducementsError {
    NotFound,
    NotInPreMatchPhase,
    TeamValuesNotRecorded,
    TreasuryUnavailable(String),
    TierRulesUnavailable(String),
    UnauthorizedInducement(String),
    Domain(DomainError),
    Repository(String),
}
```

---

## Règles ne relevant pas des use cases

Les décisions suivantes appartiennent exclusivement au domaine (Phase 6) :

- Budget insuffisant → `DomainError::BudgetExceeded`
- Quantité > `maxQty` → `DomainError::MaxQtyExceeded`
- Plus de 2 star players → `DomainError::StarPlayerLimitExceeded`
- Même star player dans les deux équipes → `DomainError::StarPlayerConflict`
- Calcul du budget Underdog → méthode `inducement_budget_for()`
- Détermination TopDog/Underdog → méthode `topdog_team_id()`

Les use cases passent les données nécessaires aux méthodes domaine et propagent les erreurs — ils ne les évaluent pas.
