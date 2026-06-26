# Step 2 — Avant-match — Architecture back

## Page handler (BC match_report)

La page step 2 est servie par le BC match_report. Le handler charge le match report (état PreMatch), puis rend le template avec les données du match + les URLs vers les endpoints JSON du BC Teams.

| Fichier | Description |
|---|---|
| `src/app/match_report/io/web/pre_match_controller.rs` | Handler GET (page) + POST (soumission fan factor) |
| `src/app/match_report/io/web/templates/pre-match.html` | Template Askama de la page |

### Routes

| Méthode | Path | Handler |
|---|---|---|
| GET | `/app/{space_id}/match-report/{match_report_id}/step2` | `get_pre_match` |
| POST | `/app/{space_id}/match-report/{match_report_id}/step2` | `post_pre_match` |

## Endpoint JSON (BC Teams)

Les données d'équipe (dedicated fans, player count, CTV, treasury, journeyman type) sont fournies par un endpoint JSON du BC Teams.

| Fichier | Description |
|---|---|
| `src/app/teams/io/web/widgets/team_match_context_widget.rs` | Handler JSON |

### Route

| Méthode | Path | Handler |
|---|---|---|
| GET | `/app/{space_id}/team/widgets/match-context/json?team_id=XXX` | `get_team_match_context_json` |

### Données retournées

```json
{
  "team_id": "...",
  "team_name": "...",
  "coach_name": "...",
  "roster_name": "...",
  "dedicated_fans": 3,
  "player_count": 12,
  "ctv": 1120,
  "treasury": 150,
  "journeyman_type": "Lineman"
}
```

### Source des données

Le handler appelle `state.teams.team_repository` pour récupérer les données. Les champs `dedicated_fans`, `player_count`, `ctv`, `treasury`, `journeyman_type` nécessitent probablement une extension du repository ou de la projection teams. À vérifier lors de la phase implémentation.

## Ports

Aucun nouveau port inter-BC nécessaire. La page match_report ne fait pas d'appel inter-BC côté serveur — les données d'équipe sont chargées côté client via `fetch()` vers l'endpoint JSON du BC Teams.

## Use case

Un seul use case pour la soumission :

| Fichier | Description |
|---|---|
| `src/app/match_report/use_cases/record_fan_factor_use_case.rs` | Persiste le fan factor dans le match report |

## Fichiers existants réutilisés

- `match_report_repository.rs` — `append()` pour persister le nouvel événement
- `match_report_pre_match.rs` — agrégat PreMatch, à étendre avec la méthode `record_fan_factor()`
- `events.rs` — ajout de l'événement `FanFactorRecorded`
