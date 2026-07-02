# Step 5 — Use cases

## `record_post_match_use_case`

Seul use case de ce step. La page GET est une lecture pure depuis l'agrégat — pas de use case.

### Fichier

`src/app/match_report/use_cases/record_post_match_use_case.rs`

### Signature

```rust
pub async fn execute(
    cmd: RecordPostMatchCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<RecordPostMatchOutcome, RecordPostMatchError>

pub struct RecordPostMatchCommand {
    pub match_report_id: MatchReportId,
    pub home_gain: MatchGain,
    pub away_gain: MatchGain,
    pub home_fan_mod: FanFactorMod,
    pub away_fan_mod: FanFactorMod,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
    pub recorded_by: CoachId,
}

pub enum RecordPostMatchOutcome {
    Success,
}

pub enum RecordPostMatchError {
    NotFound,
    NotInCompatibleState,
    Internal(String),
}
```

### Orchestration

1. Charger l'agrégat : `repo.find_by_id(&cmd.match_report_id)`
2. Vérifier l'état :
   - `PreMatch(pm)` → première soumission, continuer avec `pm`
   - `ReadyToPublish(rtp)` → re-soumission autorisée, continuer avec `rtp`
   - `Draft` / `Cancelled` → retourner `Err(NotInCompatibleState)`
3. Appeler la méthode domaine `record_post_match(cmd)` → retourne `(updated, PostMatchRecorded)`
4. Persister l'événement : `repo.append_event(event)`
5. Retourner `Ok(Success)`

### Ports

Aucun. Toutes les données viennent de la commande — pas d'appel inter-BC.

### Règles métier

Toute validation métier (contraintes sur les gains, fan mods) est déléguée à la méthode domaine — le use case ne décide pas, il orchestre.
