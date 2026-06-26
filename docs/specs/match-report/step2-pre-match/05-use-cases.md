# Step 2 — Avant-match — Use cases

## Use case : `record_fan_factor`

### Signature

```rust
pub async fn execute(
    cmd: RecordFanFactorCommand,
    repo: &dyn IMatchReportRepository,
) -> Result<MatchReportId, RecordFanFactorError>
```

### Commande

```rust
pub struct RecordFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub home_fan_roll: D3Roll,
    pub away_fan_roll: D3Roll,
    pub recorded_by: CoachId,
}
```

### Orchestration

1. Charger l'agrégat depuis le repository (`find_by_id`)
2. Vérifier que l'état est `PreMatch` (sinon erreur `NotInPreMatchPhase`)
3. Appeler `pre_match.record_fan_factor(home_fan_roll, away_fan_roll, recorded_by)` sur l'agrégat
4. Persister l'événement `FanFactorRecorded` via `repo.append()`
5. Retourner l'ID du match report

### Erreurs

```rust
#[derive(Debug)]
pub enum RecordFanFactorError {
    NotFound,
    NotInPreMatchPhase,
    Repository(String),
}
```

### Notes

- Pas d'émission d'app event — le fan factor est interne au match report, aucun autre BC n'en a besoin.
- Pas de vérification de permissions dans le use case — c'est une responsabilité du handler (vérifier que l'utilisateur est coach d'une des deux équipes ou admin).
- Le use case est idempotent : si le fan factor a déjà été enregistré, l'agrégat peut soit refuser (erreur), soit écraser. À trancher en phase 6 (domaine).
