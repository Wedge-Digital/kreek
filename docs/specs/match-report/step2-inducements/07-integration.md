# Step 2.1 — Coups de pouce — Intégration / Persistance

---

## Migration SQL

Nouvelles colonnes sur la table de projection `match_report_pre_match` :

```sql
ALTER TABLE match_report_pre_match
    ADD COLUMN home_team_value  INTEGER,
    ADD COLUMN away_team_value  INTEGER,
    ADD COLUMN home_inducements JSONB,
    ADD COLUMN away_inducements JSONB;
```

- `home_team_value` / `away_team_value` : `NULL` avant enregistrement du fan factor
- `home_inducements` / `away_inducements` : `NULL` = pas encore / `'[]'::jsonb` = passé sans achat

---

## Projection repository — nouveaux event handlers

Tous les handlers de projection s'exécutent **dans la même transaction** que le `INSERT` de l'event (règle fondamentale CLAUDE.md).

### `TeamValuesRecorded`

```sql
UPDATE match_report_pre_match
SET home_team_value = $1,
    away_team_value = $2
WHERE match_report_id = $3
```

### `InducementsRecorded`

Le champ mis à jour dépend du `team_id` de l'event comparé à `home_team_id` :

```sql
-- si team_id == home_team_id
UPDATE match_report_pre_match
SET home_inducements = $1
WHERE match_report_id = $2

-- si team_id == away_team_id
UPDATE match_report_pre_match
SET away_inducements = $1
WHERE match_report_id = $2
```

La valeur `$1` est le tableau JSON des achats sérialisé depuis `Vec<InducementPurchase>`.

### `StarPlayerEngaged`

Pas de colonne dédiée dans la projection `match_report_pre_match` — la présence des star players est déjà dérivable depuis `home_inducements` / `away_inducements`. L'event est persisté dans l'event store mais **sans mise à jour de projection**. Il sert aux listeners futurs (step 3, post-match).

---

## Rehydratation de l'agrégat

Dans `rehydrate()` de `MatchReportPreMatch`, deux nouveaux bras de `match` :

```rust
MatchReportDomainEvent::TeamValuesRecorded { home_team_value, away_team_value, .. } => {
    self.home_team_value = Some(home_team_value);
    self.away_team_value = Some(away_team_value);
}

MatchReportDomainEvent::InducementsRecorded { team_id, purchases, .. } => {
    if team_id == self.home_team_id {
        self.home_inducements = Some(purchases);
    } else {
        self.away_inducements = Some(purchases);
    }
}

MatchReportDomainEvent::StarPlayerEngaged { .. } => {
    // Pas de mutation d'état — état dérivé depuis home/away_inducements
}
```

---

## Repository — nouvelle méthode `append_many`

`record_inducements_use_case` persiste plusieurs events en une seule transaction. Si la méthode `append_many` n'existe pas encore sur `IMatchReportRepository`, elle doit être ajoutée :

```rust
async fn append_many(
    &self,
    tx: &mut PgConnection,
    match_report_id: &MatchReportId,
    events: Vec<MatchReportDomainEvent>,
) -> Result<(), RepositoryError>;
```

`record_fan_factor_use_case` persiste déjà deux events (`FanFactorRecorded` + `TeamValuesRecorded`) → il utilise également `append_many`.

---

## Infrastructure adapters

### `team_data_adapter.rs` — nouvelles méthodes

```rust
// find_team_value : lit la colonne team_value de la projection teams
async fn find_team_value(&self, team_id: &str) -> Option<u32>

// find_team_treasury : lit la colonne treasury de la projection teams
async fn find_team_treasury(&self, team_id: &str) -> Option<u32>
```

Ces deux méthodes font chacune une requête SQL sur la table de projection du BC Teams. Elles ne traversent pas de port — l'adapter est dans `src/infrastructure/match_report/` et peut accéder directement au `PgPool`.

### `competition_data_adapter.rs` — nouvelle méthode

```rust
async fn find_tier_rules_for_roster(
    &self,
    season_id: &str,
    roster_id: &str,
) -> Option<TierRulesDto>
```

**Assemblage en deux étapes** dans l'adapter :

1. Requête SQL sur BC Competitions : récupère les UIDs autorisés pour ce `(season_id, roster_id)`
2. Lookup in-memory sur BC References (`InMemoryReferenceRepository`) : enrichit chaque UID avec `max_qty` + `unit_cost`

L'adapter est le seul point où ces deux BCs sont croisés — la couche domaine et les use cases ne voient que `TierRulesDto`.

---

## Enregistrement des routes

Dans `io/web/mod.rs` du BC MatchReport, enregistrer les deux nouvelles routes :

```rust
.route(
    "/app/:space_id/match-report/:mr_id/step2/inducements/:team_id",
    get(inducements_controller::get_inducements)
        .post(inducements_controller::post_inducements),
)
```

Dans `routes.rs` du BC MatchReport, ajouter :

```rust
pub fn step2_inducements(
    &self,
    space_id: &str,
    mr_id: &str,
    team_id: &str,
) -> String {
    format!("/app/{space_id}/match-report/{mr_id}/step2/inducements/{team_id}")
}
```

Dans `routes.rs` du BC References, ajouter :

```rust
pub fn inducement_selector(&self) -> &str {
    "/references/inducement-selector"
}
```
