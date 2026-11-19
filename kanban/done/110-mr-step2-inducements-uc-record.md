# BC match_report — Use case record_inducements (nouveau)

**Priorité : haute**
**Dépend de :** 106, 108
**Contexte :** match_report step2-inducements — use case

## Objectif

Implémenter le use case d'enregistrement des achats d'inducements pour une équipe.

## Conception

Cf. `docs/specs/match-report/step2-inducements/05-use-cases.md`

### Nouveau fichier `use_cases/record_inducements_use_case.rs`

```rust
pub async fn execute(
    cmd: RecordInducementsCommand,
    repo: &dyn IMatchReportRepository,
    team_data: &dyn ITeamDataPort,
    competition_data: &dyn ICompetitionDataPort,
) -> Result<RecordInducementsOutcome, RecordInducementsError>

pub enum RecordInducementsOutcome {
    RedirectToInducements { next_team_id: String },
    RedirectToStep3,
}
```

### Orchestration

1. Charge agrégat → `PreMatch` + TV présentes, sinon `TeamValuesNotRecorded`
2. Fetch `find_tier_rules_for_roster(season_id, roster_id_of_buying_team)`
3. Vérifie que chaque UID soumis est dans les listes autorisées → `UnauthorizedInducement`
4. Fetch `find_team_treasury(cmd.team_id)`
5. Calcule `budget = pm.inducement_budget_for(&cmd.team_id, treasury)`
6. Fetch `opponent_star_uids` depuis `pm.home_inducements` / `pm.away_inducements`
7. Appelle `pm.record_inducements(...)` → `Vec<MatchReportDomainEvent>`
8. Persiste tous les events via `append_many` (même transaction)
9. Si `pm.is_inducements_phase_complete()` → `RedirectToStep3` ; sinon → `RedirectToInducements { next_team_id: underdog_id }`

> **"Passer"** : `cmd.purchases` vide — même chemin, domaine accepte, émet `InducementsRecorded { purchases: [] }`.

### Commande

```rust
pub struct RecordInducementsCommand {
    pub match_report_id: MatchReportId,
    pub team_id:         TeamId,
    pub purchases:       Vec<InducementPurchaseCmd>,
    pub recorded_by:     CoachId,
}

pub struct InducementPurchaseCmd {
    pub uid: InducementId,
    pub qty: u8,
}
```

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

## Checklist

- [ ] `RecordInducementsCommand` + `InducementPurchaseCmd`
- [ ] `RecordInducementsOutcome` avec les deux variants
- [ ] `RecordInducementsError` exhaustif
- [ ] Orchestration complète (charge agrégat → valide UIDs → budget → domaine → persist → routing)
- [ ] Gestion "Passer" (purchases vide)
- [ ] `append_many` avec tous les events (`InducementsRecorded` + N × `StarPlayerEngaged`)
- [ ] Tests unitaires du use case
