# Widget "Derniers résultats" — Phase 3 : Architecture back

## BC : competitions

---

## Constat — pourquoi pas une simple extension de `list_resultats.sql`

`list_resultats.sql` est scopé par `season_id` : il ne permet pas de lister
"tous les derniers résultats d'un espace, toutes compétitions confondues".
`competition_match_display_proj` n'a ni `space_id`, ni nom de compétition, ni
date réelle de publication — indispensables pour trier chronologiquement des
matchs venant de compétitions différentes.

Plutôt que de dénormaliser `space_id` et le nom de compétition dans la
projection (ce qui obligerait à toucher les 3 points de création de pairing —
`generate_all_pairings.rs`, `add_match_use_case.rs`, `generate_pairings.rs`),
la requête de lecture **joint** vers `competition_seasons` et `competitions` :
deux tables du **même BC**, donc pas une violation de souveraineté (juste une
jointure intra-schéma). Seule colonne réellement manquante : `published_at`,
alimentée par un seul point de code déjà en possession de la donnée
(`match_report_published_listener.rs`, `payload.published_at`).

---

## Fichiers à créer

### Migration

`migrations/<timestamp>_add_published_at_to_competition_match_display_proj.sql`
```sql
ALTER TABLE competition_match_display_proj ADD COLUMN published_at TIMESTAMPTZ;
```

### Vue (autorisation + VMs)

| Fichier | Rôle |
|---|---|
| `io/web/latest_results_view.rs` | Miroir de `resultats_view.rs` : `LatestResultsAuthorization`, `compute_authorization`, `to_latest_result_vm` |

### Widget (handler + template)

| Fichier | Rôle |
|---|---|
| `io/web/widgets/latest_results_widget.rs` | Handler GET du widget |
| `io/web/templates/widgets/latest-results-widget.html` | Fragment rendu (états chargé/vide, cf. `02-front.md`) |

### SQL

| Fichier | Rôle |
|---|---|
| `io/repository/sql/match_days/list_latest_results.sql` | Requête jointe décrite ci-dessous |

```sql
SELECT cmdp.pairing_id, cmdp.season_id, c.id AS competition_id, c.name AS competition_name,
       cmdp.round_name, cmdp.home_team_id, cmdp.home_team_name, cmdp.home_score,
       cmdp.away_team_id, cmdp.away_team_name, cmdp.away_score,
       cmdp.match_report_url, cmdp.published_at
FROM competition_match_display_proj cmdp
JOIN competition_seasons cs ON cs.id = cmdp.season_id
JOIN competitions c ON c.id = cs.competition_id
WHERE c.space_id = $1 AND cmdp.match_status = 'completed'
ORDER BY cmdp.published_at DESC NULLS LAST
LIMIT $2;
```

---

## Fichiers à modifier

### `domain/match_day_repository_port.rs`

- Nouveau DTO `LatestResultDto` (lecture, primitives assumées — cf. `04-dtos.md`)
- Nouvelle méthode sur `IMatchDayRepository` :
  ```rust
  async fn list_latest_completed_results(&self, space_id: &str, limit: i64)
      -> Result<Vec<LatestResultDto>, MatchDayRepositoryError>;
  ```

### `io/repository/match_day_repository.rs`

- Implémentation de `list_latest_completed_results`, exécute `list_latest_results.sql`

### `io/app_events/match_report_published_listener.rs`

- L'UPDATE existant (fonction autour de la ligne 249) ajoute `published_at = $n`
  depuis `payload.published_at`

### `routes.rs` / `router.rs`

- `COMPETITION_LATEST_RESULTS_WIDGET = "/app/{space_id}/competitions/widget/latest-results"`
- Enregistrement dans `router.rs`, fonction `latest_results_widget(&self, sid: &str) -> String` dans l'`impl Routes`

### `src/app/news/io/web/news_feed.rs` + `templates/news-feed.html`

- Retrait du bloc statique (`news-feed.html:137-202`), remplacé par le conteneur
  `hx-get` défini en Phase 2, via `app_routes.competitions.latest_results_widget(space_id)`

---

## Ports — aucun nouveau port nécessaire

L'autorisation réutilise les ports **déjà déclarés** dans `competitions` :
- `space_member_port: Arc<dyn ICompetitionSpaceMemberPort>` (`context.rs:29`) —
  jusqu'ici déclaré mais jamais appelé (cf. carte 277 : `resultats_view.rs`
  contourne ce port en accédant à `state.spaces` directement — le nouveau code
  ne reproduit pas ce contournement)
- `team_info_port: Arc<dyn ITeamInfoPort>` — déjà utilisé par `resultats_view.rs`

`competition_repository.find_base_info(competition_id)` reste un accès
intra-BC (repository propre à `competitions`), pas un port.
