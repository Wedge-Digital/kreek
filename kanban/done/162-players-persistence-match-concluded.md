# BC `players` — Persistance de `MatchConcluded` + lecture des events bruts

**Priorité : haute**
**Dépend de :** `161-players-domain-match-concluded.md`
**Contexte :** `players/io/repository` — event store + nouvelle méthode de lecture

## Objectif

Persister `MatchConcluded` (carte 161), et exposer une méthode pour lire les
events **bruts** d'un joueur (pas seulement l'agrégat hydraté final) —
nécessaire à la reconstruction de l'historique de matchs (carte 165), qui doit
regrouper les events par `match_report_id`.

---

## Conception

### `event_type_name()` / `player_and_team_id()` — `player_repository.rs`

Ajout du variant `MatchConcluded` aux deux matchs exhaustifs.

### `upsert_player_projection()`

```rust
PlayerDomainEvent::MatchConcluded { player_id, .. } => {
    sqlx::query("UPDATE players_proj SET version = version + 1 WHERE player_id = $1")
        .bind(&player_id.0)
        .execute(&mut **tx).await.map_err(RepositoryError::Database)?;
}
```

Pas de nouvelle colonne — `matches_played` n'est pas projeté (même choix de
portée que pour les compteurs de carrière, carte 155 : aucun lecteur de liste
n'en a besoin, disponible via l'agrégat complet).

### Nouvelle méthode — `IPlayerRepository`

```rust
async fn find_events_by_id(&self, player_id: &PlayerId) -> Result<Vec<PlayerDomainEvent>, RepositoryError>;
```

Implémentation : même requête que `find_by_id` (`SELECT payload FROM players_events WHERE player_id = $1 ORDER BY version ASC`), mais retourne les events désérialisés directement, sans passer par `Player::from_events`.

---

## Checklist

- [ ] `event_type_name()` + `player_and_team_id()` étendus
- [ ] `upsert_player_projection()` étendu (no-op fonctionnel, juste `version += 1`)
- [ ] `IPlayerRepository::find_events_by_id()` — trait + implémentation `PgPlayerRepository`
- [ ] Test d'intégration (vraie PgPool) : `find_events_by_id` retourne les events dans l'ordre, `find_by_id` et `find_events_by_id` restent cohérents entre eux
