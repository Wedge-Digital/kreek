# Step 2.1 — Coups de pouce — Architecture back

## Note architecturale — paramétrage du widget

Le widget `inducement-selector` (BC References) ne prend **pas** `competition_id` en paramètre direct. La résolution tier → UIDs autorisés est une responsabilité du BC MatchReport : le handler appelle `ICompetitionDataPort.find_tier_rules_for_roster()` et passe les listes d'UIDs calculées au widget. Le BC References reste ainsi sans port inter-BC.

---

## Machine d'états — pas de transition à cette étape

La machine d'états actuelle : `Draft → PreMatch → Cancelled`.

La feature step 2.1 **ne crée pas de nouvel état**. Les events `InducementsRecorded` restent sur `PreMatch`. L'agrégat expose `is_inducements_phase_complete()` pour que le handler sache quand rediriger vers step 3. La transition vers l'état suivant sera définie avec la feature step 3.

---

## BC MatchReport — fichiers

### Nouveaux fichiers

| Fichier | Rôle |
|---|---|
| `io/web/inducements_controller.rs` | GET (page) + POST (enregistrement achats) |
| `io/web/templates/inducements.html` | Page hôte : header, budget banner, `hx-get` widget, cart sticky footer |
| `use_cases/record_inducements_use_case.rs` | Orchestration : charge agrégat, fetch tier rules, appelle domaine, persiste |

### Fichiers modifiés

| Fichier | Modification |
|---|---|
| `use_cases/record_fan_factor_use_case.rs` | Commande étendue avec `home_team_value` + `away_team_value` ; appelle `ITeamDataPort.find_team_value()` |
| `domain/match_report_pre_match.rs` | Champs `home_team_value`, `away_team_value`, `home_inducements`, `away_inducements` ; nouvelles méthodes domaine |
| `domain/events.rs` | Nouveaux events : `TeamValuesRecorded`, `InducementsRecorded` |
| `domain/value_objects.rs` | Nouveau value object `TeamValue(u32)` |
| `domain/error.rs` | Nouvelles erreurs : `BudgetExceeded`, `MaxQtyExceeded`, `StarPlayerLimitExceeded`, `StarPlayerConflict`, `NotTopDogPhase` |
| `ports.rs` | Nouvelles méthodes sur `ITeamDataPort` et `ICompetitionDataPort` |
| `io/repository/match_report_repository.rs` | Projection : handlers des nouveaux events |
| `routes.rs` | `step2_inducements(space_id, mr_id, team_id) -> String` |
| `io/web/mod.rs` | Enregistrement du nouveau controller |

### Routes MatchReport

| Méthode | Path | Handler |
|---|---|---|
| GET | `/app/{space_id}/match-report/{mr_id}/step2/inducements/{team_id}` | `get_inducements` |
| POST | `/app/{space_id}/match-report/{mr_id}/step2/inducements/{team_id}` | `post_inducements` |

---

## Couche domaine — `MatchReportPreMatch`

L'agrégat porte toutes les règles métier. Les use cases coordonnent sans décider.

### Nouveaux champs

```rust
pub home_team_value:   Option<TeamValue>,
pub away_team_value:   Option<TeamValue>,
pub home_inducements:  Option<Vec<InducementPurchase>>,  // None = pas encore acheté
pub away_inducements:  Option<Vec<InducementPurchase>>,  // None = pas encore acheté / vide = passé
```

### Nouvelles méthodes

| Méthode | Responsabilité domaine |
|---|---|
| `record_team_values(home_tv, away_tv, recorded_by)` | Stocke les TVs → event `TeamValuesRecorded` |
| `topdog_team_id() -> TeamId` | TV plus haute = TopDog ; égalité = home team |
| `underdog_team_id() -> TeamId` | L'autre équipe |
| `inducement_budget_for(team_id, topdog_spending, treasury) -> u32` | TopDog : trésorerie. Underdog : \|diff TV\| + dépenses TopDog + trésorerie |
| `record_inducements(team_id, purchases, budget, allowed_uids, opponent_stars) -> Result<(Self, Event), DomainError>` | Valide budget, maxQty, ≤ 2 star players, pas de star player adverse ; → event `InducementsRecorded` |
| `is_inducements_phase_complete() -> bool` | True si les deux équipes ont un `Option::Some` dans leurs inducements (même vide) |
| `topdog_spending() -> u32` | Somme des coûts des achats TopDog (0 si passé) |

### Nouveaux value objects

| Type | Description |
|---|---|
| `TeamValue(u32)` | Valeur d'équipe en kPo |
| `InducementPurchase { uid: InducementId, qty: u8 }` | Un achat (uid + quantité achetée) |

### Nouveaux domain events

| Event | Émis quand |
|---|---|
| `TeamValuesRecorded` | Fan factor enregistré → TV des deux équipes capturées |
| `InducementsRecorded` | Équipe valide ses achats (même vide si "Passer") |
| `StarPlayerEngaged` | **Un event par star player recruté** — porte `team_id` + `star_player_uid` |

`StarPlayerEngaged` est émis en plus de `InducementsRecorded`, dans la même transaction. Non émis si aucun star player acheté.

### Nouvelles `DomainError`

`BudgetExceeded`, `MaxQtyExceeded`, `StarPlayerLimitExceeded`, `StarPlayerConflict`, `TeamValuesNotRecorded`, `InducementsAlreadyRecorded`

---

## BC References — fichiers

### Nouveaux fichiers

| Fichier | Rôle |
|---|---|
| `io/web/inducement_selector_controller.rs` | Widget handler (distinct du `inducement_picker_controller` existant) |
| `io/web/templates/widgets/inducement-selector.html` | 3 tabs, cartes inducements + star players, qty controls, star detail expandable |
| `assets/static/css/widgets/inducement-selector.css` | Styles du widget |

### Fichiers modifiés

| Fichier | Modification |
|---|---|
| `routes.rs` | `inducement_selector_base() -> &str` → `/references/inducement-selector` |
| `router.rs` | Enregistrement de la route |

### Route References

| Méthode | Path | Handler |
|---|---|---|
| GET | `/references/inducement-selector` | `inducement_selector_controller` |

**Params du widget** :

| Param | Obligatoire | Description |
|---|---|---|
| `allowed_inducement_uids` | oui | CSV des UIDs autorisés par le tier (calculé par MatchReport) |
| `allowed_star_player_uids` | oui | CSV des UIDs star players autorisés |
| `roster_id` | oui | Filtre spéciaux (`restrictedTo`) et star players (`availableForRosters`) |
| `instance_id` | oui | Isolation multi-instance |
| `selected` | non | Pré-sélection : format `uid:qty,uid:qty` |

---

## Ports étendus (BC MatchReport — `ports.rs`)

### `ITeamDataPort` — nouvelle méthode

```rust
async fn find_team_value(&self, team_id: &str) -> Option<u32>;
```

### `ICompetitionDataPort` — nouvelle méthode

```rust
async fn find_tier_rules_for_roster(
    &self,
    season_id: &str,
    roster_id: &str,
) -> Option<TierRulesDto>;
```

```rust
pub struct TierRulesDto {
    pub allowed_inducement_uids: Vec<String>,
    pub allowed_star_player_uids: Vec<String>,
}
```

---

## Infrastructure — fichiers modifiés

| Fichier | Modification |
|---|---|
| `src/infrastructure/match_report/team_data_adapter.rs` | Implémente `find_team_value` |
| `src/infrastructure/match_report/competition_data_adapter.rs` | Implémente `find_tier_rules_for_roster` |

---

## Flux use cases

### `record_fan_factor_use_case` (modifié)

1. Charge l'agrégat (doit être `PreMatch`)
2. Appelle `pm.record_fan_factor(home_roll, away_roll, recorded_by)` → event `FanFactorRecorded`
3. Fetch `ITeamDataPort.find_team_value()` pour les deux équipes
4. Appelle `pm.record_team_values(home_tv, away_tv, recorded_by)` → event `TeamValuesRecorded`
5. Persiste les deux events (même transaction)
6. Si aucun inducement disponible pour la compétition (vérification via `ICompetitionDataPort`), redirige vers step 3 ; sinon vers step 2.1

### `record_inducements_use_case` (nouveau)

1. Charge l'agrégat (doit être `PreMatch`, `home_team_value` et `away_team_value` doivent être présents)
2. Fetch `ICompetitionDataPort.find_tier_rules_for_roster()` pour valider les UIDs soumis
3. Fetch trésorerie de l'équipe via `ITeamDataPort.find_team_value()` (ou étendre `TeamInfoDto`)
4. Calcule `budget = pm.inducement_budget_for(team_id, pm.topdog_spending(), treasury)`
5. Appelle `pm.record_inducements(team_id, purchases, budget, allowed_uids, opponent_stars)` → event `InducementsRecorded`
6. Persiste l'event
7. Si `pm.is_inducements_phase_complete()` → redirige vers step 3 ; sinon vers step 2.1 de l'équipe suivante

> **"Passer"** : le handler POST reçoit une sélection vide (`purchases: []`). Le use case enregistre `InducementsRecorded { team_id, purchases: [] }`. L'agrégat marque l'équipe comme ayant terminé sa phase inducements avec 0 dépensé.
