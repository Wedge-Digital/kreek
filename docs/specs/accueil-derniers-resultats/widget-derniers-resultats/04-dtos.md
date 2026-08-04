# Widget "Derniers résultats" — Phase 4 : Contrats de données

---

## Point d'attention — `chrono` vs `time`

`payload.published_at` (app event `MatchReportPublished`) est un
`chrono::DateTime<Utc>`, mais `sqlx` est compilé avec la feature `time`, pas
`chrono` (cf. CLAUDE.md, stack cible). Conversion au point de persistance,
même pattern déjà en place dans `ranking_repository.rs:126-132` :

```rust
// sqlx est compilé avec la feature `time`, pas `chrono` — conversion
// nécessaire au point de persistance.
let published_at = time::OffsetDateTime::from_unix_timestamp_nanos(
    payload.published_at.timestamp_nanos_opt().unwrap_or(0) as i128,
)?;
```

En lecture, `list_latest_results.sql` renvoie donc `published_at` en
`Option<time::OffsetDateTime>` (colonne nullable — les lignes `completed`
antérieures à cette migration n'auront pas de valeur).

---

## DTO de repository (lecture depuis la jointure `competition_match_display_proj` × `competition_seasons` × `competitions`)

```rust
// Émis par : match_day_repository (list_latest_completed_results)
// Consommé par : latest_results_widget (handler)
pub struct LatestResultDto {
    pub pairing_id: String,
    pub season_id: String,           // nécessaire à compute_authorization (team_info_port), pas affiché
    pub competition_id: String,      // nécessaire à compute_authorization (is_competition_admin), pas affiché
    pub competition_name: String,
    pub round_name: String,
    pub home_team_id: String,        // nécessaire à compute_authorization, pas affiché
    pub home_team_name: String,
    pub home_score: Option<i32>,
    pub away_team_id: String,        // nécessaire à compute_authorization, pas affiché
    pub away_team_name: String,
    pub away_score: Option<i32>,
    pub match_report_url: Option<String>,
    pub published_at: Option<time::OffsetDateTime>,
}
```

---

## View Model (sortie vers le template)

```rust
// Émis par : latest_results_widget, via latest_results_view::to_latest_result_vm
// Consommé par : latest-results-widget.html
pub struct LatestResultVm {
    pub competition_name: String,
    pub round_name: String,
    pub home_name: String,
    pub home_score: u32,
    pub home_is_winner: bool,
    pub away_name: String,
    pub away_score: u32,
    pub away_is_winner: bool,
    pub date: String,                // formatée "24 août 2024", helper local à `competitions`
    pub report_url: Option<String>,  // None si match sans rapport, ou utilisateur non autorisé
}
```

```rust
// Émis par : latest_results_widget (handler)
// Consommé par : latest-results-widget.html (racine du fragment)
pub struct LatestResultsWidgetTemplate {
    pub results: Vec<LatestResultVm>,
}
```

`home_is_winner` / `away_is_winner` : `home_score > away_score` /
`away_score > home_score` — les deux `false` en cas d'égalité (règle validée
en Phase 1).

`date` : formatage "24 août 2024" — helper privé local à `competitions`
(mois en français), **pas réutilisé depuis `news_feed.rs:26`** (le BC `news`
n'est pas une source valide d'import pour `competitions`, cf. CLAUDE.md
souveraineté des BCs). Petite duplication assumée, pas d'abstraction
`shared_kernel` pour une fonction de formatage aussi locale.

---

## Émetteurs / consommateurs — récapitulatif

| DTO / VM | Émetteur | Consommateur |
|---|---|---|
| `LatestResultDto` | `match_day_repository::list_latest_completed_results` | `latest_results_widget` (handler) |
| `LatestResultsAuthorization` | `latest_results_view::compute_authorization` | `latest_results_widget` (handler) |
| `LatestResultVm` | `latest_results_view::to_latest_result_vm` | `latest-results-widget.html` |
| `LatestResultsWidgetTemplate` | `latest_results_widget` (handler) | Askama (rendu du fragment) |
