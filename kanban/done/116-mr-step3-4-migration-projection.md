# BC match_report — Migration SQL + projection repository step3-4

**Priorité : haute**
**Dépend de :** 114
**Contexte :** match_report step3-4-actions — persistance

## Objectif

Migrer la base de données et implémenter les handlers de projection pour les 4 nouveaux events de la feature step3-4.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/07-integration.md`

### Migration SQL (nouveau fichier `migrations/`)

```sql
ALTER TABLE match_report_pre_match
    ADD COLUMN home_temp_players JSONB NOT NULL DEFAULT '[]'::jsonb,
    ADD COLUMN away_temp_players JSONB NOT NULL DEFAULT '[]'::jsonb;

CREATE TABLE match_report_actions (
    action_id           TEXT        PRIMARY KEY,
    match_report_id     TEXT        NOT NULL,
    team_side           TEXT        NOT NULL,
    turn_number         SMALLINT    NOT NULL,
    player_id           TEXT        NOT NULL,
    player_type         TEXT        NOT NULL,
    action_json         JSONB       NOT NULL,
    player_display_name TEXT        NOT NULL,
    recorded_at         TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    is_deleted          BOOLEAN     NOT NULL DEFAULT FALSE
);

CREATE INDEX idx_mr_actions_mr_side
    ON match_report_actions (match_report_id, team_side)
    WHERE NOT is_deleted;
```

### Nouveaux event handlers dans le projection repository

Chaque handler reçoit un `&mut Transaction` et s'exécute dans la même transaction que l'`INSERT` de l'event.

| Event | SQL |
|---|---|
| `TempPlayersInitialized` | `UPDATE match_report_pre_match SET home_temp_players = $1` (ou `away_temp_players`) `WHERE match_report_id = $2` |
| `TempPlayersReset` | `UPDATE match_report_pre_match SET home_temp_players = '[]'::jsonb WHERE match_report_id = $1` |
| `ActionRecorded` | `INSERT INTO match_report_actions …` |
| `ActionDeleted` | `UPDATE match_report_actions SET is_deleted = TRUE WHERE action_id = $1` |

Le choix `home` / `away` se fait en Rust (comparer `team_id` avec `pm.home_team_id`).

### Lecture pour le widget action-log

```sql
SELECT action_id, team_side, turn_number, player_id, player_type,
       action_json, player_display_name, recorded_at
FROM match_report_actions
WHERE match_report_id = $1
  AND team_side = $2
  AND NOT is_deleted
ORDER BY recorded_at;
```

## Checklist

- [ ] Fichier de migration SQL créé et testé (`sqlx migrate run`)
- [ ] Handler projection `TempPlayersInitialized` (en transaction)
- [ ] Handler projection `TempPlayersReset` (en transaction)
- [ ] Handler projection `ActionRecorded` (en transaction)
- [ ] Handler projection `ActionDeleted` (en transaction)
- [ ] Méthode de lecture `find_actions_by_match_and_side(mr_id, team_side)` dans le repository
