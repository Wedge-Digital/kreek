# Classement — Phase 5 : Use cases

Pas de mutation HTTP dans cette feature (le widget est en lecture seule). Le seul use case est déclenché par le listener d'app event, pas par un handler.

## `record_match_ranking_use_case`

**Déclencheur** : `ranking/io/app_events/match_report_published_listener.rs`, sur réception de `MatchReportAppEvent::MatchReportPublished`.

### Commande

```rust
pub struct RecordMatchRankingCommand {
    pub competition_id:  CompetitionId,   // shared_kernel::common_types
    pub season_id:        SeasonId,        // shared_kernel::common_types
    pub round_id:          RoundId,         // shared_kernel::common_types
    pub match_report_id:   MatchReportId,   // shared_kernel::common_types
    pub home_team_id:      TeamId,          // shared_kernel::team
    pub away_team_id:      TeamId,
    pub home_score:         MatchScore,      // nouveau VO ranking, formalisé Phase 6
    pub away_score:         MatchScore,
    pub published_at:       DateTime<Utc>,
}
```

Construite par le listener à partir du payload `MatchReportPublishedPayload` (parsing des `String` en VOs — logique du listener, pas du use case).

### Orchestration

1. Charger les règles de classement de la saison via `IRankingCompetitionPort::find_ranking_rules(season_id)` → si `None`, retourner `RecordMatchRankingError::RulesNotConfigured` immédiatement (aucune ligne écrite)
2. Charger la dernière ligne de classement de `home_team_id` et `away_team_id` via `IRankingRepository::find_latest_line(season_id, team_id)` (`None` si l'équipe n'a encore aucune ligne — première apparition dans le classement)
3. Appeler la méthode domaine (Phase 6) qui, à partir des 2 lignes précédentes (ou absence) + scores + règles, calcule les 2 nouvelles lignes (résultat V/N/D, points de classement du match, nouveaux cumuls)
4. Persister les 2 nouvelles lignes via `IRankingRepository::insert_line`, **dans une seule transaction** (les deux lignes d'un même match doivent apparaître ensemble ou pas du tout — pas de règle métier qui tolère une écriture partielle)

### Erreurs applicatives

```rust
pub enum RecordMatchRankingError {
    RulesNotConfigured,
    Repository(String),
}
```

`RulesNotConfigured` : le listener logue une erreur (`tracing::error!`) et ignore l'event — même politique que les autres listeners du projet (pas de retry, pas de dead-letter queue).

### Émission d'événements

Aucun événement de domaine émis par ce use case dans cette feature — rien n'écoute "une ligne de classement a été enregistrée" pour l'instant. À réévaluer si une feature future en a besoin (ex. notification, recalcul de départage).

## Règles métier identifiées

- Les 2 lignes (domicile + extérieur) d'un même match sont écrites **atomiquement** — jamais l'une sans l'autre
- Si les règles de classement ne sont pas configurées au moment de la publication du rapport, **aucune ligne n'est créée** pour ce match (pas de valeur par défaut, pas de calcul partiel) — cohérent avec l'état d'erreur du widget (Phase 4)
