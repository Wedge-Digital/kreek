# BC `competitions` — colonne `published_at` sur la projection résultats

**Priorité : haute**
**Dépend de :** —
**Contexte :** `migrations/`, `competitions/io/app_events/match_report_published_listener.rs`

## Objectif

Ajouter la date réelle de publication à `competition_match_display_proj`,
seule donnée manquante pour trier des résultats venant de plusieurs
compétitions/saisons par ordre chronologique. Spec complète :
`docs/specs/accueil-derniers-resultats/widget-derniers-resultats/07-integration.md`.

---

## Conception

### Migration

```sql
ALTER TABLE competition_match_display_proj ADD COLUMN published_at TIMESTAMPTZ;
```

Pas de backfill : les lignes `completed` déjà en base restent à `NULL`
(triées en dernier par la future requête, `NULLS LAST`).

### `match_report_published_listener::update_projection`

`payload.published_at` est un `chrono::DateTime<Utc>` ; `sqlx` est compilé
avec la feature `time`, pas `chrono` — conversion au point de persistance,
même pattern que `ranking_repository.rs:126-132` :

```rust
// sqlx est compilé avec la feature `time`, pas `chrono` — conversion
// nécessaire au point de persistance.
let published_at = time::OffsetDateTime::from_unix_timestamp_nanos(
    payload.published_at.timestamp_nanos_opt().unwrap_or(0) as i128,
)?;
```

```sql
UPDATE competition_match_display_proj
SET match_status = 'completed',
    home_score = $2, away_score = $3,
    home_casualties = $4, away_casualties = $5,
    match_report_url = $6,
    published_at = $7
WHERE pairing_id = $1
```

## Checklist

- [ ] Migration `ALTER TABLE ... ADD COLUMN published_at TIMESTAMPTZ`
- [ ] `update_projection` convertit `payload.published_at` en `time::OffsetDateTime` et l'ajoute à l'UPDATE
- [ ] Test existant du listener toujours vert (ou étendu pour vérifier `published_at`)
