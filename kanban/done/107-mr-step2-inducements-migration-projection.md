# BC match_report — Migration SQL + projection repository

**Priorité : haute**
**Dépend de :** 105
**Contexte :** match_report step2-inducements — infrastructure persistance

## Objectif

Ajouter les colonnes de projection pour les TeamValues et les inducements, et implémenter les handlers de projection pour les trois nouveaux events.

## Conception

Cf. `docs/specs/match-report/step2-inducements/07-integration.md`

### Migration SQL

```sql
ALTER TABLE match_report_pre_match
    ADD COLUMN home_team_value  INTEGER,
    ADD COLUMN away_team_value  INTEGER,
    ADD COLUMN home_inducements JSONB,
    ADD COLUMN away_inducements JSONB;
```

- `NULL` = pas encore enregistré
- `'[]'::jsonb` = "Passer" (aucun achat)

### Rehydratation (`domain/match_report_pre_match.rs`)

Deux nouveaux bras dans `rehydrate()` :

```rust
TeamValuesRecorded { home_team_value, away_team_value, .. } => {
    self.home_team_value = Some(home_team_value);
    self.away_team_value = Some(away_team_value);
}
InducementsRecorded { team_id, purchases, .. } => {
    if team_id == self.home_team_id { self.home_inducements = Some(purchases); }
    else { self.away_inducements = Some(purchases); }
}
StarPlayerEngaged { .. } => { /* pas de mutation — état dérivé depuis inducements */ }
```

### Projection repository (`io/repository/match_report_repository.rs`)

Handlers dans `update_projection_in_tx()` — même transaction que le INSERT event :

- `TeamValuesRecorded` → `UPDATE ... SET home_team_value = $1, away_team_value = $2`
- `InducementsRecorded` → `UPDATE ... SET home_inducements = $1` ou `away_inducements = $1` selon `team_id`
- `StarPlayerEngaged` → pas de colonne dédiée, pas de mise à jour projection

### `append_many`

Si `IMatchReportRepository` ne possède pas encore `append_many`, l'ajouter :

```rust
async fn append_many(
    &self,
    tx: &mut PgConnection,
    match_report_id: &MatchReportId,
    events: Vec<MatchReportDomainEvent>,
) -> Result<(), RepositoryError>;
```

## Checklist

- [ ] Migration SQL (`migrations/YYYYMMDDHHMMSS_match_report_pre_match_inducements.sql`)
- [ ] Bras `TeamValuesRecorded` dans `rehydrate()`
- [ ] Bras `InducementsRecorded` dans `rehydrate()`
- [ ] Bras `StarPlayerEngaged` dans `rehydrate()` (no-op)
- [ ] Handler projection `TeamValuesRecorded` dans `update_projection_in_tx()`
- [ ] Handler projection `InducementsRecorded` dans `update_projection_in_tx()`
- [ ] `append_many()` sur le repository (si absent)
- [ ] Test d'intégration : séquence d'events → projection correcte
- [ ] Test d'intégration : rollback → event store ET projection inchangés
