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

- [x] Migration : table + index unique sur `COALESCE(round_id, '')`
- [x] `notification_delivery_repository.rs` : `claim` (`INSERT … ON CONFLICT DO
      NOTHING RETURNING 1`) et `confirm` (`sent_at = now()`)
- [x] `claim_delivery.sql`, `confirm_delivery.sql`
- [x] Test `#[sqlx::test]` : deux `claim` identiques → **un seul** succès
- [x] Test `#[sqlx::test]` : deux `claim` identiques **sans `round_id`** → un
      seul succès *(ce test échouerait avec une `UNIQUE` ordinaire)*
- [x] Test : `claim` puis `confirm` → `sent_at` renseigné
- [x] `make check-arch`

## Ce qui a été fait

`domain/notification_delivery.rs` s'ajoute à la checklist : le dépôt ne peut pas
recevoir une clé autrement, et cinq paramètres nus violeraient la règle des
types primitifs. C'est l'emplacement que la spec prévoit. `NotificationType`
porte son `as_str()` **écrit à la main** plutôt que dérivé d'un `Debug` ou d'un
`Serialize` : ce sont des valeurs persistées, et un renommage de variante
réarmerait sinon toutes les notifications déjà envoyées, sans bruit. Un test les
fige.

Pas de trait pour le dépôt : un seul implémenteur, un seul consommateur — le use
case de la 339, dans le même BC. L'abstraction viendra avec un second
implémenteur.

## Le test qui garde `COALESCE`, vérifié plutôt qu'affirmé

La carte annonce qu'un test échouerait avec une `UNIQUE` ordinaire. Vérifié —
mais pas du premier coup.

**Première tentative, fausse.** Remplacer le seul index par une forme ordinaire
fait tomber **les quatre** tests : `ON CONFLICT` ne correspond alors plus à aucun
index, et PostgreSQL refuse chaque insertion. Le contrôle ne montrait donc rien
de ce qu'il prétendait montrer.

**Seconde, juste.** En passant l'index **et** la clause `ON CONFLICT` en forme
ordinaire — ce qu'un développeur aurait réellement écrit — **seul** le test sans
journée tombe :

```
…sans_journee_n_en_accordent_qu_une   FAILED
…n_en_accordent_qu_une                ok
…une_journee_differente…              ok
…confirmer_renseigne_la_date_d_envoi  ok
```

C'est exactement le défaut que la carte décrit : invisible si l'on ne teste que
les journées, et portant sur deux des quatre notifications.
