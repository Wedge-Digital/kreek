# Calendrier · appariement des présents — contrats de données

**Format d'entrée** : formulaires HTMX, lus par `axum_extra::extract::Form`
(déjà employé dans `settings/pools_panel.rs`), qui accepte les clés répétées
(`team_ids=…&team_ids=…`). Les actions actuelles du Calendrier passent par
`fetch` et du JSON ; le widget n'en a pas besoin.

## 1. `GET …/schedule/draw` — l'étape « Présents »

```rust
// Entrée HTTP — lecture, primitives acceptées
#[derive(Deserialize)]
pub struct DrawQuery {
    pub round_id: String,
    pub absent: Option<String>, // ids séparés par des virgules
}

// Sortie du service de lecture — use_cases/admin/draw/draw_candidates_service.rs
pub struct DrawCandidates { pub groups: Vec<CandidateGroup> }
pub struct CandidateGroup { pub group_name: String, pub teams: Vec<Candidate> }
pub struct Candidate {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub matches_played: u32,
}

// View models — io/web/admin/view_models
pub struct DrawPresenceVm { pub round_id: String, pub groups: Vec<PresenceGroupVm> }
pub struct PresenceGroupVm { pub name: String, pub teams: Vec<PresenceTeamVm> }
pub struct PresenceTeamVm {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub matches_played: u32,
    pub present: bool,
}
```

| DTO | Émis par | Consommé par |
|---|---|---|
| `DrawQuery` | le navigateur (clic « Faire les appariements », retour « Modifier les présents ») | `schedule_draw_widget` |
| `DrawCandidates` | `draw_candidates_service`, à partir de `ITeamInfoPort`, `IGroupRepository` et des journées | `schedule_draw_widget` |
| `DrawPresenceVm` | `DrawPresenceVm::from_candidates(&candidates, &absent)` | gabarit `schedule-draw-presence.html` ; Alpine y compte les cases cochées |

Le service refuse une journée de repos ou une journée déjà pourvue de
rencontres.

## 2. `POST …/draw/preview` — l'aperçu, sans écriture

```rust
#[derive(Deserialize)]
pub struct PreviewDrawForm {
    pub round_id: String,
    #[serde(default)]
    pub team_ids: Vec<String>,
}

pub struct PreviewDrawCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub present: Vec<TeamId>,
}

// Sortie du use case
pub struct DrawPreview { pub groups: Vec<GroupPreview> }
pub struct GroupPreview {
    pub group_name: String,
    pub pairings: Vec<PreviewPairing>,
    pub exempted: Option<Candidate>,
    pub absent: Vec<Candidate>,
}
pub struct PreviewPairing {
    pub home: Candidate,
    pub away: Candidate,
    pub history: History, // domaine : inédite, ou revanche (fois, dernière journée)
}

// View models
pub struct DrawPreviewVm {
    pub round_id: String,
    pub groups: Vec<PreviewGroupVm>,
    pub absent_csv: String, // pour « Modifier les présents »
}
pub struct PreviewGroupVm {
    pub name: String,
    pub pairings: Vec<PreviewPairingVm>,
    pub exempted: Option<BenchTeamVm>,
    pub absent_names: Vec<String>,
    pub has_rematch: bool,
}
pub struct PreviewPairingVm {
    pub home_id: String,
    pub home_name: String,
    pub away_id: String,
    pub away_name: String,
    pub history_label: String, // « 1re rencontre », « Revanche · déjà jouée en J1 »
    pub is_rematch: bool,
}
pub struct BenchTeamVm { pub name: String, pub matches_played: u32 }
```

| DTO | Émis par | Consommé par |
|---|---|---|
| `PreviewDrawForm` | le navigateur (« Tirer au sort », « Retirer au sort ») | `schedule_draw_actions::post_preview` |
| `PreviewDrawCommand` | le handler, chaque id passé par `TeamId::try_new` | `preview_draw_use_case` |
| `DrawPreview` | `preview_draw_use_case` | le handler |
| `DrawPreviewVm` | `DrawPreviewVm::from_domain(&preview)` | gabarit `schedule-draw-preview.html` |

Les rencontres sont recopiées dans des champs cachés de l'aperçu, qui les rend
à la confirmation ; « Retirer au sort » renvoie le même formulaire.

`has_rematch` suffit à « une revanche n'a pas pu être évitée » : le tirage
maximise d'abord le nombre de rencontres, puis minimise les revanches — une
revanche n'y figure que si elle était inévitable.

## 3. `POST …/draw/confirm` — l'écriture

```rust
#[derive(Deserialize)]
pub struct ConfirmDrawForm {
    pub round_id: String,
    #[serde(default)]
    pub pairings: Vec<String>, // "home_id:away_id"
    pub exempted: Option<String>,
}

pub struct ConfirmDrawCommand {
    pub round_id: MatchId,
    pub season_id: SeasonId,
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub pairings: Vec<(TeamId, TeamId)>,
    pub exempted: Option<TeamId>,
}

pub struct ConfirmOutcome {
    pub written: usize,
    pub exempted_name: Option<String>,
    pub absent_names: Vec<String>,
}

pub enum ConfirmDrawError {
    RoundNotFound,
    IsRestDay,
    PairingsAlreadyExist,
    Refused(DomainError), // équipe non inscrite, même coach, équipe en double, poules mêlées
    Repository(String),
}
```

| DTO | Émis par | Consommé par |
|---|---|---|
| `ConfirmDrawForm` | le navigateur (« Écrire au calendrier ») | `schedule_draw_actions::post_confirm` |
| `ConfirmDrawCommand` | le handler, qui découpe chaque `"a:b"` et valide les deux ids | `confirm_draw_use_case` |
| `ConfirmOutcome` | `confirm_draw_use_case` | le handler : `HX-Trigger: scheduleChanged` et JSON `{ written, exempted, absent }` pour le toast de succès |
| `ConfirmDrawError` | `confirm_draw_use_case` | le handler : JSON `{ error }`, rendu en toast d'erreur par `handleScheduleActionResponse` |

Les absentes se déduisent — les inscrites ni appariées ni exemptées — et n'ont
pas à être transmises.

## 4. `POST …/generate-round` (existant)

Contrats inchangés ; seul l'intérieur du use case change.

## Ce qui n'est pas affiché

`optimum_prouve` du tirage, faux quand le budget de calcul a coupé la recherche.
Le tirage rendu reste valide et maximal ; le cas n'apparaît qu'au-delà de
vingt-quatre équipes dans une poule où presque toutes les paires sont jouées.

## Règles métier ajoutées à cette étape

Aucune.
