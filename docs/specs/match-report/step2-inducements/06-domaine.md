# Step 2.1 — Coups de pouce — Domaine

## Règles métier — récapitulatif exhaustif validé

| # | Règle |
|---|---|
| R1 | TeamValue plus haute = TopDog. Égalité = home team = TopDog |
| R2 | TopDog achète en premier. Budget TopDog = trésorerie TopDog (hard cap) |
| R3 | Underdog budget = \|TV_home − TV_away\| + dépenses TopDog + min(trésorerie Underdog, 50 000) |
| R4 | Si trésorerie Underdog < 50 000 → on engage la totalité (pas de pénalité) |
| R5 | Budget dépassé → `BudgetExceeded` (front bloqué + validation back) |
| R6 | Quantité achetée > `maxQty` d'un inducement → `MaxQtyExceeded` |
| R7 | Max 2 star players par équipe → `StarPlayerLimitExceeded` |
| R8 | Même star player dans les deux équipes → `StarPlayerConflict` |
| R9 | "Passer" = `purchases: []` → `InducementsRecorded` vide — dépenses = 0, budget Underdog non impacté |
| R10 | Phase complète quand les deux équipes ont un `InducementsRecorded` (même vide) |
| R11 | La trésorerie n'est pas déduite à l'achat — elle sera recalculée au bilan post-match (step 5) |
| R12 | Le plafond de 50 000 trésorerie Underdog est une constante fixe (non configurable) |
| R13 | TopDog trésorerie = 0 → aucun achat possible (budget = 0) |

---

## Addendum Phase 4 — extension de `TierRulesDto`

Pour que le domaine puisse valider `maxQty` et calculer le coût total, `TierRulesDto` doit inclure les specs complètes (pas seulement les UIDs). L'adapter infrastructure assemble ces données depuis Competitions BC (UIDs autorisés) + References BC (détails per UID).

```rust
pub struct TierRulesDto {
    pub allowed_inducements:   Vec<InducementSpecDto>,
    pub allowed_star_players:  Vec<InducementSpecDto>,
}

pub struct InducementSpecDto {
    pub uid:       String,
    pub max_qty:   u8,
    pub unit_cost: u32,
}
```

---

## Agrégat `MatchReportPreMatch` — nouveaux champs

```rust
pub home_team_value:  Option<TeamValue>,
pub away_team_value:  Option<TeamValue>,
pub home_inducements: Option<Vec<InducementPurchase>>,  // None = pas encore / Some([]) = passé
pub away_inducements: Option<Vec<InducementPurchase>>,
```

---

## Méthodes domaine

### `record_team_values`

```rust
pub fn record_team_values(
    &self,
    home_tv: TeamValue,
    away_tv: TeamValue,
    recorded_by: CoachId,
) -> (Self, MatchReportDomainEvent)
```

Stocke les deux TV. Détermine implicitement TopDog/Underdog (calculé à la demande via `topdog_team_id()`). Émet `TeamValuesRecorded`.

---

### `topdog_team_id` / `underdog_team_id`

```rust
pub fn topdog_team_id(&self) -> &TeamId
pub fn underdog_team_id(&self) -> &TeamId
```

R1 : TV plus haute = TopDog. Égalité = home team.  
Paniquent si appelées avant que les TV soient enregistrées (ne devrait pas arriver — le use case vérifie).

---

### `topdog_spending`

```rust
pub fn topdog_spending(&self) -> u32
```

Somme des `qty × unit_cost` des achats TopDog. Retourne 0 si `None` (pas encore acheté) ou `Some([])` (passé).

---

### `inducement_budget_for`

```rust
pub fn inducement_budget_for(
    &self,
    team_id: &TeamId,
    treasury: u32,
) -> u32
```

- Si `team_id == topdog_team_id()` : retourne `treasury`
- Sinon (Underdog) :
  ```
  let tv_diff = home_tv.abs_diff(away_tv);
  let treasury_contribution = treasury.min(50_000);
  tv_diff + self.topdog_spending() + treasury_contribution
  ```

---

### `record_inducements`

```rust
pub fn record_inducements(
    &self,
    team_id: &TeamId,
    purchases: Vec<InducementPurchaseCmd>,
    budget: u32,
    allowed_specs: &[InducementSpecDto],
    opponent_star_uids: &[InducementId],
) -> Result<(Self, MatchReportDomainEvent), DomainError>
```

**Validations (dans cet ordre)** :

1. `maxQty` : pour chaque achat, `qty ≤ allowed_specs[uid].max_qty` → `MaxQtyExceeded { uid, qty, max_qty }`
2. Star players ≤ 2 : nombre d'UIDs star dans `purchases` ≤ 2 → `StarPlayerLimitExceeded`
3. Conflit star : intersection entre star UIDs achetés et `opponent_star_uids` → `StarPlayerConflict { uid }`
4. Budget : `total_cost ≤ budget` (calculé comme Σ `qty × unit_cost`) → `BudgetExceeded { spent, budget }`

Si toutes les validations passent, retourne `Vec<MatchReportDomainEvent>` :
- `InducementsRecorded { team_id, purchases, recorded_by }` — toujours présent
- Pour chaque star player dans `purchases` : `StarPlayerEngaged { team_id, star_player_uid, recorded_by }`

Signature de retour mise à jour :

```rust
pub fn record_inducements(
    &self,
    team_id: &TeamId,
    purchases: Vec<InducementPurchaseCmd>,
    budget: u32,
    allowed_specs: &[InducementSpecDto],
    opponent_star_uids: &[InducementId],
) -> Result<(Self, Vec<MatchReportDomainEvent>), DomainError>
```

---

### `is_inducements_phase_complete`

```rust
pub fn is_inducements_phase_complete(&self) -> bool
```

Retourne `true` si `home_inducements.is_some() && away_inducements.is_some()`.

---

## Value objects nouveaux

### `TeamValue`

```rust
#[nutype(
    validate(greater_or_equal = 0),
    derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Display)
)]
pub struct TeamValue(u32);
```

Dérive `PartialOrd + Ord` pour la comparaison TopDog/Underdog.

### `InducementPurchase`

```rust
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InducementPurchase {
    pub uid:       InducementId,
    pub qty:       u8,
    pub unit_cost: u32,
}
```

Stocké dans l'agrégat et dans l'event — `unit_cost` est conservé pour `topdog_spending()` sans recalcul.

---

## Domain events nouveaux

```rust
TeamValuesRecorded {
    home_team_value: TeamValue,
    away_team_value: TeamValue,
    recorded_by:     CoachId,
},

InducementsRecorded {
    team_id:     TeamId,
    purchases:   Vec<InducementPurchase>,  // vide si "Passer"
    recorded_by: CoachId,
},

StarPlayerEngaged {
    team_id:         TeamId,
    star_player_uid: InducementId,
    recorded_by:     CoachId,
},
```

`StarPlayerEngaged` est émis **une fois par star player recruté**, en plus de `InducementsRecorded`, dans la même transaction. Non émis si aucun star player acheté.

---

## Erreurs domaine nouvelles

```rust
// dans DomainError
BudgetExceeded { spent: u32, budget: u32 },
MaxQtyExceeded { uid: String, qty: u8, max_qty: u8 },
StarPlayerLimitExceeded,
StarPlayerConflict { uid: String },
TeamValuesNotRecorded,
InducementsAlreadyRecorded,
```

---

## Tests unitaires prévus

| Test | Règle couverte |
|---|---|
| `topdog_is_home_when_tv_equal` | R1 |
| `topdog_is_away_when_away_higher_tv` | R1 |
| `topdog_budget_equals_treasury` | R2 |
| `underdog_budget_includes_tv_diff_and_topdog_spending` | R3 |
| `underdog_budget_caps_treasury_at_50k` | R3, R12 |
| `underdog_budget_uses_full_treasury_when_below_50k` | R4 |
| `topdog_spending_zero_when_passed` | R9 |
| `topdog_spending_zero_before_purchase` | R9 |
| `record_inducements_fails_on_budget_exceeded` | R5 |
| `record_inducements_fails_on_max_qty_exceeded` | R6 |
| `record_inducements_fails_when_star_player_limit_exceeded` | R7 |
| `record_inducements_fails_on_star_player_conflict` | R8 |
| `record_inducements_with_empty_purchases_succeeds` | R9 |
| `record_inducements_emits_star_player_engaged_per_star` | — |
| `record_inducements_no_star_player_engaged_when_none_hired` | — |
| `is_inducements_phase_complete_when_both_recorded` | R10 |
| `is_inducements_phase_not_complete_when_only_one_recorded` | R10 |
| `record_inducements_fails_when_already_recorded` | — |
