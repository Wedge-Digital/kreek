# Journal d'envois — table et repository

**Spec :** `docs/specs/notifications/envoi/07-integration.md`
**Dépend de :** rien
**Ouvre :** 339

## Objectif

La table qui empêche qu'un coach reçoive deux fois le même email (R3), et qui
garde trace des envois perdus (R1).

## Conception

### L'index porte sur `COALESCE(round_id, '')`

```sql
CREATE UNIQUE INDEX IF NOT EXISTS idx_notification_deliveries_key
    ON competition_notification_deliveries
       (notification_type, season_id, COALESCE(round_id, ''), target_date, coach_id);
```

**PostgreSQL ne considère jamais deux `NULL` comme égaux.** Une contrainte
`UNIQUE` ordinaire laisserait donc passer autant de doublons qu'on veut pour les
**deux notifications de saison**, qui n'ont pas de journée — et seulement pour
celles-là. La protection tomberait exactement là où on la croit acquise, de façon
invisible si les tests ne portent que sur les journées.

### `sent_at` est nullable, et c'est structurant

La ligne est insérée **avant** l'envoi : c'est elle qui réserve le créneau, et
deux crons parallèles se disputent l'index. `sent_at` n'est renseigné qu'après
confirmation.

Une ligne restée à `NULL` est donc un **échec constaté** — la journalisation que
R1 demande. Et elle n'est **jamais rejouée le lendemain** : la reprendre serait
le « cherche ce qui n'est pas parti » que R9 interdit.

### `target_date` est la date visée, pas la date d'envoi

C'est ce seul choix qui fait tenir R2 : une journée décalée change la clé, donc
réarme la notification, sans qu'une ligne de code lui soit consacrée.

## Checklist

- [ ] Migration : table + index unique sur `COALESCE(round_id, '')`
- [ ] `notification_delivery_repository.rs` : `claim` (`INSERT … ON CONFLICT DO
      NOTHING RETURNING 1`) et `confirm` (`sent_at = now()`)
- [ ] `claim_delivery.sql`, `confirm_delivery.sql`
- [ ] Test `#[sqlx::test]` : deux `claim` identiques → **un seul** succès
- [ ] Test `#[sqlx::test]` : deux `claim` identiques **sans `round_id`** → un
      seul succès *(ce test échouerait avec une `UNIQUE` ordinaire)*
- [ ] Test : `claim` puis `confirm` → `sent_at` renseigné
- [ ] `make check-arch`
