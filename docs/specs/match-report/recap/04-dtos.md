# Récap — Phase 4 : Contrats de données ✅

Les 3 points ouverts de la première passe sont tranchés (cf. « Décisions » en fin de fichier).

## DTO d'entrée

Aucun body/form. `recap_controller.rs` reçoit `space_id`, `match_report_id` via le path uniquement (comme les autres controllers `match_report`) :

```rust
pub async fn get_recap(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse { ... }

pub async fn post_publish(
    auth_session: AuthSession,
    Path((space_id, match_report_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse { ... }
```

`post_publish` ne prend aucune donnée de formulaire — la publication ne fait que transitionner l'état existant, `recorded_by` vient de `auth_session.user.id` (comme `post_inducements` etc.).

## Template

```rust
#[derive(Template)]
#[template(path = "recap.html")]
pub struct RecapTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub match_report_id: String,
    pub is_published: bool,           // discrimine les 2 jeux de CTA (cf. 02-front.md)
    pub round_context: Option<RoundContextVm>,   // dégradation gracieuse si absent
    pub submitted_by: Option<String>,  // nom du coach ayant créé le rapport — dégradation gracieuse si résolution impossible
    pub home_banner: TeamBannerVm,
    pub away_banner: TeamBannerVm,
    pub result: MatchResultVm,
    pub gains_fan: GainsFanVm,
    pub timeline: Vec<TimelineEventVm>,
    pub mvps: Vec<MvpRowVm>,
    pub injuries: Vec<InjuryRowVm>,
    pub performances: Vec<PerformanceRowVm>,
    pub publish_url: String,
    pub back_to_step5_url: String,    // état ReadyToPublish
    pub competition_url: String,      // état Published
    pub home_team_detail_url: String, // état Published — "Voir fiche [équipe home]"
}
```

## View Models — pur domaine (`view_models.rs`, `from_domain()` / `all_from_domain()`)

Construits uniquement depuis `MatchReportReadyToPublish` / `MatchReportPublished` (état local), sans appel de port — les actions portent déjà `player_display_name`/`player_position` dénormalisés.

```rust
pub struct MatchResultVm {
    pub home_score: u8,   // via compute_score() (existe déjà sur PreMatch, à porter sur ReadyToPublish/Published — phase 6)
    pub away_score: u8,
    pub summary_title: Option<String>,
    pub summary_body: Option<String>,
}

pub struct GainsFanVm {
    pub home_gain_kpo: u32,   // MatchGain
    pub away_gain_kpo: u32,
    pub home_fan_mod: i8,     // FanFactorMod — delta uniquement, pas d'avant/après (supprimé phase 1)
    pub away_fan_mod: i8,
}

pub struct TimelineEventVm {
    pub turn: u8,
    pub side: String,             // "home" | "away"
    pub player_display_name: String,
    pub player_position: String,
    pub action_label: String,     // dérivé de MatchActionType (icône + libellé)
    pub injury_label: Option<String>,  // Some(...) seulement pour Blesse{injury} — jamais pour Sortie (cf. décision phase 1 : Sortie n'est pas un KO trackable)
}

pub struct MvpRowVm {
    pub side: String,
    pub player_display_name: String,
    pub player_position: String,
    pub team_name: String,        // dupliqué depuis TeamBannerVm correspondant, pour affichage autonome de la carte sidebar
}

pub struct InjuryRowVm {
    pub side: String,
    pub player_display_name: String,
    pub player_position: String,
    pub turn: u8,
    pub injury_label: String,     // Commotion | Amoché | Blessure Sérieuse | Séquelle | Mort — jamais Sortie/KO
}
```

## View Models — dépendant d'un port (`builders.rs`)

```rust
pub struct TeamBannerVm {
    pub team_name: String,        // ITeamDataPort::find_team_info
    pub team_initials: String,
    pub coach_name: String,
    pub logo_url: Option<String>,
    pub result_badge: String,     // "Victoire" | "Défaite" | "Égalité" — calculé depuis MatchResultVm (domaine) + composé ici
}

pub struct RoundContextVm {
    pub competition_name: String, // ICompetitionDataPort::find_round_context
    pub season_name: String,
    pub round_name: String,
}

pub struct PerformanceRowVm {
    pub side: String,
    pub jersey_or_label: String,  // player_display_name déjà connu localement, mais regroupé ici car dépend de l'ordre/valeurs SppMatchResult
    pub player_position: String,
    pub spp_earned: u8,           // ISppCalculatorPort::calculate_match_spp
}
```

## DTOs de port

### `ICompetitionDataPort` (étendu — `match_report/ports.rs`)

```rust
#[async_trait]
pub trait ICompetitionDataPort: Send + Sync {
    // ... méthodes existantes ...
    async fn find_round_context(&self, season_id: &str, round_id: &str) -> Option<RoundContextDto>;
}

#[derive(Debug, Clone)]
pub struct RoundContextDto {
    pub competition_name: String,
    pub season_name: String,
    pub round_name: String,       // Round.name existant côté BC competitions
}
```

### `ISppCalculatorPort` (nouveau — `match_report/ports.rs`)

DTOs définis côté `match_report` (BC consommateur) — réutilisent `ActionPlayer` du domaine `match_report`, ce qui est licite puisque le port appartient à `match_report`.

```rust
#[async_trait]
pub trait ISppCalculatorPort: Send + Sync {
    async fn calculate_match_spp(
        &self,
        home_actions: &[MatchAction],
        away_actions: &[MatchAction],
        home_roster_id: &str,
        away_roster_id: &str,
    ) -> SppMatchResult;
}

pub struct SppMatchResult {
    pub home: Vec<PlayerSppDto>,
    pub away: Vec<PlayerSppDto>,
}

pub struct PlayerSppDto {
    pub action_player: ActionPlayer,  // Regular(PlayerId) | Temp(TempPlayerId) — clé de corrélation retour vers MatchAction
    pub spp: u8,
}
```

L'adapter `spp_calculator_adapter.rs` (infra) traduit les `MatchAction` en entrées opaques pour le mini BC `spp_calculator` (qui ne connaît jamais `ActionPlayer`/`MatchAction`, types du domaine `match_report`), puis retraduit le résultat en `SppMatchResult`. Détail de cette traduction et du contrat interne de `spp_calculator` : phases 5-6.

### `ICoachDataPort` (nouveau — `match_report/ports.rs`)

Résout `created_by: CoachId` en nom d'affichage pour la byline « Soumis par {coach} ». Réutilise le cache dénormalisé déjà exposé par le BC `spaces` (`ISpaceUserCacheRepository::find_user_by_id`, table `spaces__user_cache`) — capable de résoudre n'importe quel coach_id, pas seulement celui de la session courante.

```rust
#[async_trait]
pub trait ICoachDataPort: Send + Sync {
    async fn find_coach_name(&self, coach_id: &str) -> Option<String>;
}
```

Adapter `src/infrastructure/match_report/coach_data_adapter.rs` — implémente `ICoachDataPort` en appelant `ISpaceUserCacheRepository::find_user_by_id` (BC `spaces`) et projette `User.coach_name`. Même pattern que les 4 adapters existants (cf. 03-back.md).

### `IRosterSppPort` (nouveau, réduit — `spp_calculator/ports.rs`)

```rust
#[async_trait]
pub trait IRosterSppPort: Send + Sync {
    async fn find_special_rules(&self, roster_id: &str) -> Vec<String>;
}
```

Pas de nouveau DTO — réutilise la donnée déjà exposée par `IReferenceRepository::find_team_by_uid(...).special_rules`.

## Interfaces d'utilisation (émetteur → consommateur)

| DTO / VM | Émis par | Consommé par |
|---|---|---|
| `RoundContextDto` | `infrastructure/match_report/competition_data_adapter.rs` (nouvelle méthode) | `builders.rs::build_round_context_vm` |
| `SppMatchResult` / `PlayerSppDto` | `infrastructure/match_report/spp_calculator_adapter.rs` | `builders.rs::build_performance_rows` |
| `TeamInfoDto` (existant) | `infrastructure/match_report/ref_team_data_adapter.rs` (existant) | `builders.rs::build_team_banner` |
| Nom de coach (`Option<String>`) | `infrastructure/match_report/coach_data_adapter.rs` (nouveau) | `builders.rs::build_submitted_by` |
| `MatchResultVm`, `GainsFanVm`, `TimelineEventVm`, `MvpRowVm`, `InjuryRowVm` | `view_models.rs` (`from_domain`/`all_from_domain`) | `recap_controller::get_recap` → `RecapTemplate` |
| `TeamBannerVm`, `RoundContextVm`, `PerformanceRowVm` | `builders.rs` | `recap_controller::get_recap` → `RecapTemplate` |
| `RecapTemplate` | `recap_controller::get_recap` | Askama (`recap.html`) |

## Règles métier identifiées à cette étape

- `injury_label` sur `TimelineEventVm`/`InjuryRowVm` n'est jamais renseigné pour `MatchActionType::Sortie` — seul `Blesse { injury }` produit un libellé de blessure (rappel de la règle phase 1 : Sortie n'est pas un KO trackable).
- `result_badge` (Victoire/Défaite/Égalité) est dérivé de `MatchResultVm.home_score`/`away_score`, jamais stocké — pas de nouvel état domaine.
- `GainsFanVm` n'expose que le delta fan_mod (pas de avant/après) — cohérent avec la suppression validée en phase 1.

## Décisions actées

1. **Byline « Soumis par {coach} »** — conservée. Résolue via nouveau port `ICoachDataPort` (`match_report → spaces`, réutilise le cache `spaces__user_cache` déjà en place). `RecapTemplate.submitted_by: Option<String>`, dégradation gracieuse (`None`) si la résolution échoue.
2. **Chronologie = liste complète des actions** — confirmé. `TimelineEventVm::all_from_domain()` prend toutes les actions (`home_actions` + `away_actions` triées par tour), pas de sélection curatée de « moments clés ».
3. **MVP affiché 2 fois** (sidebar dédiée + fin de timeline) — confirmé. Les deux affichages, alimentés par la même liste `MvpRowVm`.
