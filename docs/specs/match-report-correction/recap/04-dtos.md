# Phase 4 — Contrats de données · page Recap

## Table des interfaces

| Type | Émis par | Consommé par |
|---|---|---|
| `UnpublishMatchReportCommand` | `post_unpublish` (io/web) | `unpublish_match_report_use_case` |
| `CorrectionEligibility` / `CorrectionBlocker` | `correction_eligibility_service` (use_cases) | `MatchReportPublished::unpublish()` (domaine) · `build_correction_zone` (io/web) |
| `DomainError::CorrectionNotAllowed` | `MatchReportPublished::unpublish()` | `unpublish_match_report_use_case` |
| `UnpublishMatchReportError` | `unpublish_match_report_use_case` | `post_unpublish` (io/web) |
| `MatchReportDomainEvent::MatchReportUnpublished` | `MatchReportPublished::unpublish()` | event store · `rehydrate()` · `app_event_publisher` |
| `MatchReportUnpublishedPayload` | `app_event_publisher` (io) | listeners `competitions`, `ranking`, `teams` |
| `TeamMatchImpactReverted` | `app_event_publisher` (io) | `player_match_impact_listener` (players) |
| `CorrectionZoneVm` | `build_correction_zone` (io/web/builders) | `recap.html` |
| `is_team_in_player_improvement` | `ref_team_data_adapter` (infrastructure) | `correction_eligibility_service` |
| `has_spent_spp_since_match` | `player_data_adapter` (infrastructure) | `correction_eligibility_service` |

## A. Entrée HTTP

**Aucun DTO d'entrée.** La route ne porte que des paramètres de chemin, comme
`post_publish` :

```rust
Path((space_id, match_report_id)): Path<(String, String)>
```

## B. Commande applicative

```rust
// use_cases/unpublish_match_report_use_case.rs
pub struct UnpublishMatchReportCommand {
    pub match_report_id: MatchReportId,
    pub unpublished_by:  CoachId,
}
```

Value objects, jamais de primitives nues (cf. CLAUDE.md, principe CQRS).
Symétrique de `PublishMatchReportCommand`.

## C. Value objects domaine

```rust
// domain/value_objects.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrectionEligibility {
    Eligible,
    Blocked(CorrectionBlocker),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CorrectionBlocker {
    /// Un joueur de ce camp a dépensé des SPP depuis le match (règle 2).
    SppAlreadySpent { side: TeamSide },
    /// Ce camp a quitté la phase `PlayerImprovement` (règle 1).
    PhaseAdvanced { side: TeamSide },
    /// Un port du garde-fou n'a pas pu répondre — on échoue fermé (règle 12).
    /// Sans `side` : l'échec ne désigne aucun camp.
    EligibilityUnknown,
}
```

> Le variant `EligibilityUnknown` est un **amendement issu de la phase 5**, où la
> gestion d'échec des ports a été tranchée. Voir `05-use-cases.md`.

### Le blocker porte un `TeamSide`, pas un nom d'équipe

La règle 3 exige que le message **nomme l'équipe** qui bloque. Le nom ne descend
pourtant pas dans le domaine :

- `TeamSide` existe déjà dans `match_report/domain/value_objects.rs` — aucun
  nouveau type à créer.
- Le domaine reste libre de toute chaîne d'affichage. Un nom d'équipe ne porte
  aucun invariant : c'est une donnée de présentation.
- Le nom est résolu par le **VM builder**, qui a déjà les deux `TeamInfoDto` en
  main (`build_recap_template` les charge pour les bannières).
- Faire descendre le nom aurait imposé un VO de nom dans le BC. `NameVo`
  (shared kernel) ne conviendrait pas : son regex `^[\p{L}0-9 -]+$` rejette les
  apostrophes, fréquentes dans les noms d'équipe.

**Si les deux camps bloquent simultanément**, le service retient le premier dans
l'ordre home → away. Un seul message est affiché : la maquette n'en prévoit
qu'un, et lever le premier blocage ne rendrait de toute façon pas le rapport
corrigeable.

## D. Erreur domaine

```rust
// domain/error.rs — nouveau variant
pub enum DomainError {
    // …existant…
    CorrectionNotAllowed(CorrectionBlocker),
}
```

Le blocker est transporté dans l'erreur : la règle 9 impose de réafficher la
raison **à jour** quand la revérification serveur échoue, pas un message
générique.

## E. Erreur applicative

```rust
// use_cases/unpublish_match_report_use_case.rs
#[derive(Debug)]
pub enum UnpublishMatchReportError {
    NotFound,
    NotPublished,
    NotEligible(CorrectionBlocker),
    Repository(String),
}
```

Pas de variant `Forbidden` : l'autorisation est vérifiée dans le handler via
`is_authorized()`, en amont du use case.

## F. Événement domaine

```rust
// domain/events.rs — nouveau variant
MatchReportUnpublished {
    unpublished_by: CoachId,
    unpublished_at: DateTime<Utc>,
},
```

Symétrique de `MatchReportPublished`. Pas de motif : règle 5.

## G. État — le drapeau de correction

`MatchReportReadyToPublish` porte un champ supplémentaire :

```rust
pub was_published_before: bool,
```

Positionné par `unpublish()`, il enregistre un **fait** — « ce rapport a déjà été
publié au moins une fois » — plutôt qu'un mode. `from_pre_match()` le met à
`false` : un rapport qui atteint cet état pour la première fois n'a jamais été
publié.

Condition d'affichage du bandeau de l'état 5 : `!is_published && was_published_before`.

### Pourquoi il ne vit pas sur `MatchReportPreMatch`

> **Correction apportée en phase 6.** Cette section affirmait l'inverse : que le
> drapeau devait exister sur les deux états, parce que l'édition d'une action
> ferait repasser le rapport par `PreMatch`. C'est faux.

`into_pre_match()` est une **conversion transitoire interne aux use cases**,
destinée à réutiliser les méthodes de commande de `MatchReportPreMatch`. Son
résultat n'est jamais persisté.

Dans `rehydrate()`, un rapport en `ReadyToPublish` qui reçoit `ActionRecorded`,
`ActionDeleted`, `PostMatchRecorded` ou tout autre événement d'édition **reste en
`ReadyToPublish` et voit son état muté en place**
(`match_report_state.rs`, arms `(ReadyToPublish(rtp), …)`). Aucun événement ne
ramène `ReadyToPublish` vers `PreMatch`.

Le drapeau survit donc à toute la séquence de correction en n'existant que sur
`ReadyToPublish`. Couvert par le test
`le_drapeau_survit_a_l_edition_apres_depublication`.

## H. View model

```rust
// io/web/builders.rs — VM dépendant du port, donc pas dans view_models.rs
pub struct CorrectionZoneVm {
    pub can_correct:    bool,
    pub blocked_reason: Option<String>,
    pub unpublish_url:  String,
}

pub fn build_correction_zone(
    eligibility:   &CorrectionEligibility,
    home_info:     &TeamInfoDto,
    away_info:     &TeamInfoDto,
    unpublish_url: String,
) -> CorrectionZoneVm
```

Placé dans `builders.rs` et non dans `view_models.rs` : il dépend de
`TeamInfoDto`, un DTO de port. C'est la règle CLAUDE.md « VMs purs domaine :
`from_domain()` co-localisé ; VMs dépendant du port : fonctions dans
`builders.rs` ». `PerformanceRowVm`, `RoundContextVm` et `TeamBannerVm` suivent
déjà ce placement.

`blocked_reason` est une **phrase complète prête à afficher**, construite ici :
le template ne fait aucune composition de message.

## I. Champs ajoutés à `RecapTemplate`

```rust
pub correction:       Option<CorrectionZoneVm>,  // Some(_) si le rapport est publié
pub under_correction: bool,                      // bandeau de l'état 5
```

`Option` plutôt qu'un booléen accompagnant une struct : la zone n'existe pas du
tout pour un rapport non publié, et le template n'a pas à combiner deux champs
pour le savoir.

## J. Signatures de port

```rust
// ports.rs — ITeamDataPort
async fn is_team_in_player_improvement(&self, team_id: &str) -> Result<bool, String>;

// ports.rs — IPlayerDataPort
async fn has_spent_spp_since_match(
    &self,
    team_id:         &str,
    match_report_id: &str,
) -> Result<bool, String>;
```

**Aucun nouveau DTO de port** — deux booléens.

`is_team_in_player_improvement` plutôt que `find_game_phase(team_id) -> String` :
le BC `match_report` n'a pas besoin de connaître la taxonomie complète des phases
de `teams`, et une phase typée en chaîne se dégrade en silence. La signature est
symétrique de l'`is_team_ready_to_play` déjà présente sur le même port.

## K. Payloads d'app events

```rust
// shared_kernel/app_events/match_report_app_events.rs
MatchReportUnpublished(MatchReportUnpublishedPayload)

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct MatchReportUnpublishedPayload {
    pub match_report_id: String,
    pub space_id:        String,
    pub competition_id:  String,
    pub season_id:       String,
    pub round_id:        String,
    pub pairing_id:      Option<String>,
    pub home_team_id:    String,
    pub away_team_id:    String,
    pub unpublished_at:  DateTime<Utc>,
}
```

```rust
// shared_kernel/app_events/player_match_impact_app_events.rs
TeamMatchImpactReverted {
    team_id:         String,
    match_report_id: String,
},
```

Identifiants seulement, **aucune action** : chaque BC défait ce qu'il a
lui-même enregistré (cf. `03-back.md`).

`pairing_id` est présent bien qu'aucune compensation ne recrée de pairing — il
permet au listener `competitions` de cibler sa ligne de projection sans requête
de résolution supplémentaire.

## Règles métier identifiées en phase 4

Aucune règle nouvelle. Deux **précisions** sur des règles existantes, à porter
en phase 6 :

1. **Règle 3** — quand les deux camps bloquent simultanément, un seul message
   est affiché, celui du camp home. Lever le premier blocage ne rendrait pas le
   rapport corrigeable de toute façon.
2. **Règle 9** — la raison réaffichée après un échec de revérification est celle
   recalculée au moment du POST, pas celle affichée à l'ouverture de la page.
   C'est ce qui justifie que `CorrectionBlocker` voyage dans l'erreur.
