# BC `players` — Persistance : append_batch, display_order, tri

**Priorité : haute**
**Dépend de :** `290-players-roster-edit-domain.md`
**Contexte :** `players` — repository / migration

## Objectif

Câbler la persistance des trois nouveaux événements (projection), ajouter la
colonne portant l'ordre libre, et fournir la méthode de repository
permettant au futur use case de committer tout un batch de joueurs dans une
seule transaction (atomicité actée en Phase 2/5 de la spec).

**Spec de référence :** `docs/specs/player-edition/team-detail/03-back.md`,
`07-integration.md`.

---

## Conception

### Migration

`migrations/<timestamp>_add_display_order_to_players_proj.sql` :

```sql
ALTER TABLE players_proj ADD COLUMN display_order INTEGER;
```

Nullable — un joueur jamais réordonné n'a pas de valeur, retombe sur le tri
par numéro de maillot (cf. plus bas).

### `IPlayerRepository::append_batch` (`players/ports.rs`)

```rust
async fn append_batch(
    &self,
    entries: Vec<(PlayerId, TeamId, PlayerDomainEvent, i32)>,
) -> Result<(), RepositoryError> {
    for (player_id, team_id, event, version) in entries {
        self.append(&player_id, &team_id, &event, version).await?;
    }
    Ok(())
}
```

Implémentation par défaut sur le trait (séquentielle) — **aucune fausse
implémentation de test à modifier**.

### `PgPlayerRepository::append_batch` (`players/io/repository/player_repository.rs`)

Surcharge : une transaction unique enveloppant `insert_player_event` +
`upsert_player_projection` pour **chaque** entrée du batch (mêmes fonctions
déjà existantes, déjà conçues pour prendre un `&mut Transaction` partagé —
cf. `player_repository.rs:428-431` pour le patron single-event à répliquer
en boucle avant le `commit()` final).

### Nouvelles branches (mécaniques)

- `event_type_name()` et `player_and_team_id()` (`player_repository.rs`) :
  3 branches `PlayerRenamed`/`PlayerJerseyChanged`/`PlayerReordered`.
- `upsert_player_projection()` : 3 branches, chacune un
  `UPDATE players_proj SET <colonne> = $2 WHERE player_id = $1`.

### Tri (`players/io/repository/projection_repository.rs:30`)

```sql
ORDER BY display_order NULLS LAST, jersey NULLS LAST, player_id
```

(au lieu de `ORDER BY jersey NULLS LAST, player_id`). Pas de nouvelle colonne
à lire dans le `SELECT` — l'ordre est porté par le tri, pas affiché.

---

## Checklist

- [ ] Migration `add_display_order_to_players_proj`
- [ ] `IPlayerRepository::append_batch` (implémentation par défaut)
- [ ] `PgPlayerRepository::append_batch` (transaction unique)
- [ ] `event_type_name()` : 3 branches
- [ ] `player_and_team_id()` : 3 branches
- [ ] `upsert_player_projection()` : 3 branches
- [ ] `find_by_team_id` (projection) : `ORDER BY` mis à jour
- [ ] Test repository : `append_batch` persiste plusieurs événements de
      joueurs différents dans une seule transaction
- [ ] Test repository : un joueur avec `display_order` défini passe avant un
      joueur sans (`NULLS LAST`)
