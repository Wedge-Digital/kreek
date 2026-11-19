# BC `players` — Persistance des événements d'impact de match

**Priorité : haute**
**Dépend de :** `154-players-domain-match-impact.md`
**Contexte :** `players/io/repository` — event store + projection

## Objectif

Persister les 8 nouveaux `PlayerDomainEvent` (carte 154) dans `players_events`, et
maintenir `players_proj` synchronisé dans la même transaction (règle event-sourcing
CLAUDE.md). Spec :
`docs/specs/player-match-impact/player-report-events/07-integration.md` §3.

---

## Conception

### `event_type_name()` / `player_and_team_id()` — `player_repository.rs`

Matchs exhaustifs à étendre avec les 8 nouveaux variants (ne compile pas tant
qu'ils ne sont pas couverts).

### `upsert_player_projection()` — nouvelles branches

| Event | Effet sur `players_proj` |
|---|---|
| `TouchdownScored`/`PassCompleted`/`InterceptionMade`/`CasualtyInflicted`/`MatchMvpNamed` | `spp = spp + $spp_earned`, `version += 1` |
| `FoulCommitted` | `version += 1` uniquement (rien à afficher en liste pour l'instant) |
| `InjurySustained` | `participation_status = $status` (dérivé du même mapping que `apply()` — Commotion ne change pas le statut, donc pas d'update de colonne dans ce cas, juste `version += 1`), `version += 1` |
| `PlayerAvailabilityRestored` | `participation_status = 'Available'`, `version += 1` |

**Choix de portée assumé** : `career_touchdowns`/`career_passes`/etc., `injuries`,
`stat_adjustments` ne sont **pas** projetés dans cette carte — aucun lecteur
existant ou prévu à court terme (la fiche joueur détaillée n'est pas encore
câblée). Restent disponibles via l'agrégat complet (`find_by_id`/`find_by_team_id`,
déjà existants). Rebuildable depuis l'event store si projeté plus tard.

### Migration SQL

```sql
ALTER TABLE players_proj
  ADD COLUMN participation_status TEXT NOT NULL DEFAULT 'Available';
```

### `find_by_team_id`

Déjà existant (`player_repository.rs:175-209`) — aucune modification nécessaire,
réutilisé tel quel par la carte 158.

---

## Checklist

- [ ] Migration `players_proj.participation_status`
- [ ] `event_type_name()` + `player_and_team_id()` étendus (8 variants)
- [ ] `upsert_player_projection()` étendu selon le tableau ci-dessus
- [ ] Tests d'intégration (vraie PgPool, pas de mock) : chaque nouveau event met à jour la projection comme attendu
