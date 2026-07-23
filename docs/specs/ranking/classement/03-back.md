# Classement — Phase 3 : Architecture back

## Nouveau BC `ranking` — squelette

Même structure que les BCs existants (`players` pris comme modèle) :

```
src/app/ranking/
├── mod.rs                                    # pub mod context/domain/io/ports/router/routes/use_cases
├── context.rs                                # RankingContext + init_listeners()
├── ports.rs                                  # IRankingRepository (interne) + IRankingCompetitionPort (ACL)
├── router.rs                                 # router() -> Router<AppState>
├── routes.rs                                 # path::* + struct Routes
├── domain/                                   # détaillé en Phase 6
├── io/
│   ├── web/
│   │   └── widgets/
│   │       └── classement_widget.rs          # handler du widget
│   ├── app_events/
│   │   └── match_report_published_listener.rs  # écoute MatchReportAppEvent::MatchReportPublished
│   └── repository/
│       └── ranking_repository.rs             # implémentation Postgres de IRankingRepository
├── use_cases/
│   └── record_match_ranking_use_case.rs      # détaillé en Phase 5
└── templates/widgets/
    └── classement-widget.html                # récupéré de competition-tab-standings.html (adapté)
```

`src/infrastructure/ranking/competition_info_adapter.rs` implémente `IRankingCompetitionPort` côté infrastructure (cf. ports ci-dessous).

## Widget

| Widget | Fichier handler | Fichier template | Route |
|---|---|---|---|
| Classement | `ranking/io/web/widgets/classement_widget.rs` | `ranking/templates/widgets/classement-widget.html` | `GET /app/{space_id}/ranking/{competition_id}/{season_id}/widget` |

## Ports nécessaires

### `IRankingCompetitionPort` (ranking → competitions)

Un seul port, deux besoins (règles de calcul + distinction des 2 états vides — cf. Phase 2) :

```rust
pub struct RankingRulesInfo {
    pub win_points:  u32,
    pub draw_points: u32,
    pub lose_points: u32,
}

pub struct EnrolledTeamInfo {
    pub team_id:   String,
    pub team_name: String,
}

#[async_trait]
pub trait IRankingCompetitionPort: Send + Sync {
    async fn find_ranking_rules(&self, season_id: &str) -> Option<RankingRulesInfo>;
    async fn find_enrolled_teams(&self, season_id: &str) -> Vec<EnrolledTeamInfo>;
}
```

Adapter `infrastructure/ranking/competition_info_adapter.rs` : enveloppe `Arc<dyn ISeasonRepository>` (pour `find_rules` → `RankingRulesInfo`, déjà utilisé par `summary_tab.rs`) et `Arc<dyn ITeamInfoPort>` (le port `competitions → teams` **déjà existant**, réutilisé tel quel — `ranking` ne parle jamais directement à `teams`, uniquement à `competitions`, conformément à ta demande).

`find_enrolled_teams` sert deux besoins à la fois : (a) distinguer les 2 états vides (liste vide = "aucune équipe"), (b) résoudre les noms d'équipe pour l'affichage — le payload `MatchReportPublished` ne contient que des `team_id`, jamais de noms.

### `IRankingRepository` (interne, event-sourcing/projection du BC)

Détaillé en Phase 6/7 (le modèle exact — table d'historique append-only vs autre — sera précisé en Phase 6 domaine). À ce stade, deux opérations identifiées :
- Lire la dernière ligne de classement d'une équipe pour une saison (pour calculer le cumul suivant) — "dernière" au sens **ordre d'enregistrement global**, pas par journée (cf. règle métier ci-dessous)
- Insérer une nouvelle ligne de classement (équipe, saison, `round_id`, `match_report_id`, points de classement du match, cumul, date d'enregistrement)

Pas de déduplication/idempotence nécessaire à l'écriture : chaque event traité insère une nouvelle ligne, jamais de mise à jour d'une ligne existante. La lecture (widget) ne prend toujours que la dernière ligne par équipe.

## Domain service

`use_cases/record_match_ranking_use_case.rs` a besoin de combiner `RankingRulesInfo` (port) avec le payload `MatchReportPublished` (score) pour déterminer victoire/nul/défaite et les points correspondants. Cette transformation (DTO de port → décision domaine) vit dans un domain service colocalisé au use case (pas dans le handler, pas dans le listener) — nommage exact en Phase 5/6.

## Écoute des événements

`ranking::context::init_listeners(app_event_bus, pool, competition_port)` s'abonne à `MatchReportAppEvent::MatchReportPublished` sur l'`app_event_bus` (même pattern que `players::team_created_listener`). Un seul event traité génère **2** lignes de classement (équipe domicile + équipe visiteuse).

## Modifications dans `competitions`

- `competition_detail.rs` : suppression de `mock_standings()` et de la struct `StandingRow`
- `get_tab_standings` : simplifié en simple host (plus de calcul, juste le rendu du wrapper `hx-get` défini en Phase 2)
- `competition-tab-standings.html` : remplacé par le wrapper `hx-get` vers le widget `ranking`
- Ferme la partie "Classement" de la carte `13-mock-data-competition-detail.md`

## Enregistrement du BC (`state.rs` / `main.rs`)

Même schéma que `players` (cf. investigation précédente) :
1. `pub mod ranking;` dans `app/mod.rs`
2. `pub ranking: RankingContext` dans `AppState`
3. Construction de `RankingCompetitionAdapter` avant `RankingContext::new(...)`
4. `ranking::context::init_listeners(...)` appelé **avant** la construction d'`AppState` (comme pour `players`)
5. `.merge(app::ranking::router::router())` sur le router protégé

## Règles métier identifiées

- `ranking` ne parle jamais directement à `teams` — uniquement à `competitions`, qui ré-expose ce dont `ranking` a besoin (règles + équipes inscrites) via son propre port existant vers `teams`
- Un `MatchReportPublished` génère toujours exactement 2 lignes de classement (une par équipe), jamais 0, jamais 1
- Le payload `MatchReportPublished` ne contient pas de noms d'équipe — toujours résolus via `IRankingCompetitionPort`, jamais stockés en dur dans une ligne de classement
- Si `find_ranking_rules` ne retourne rien pour la saison (règles non configurées), le widget affiche une **erreur explicite**, jamais un classement à 0 ou un état vide
- Une équipe peut avoir plusieurs lignes de classement pour une même journée (plusieurs matchs joués le même jour de calendrier, ou — à terme — une correction de rapport de match qui ajoute une nouvelle ligne sans modifier l'ancienne). Le classement affiché ne retient toujours que la **dernière ligne enregistrée par équipe** (ordre d'enregistrement global, pas par journée) — pas de déduplication ni d'idempotence à gérer à l'écriture
