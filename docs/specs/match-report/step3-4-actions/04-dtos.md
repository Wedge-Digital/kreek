# Step 3 & 4 — Actions match — Contrats de données

Chaque DTO est décrit avec son **interface d'utilisation** : qui le produit, qui le consomme.

---

## 1. DTOs d'entrée — GET pages et widgets

### Params de path — page hôte

```
{space_id}        : String
{match_report_id} : String
```

### Params de path — widgets BC MatchReport

```
{space_id}        : String
{match_report_id} : String
```

Le `team_id` de l'équipe traitée n'est **pas** un paramètre de path des widgets — il
est dérivé par le serveur depuis l'agrégat selon que l'on est en step3 (home) ou step4 (away).

### Params de path — widget BC Players

```
{space_id} : String
{team_id}  : String
```

---

## 2. DTOs d'entrée — POST enregistrement d'une action

### Form HTTP unifié

```rust
#[derive(Deserialize)]
pub struct RecordActionForm {
    pub turn:        u8,
    pub player_id:   String,  // PlayerId (regular) ou TempPlayerId (star/merc/journalier)
    pub player_type: String,  // "regular" | "temp"
    pub action_type: String,  // "td"|"passe"|"interception"|"agression"|"lancer"|"sortie"|"mvp"|"blesse"
    pub injury_type: Option<String>,  // "commotion"|"amoche"|"serious"|"sequel"|"death" — si action_type = "blesse"
    pub sequel_stat: Option<String>,  // "-1_av"|"-1_ma"|"-1_pa"|"-1_ag"|"-1_st" — si injury_type = "sequel"
}
```

| Produit par | Consommé par |
|---|---|
| Alpine (`hx-vals` ou `hx-include`) dans `action-panel.html` | `record_action_controller::post_action()` → construit `RecordActionCommand` |

### Paramètre de path — DELETE action

```
DELETE /app/{space_id}/match-report/{match_report_id}/actions/{action_id}
```

| Produit par | Consommé par |
|---|---|
| Bouton suppression dans `action-log.html` (attribut `hx-delete`) | `record_action_controller::delete_action()` → construit `DeleteActionCommand` |

---

## 3. Commandes use case

### `RecordActionCommand` (nouveau)

```rust
pub struct RecordActionCommand {
    pub match_report_id: MatchReportId,
    pub team_side:       TeamSide,       // Home | Away — dérivé de step3/step4
    pub turn:            TurnNumber,
    pub player:          ActionPlayer,
    pub action:          MatchActionType,
    pub recorded_by:     CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `record_action_controller::post_action()` après validation des value objects | `record_action_use_case::execute()` |

### `DeleteActionCommand` (nouveau)

```rust
pub struct DeleteActionCommand {
    pub match_report_id: MatchReportId,
    pub action_id:       ActionId,
    pub deleted_by:      CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `record_action_controller::delete_action()` | `delete_action_use_case::execute()` |

### `InitTempPlayersCommand` (nouveau)

```rust
pub struct InitTempPlayersCommand {
    pub match_report_id: MatchReportId,
    pub team_id:         TeamId,
    pub recorded_by:     CoachId,
    // Le use case résout lui-même les données via les ports
}
```

| Produit par | Consommé par |
|---|---|
| `inducements_controller::post_inducements()` — appelé après `record_inducements_use_case` | `init_temp_players_use_case::execute()` |

---

## 4. Value objects nouveaux — domaine BC MatchReport

```rust
// Identifiant d'une action enregistrée (ULID)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

// Identifiant d'un joueur temporaire (ULID, scoped au match)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TempPlayerId(pub String);

// Numéro de tour : 1..=16
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnNumber(pub u8);

// Côté de l'équipe dans ce rapport
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamSide { Home, Away }

// Joueur qui réalise l'action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionPlayer {
    Regular(PlayerId),     // joueur permanent BC Players
    Temp(TempPlayerId),    // star / merc / journalier
}

// Type d'action
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MatchActionType {
    Touchdown,
    Passe,
    Interception,
    Agression,
    Lancer,
    Sortie,
    Mvp,
    Blesse { injury: InjuryType },
}

// Type de blessure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InjuryType {
    Commotion,               // 1-8
    Amoche,                  // 9-10
    BlessureSerieuse,        // 11-12 → niggling pour joueurs réguliers
    Sequel { stat: SequelStat }, // 13-14 → pénalité de caractéristique
    Mort,                    // 15-16
}

// Pénalité de séquelle
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SequelStat { MinusAv, MinusMa, MinusPa, MinusAg, MinusSt }

// Entité joueur temporaire (star, merc, journalier)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempPlayer {
    pub id:           TempPlayerId,
    pub team_id:      TeamId,
    pub kind:         TempPlayerKind,
    pub display_name: Option<String>, // Some pour les star players (nom du référentiel)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TempPlayerKind {
    StarPlayer   { ref_uid: String, position_uid: String },
    Mercenary    { position_uid: String },
    Journalier   { position_uid: String },
}

// Action enregistrée (état résultant d'un ActionRecorded)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAction {
    pub id:     ActionId,
    pub turn:   TurnNumber,
    pub player: ActionPlayer,
    pub action: MatchActionType,
}
```

| Produit par | Consommé par |
|---|---|
| Smart constructors dans `value_objects.rs` | Commandes, événements domaine, agrégat `MatchReportPreMatch` |

---

## 5. Domain events nouveaux

### `TempPlayersInitialized`

```rust
TempPlayersInitialized {
    team_id: TeamId,
    players: Vec<TempPlayer>,  // liste complète pour cette équipe
}
```

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::init_temp_players()` | `init_temp_players_use_case` (persiste via `repo.append()`) · `rehydrate()` (peuple `home_temp_players` / `away_temp_players`) |

### `TempPlayersReset`

```rust
TempPlayersReset {
    team_id: TeamId,
}
```

Émis avant `TempPlayersInitialized` quand les inducements d'une équipe sont re-soumis.

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::reset_temp_players()` | `init_temp_players_use_case` · `rehydrate()` (vide `home_temp_players` / `away_temp_players` selon team_id) |

### `ActionRecorded`

```rust
ActionRecorded {
    action_id:           ActionId,
    team_side:           TeamSide,
    turn:                TurnNumber,
    player:              ActionPlayer,
    action:              MatchActionType,
    player_display_name: String,  // dénormalisé — "{personal_name} (#{jersey})" pour Regular, display_name pour Temp
    recorded_by:         CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::record_action()` | `record_action_use_case` (persiste) · `rehydrate()` (ajoute à `home_actions` / `away_actions`) · broadcast futur vers BC Players |

### `ActionDeleted`

```rust
ActionDeleted {
    action_id: ActionId,
    team_side: TeamSide,
    deleted_by: CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::delete_action()` | `delete_action_use_case` (persiste) · `rehydrate()` (retire de `home_actions` / `away_actions`) |

---

## 6. DTOs de port

### `IPlayerDataPort` (nouveau — `match_report/ports.rs`)

```rust
pub struct MatchPlayerCountDto {
    pub available: u8,  // joueurs en état de jouer (toujours = total en V1)
    pub total: u8,
}

pub struct PlayerDisplayDto {
    pub display_name: String,  // "{personal_name} (#{jersey})" — None jersey → nom seul
}

#[async_trait]
pub trait IPlayerDataPort: Send + Sync {
    async fn count_available_players(
        &self,
        team_id: &str,
    ) -> Result<MatchPlayerCountDto, String>;

    async fn find_player_display(
        &self,
        player_id: &str,
    ) -> Option<PlayerDisplayDto>;
}
```

Adapter : `src/infrastructure/match_report/player_data_adapter.rs`

| Méthode | Produit par | Consommé par |
|---|---|---|
| `count_available_players` | Adapter → `IPlayerProjectionRepository` BC Players | `init_temp_players_use_case` — calcule `max(0, 11 - available)` journaliers |
| `find_player_display` | Adapter → `IPlayerProjectionRepository` BC Players | `record_action_use_case` — dénormalise le nom dans `ActionRecorded` |

### `ITeamDataPort` — nouvelle méthode

```rust
pub struct JournalierPositionDto {
    pub position_uid:  String,
    pub position_name: String,
}

// Ajout au trait ITeamDataPort
async fn find_journalier_position(
    &self,
    team_id: &str,
) -> Option<JournalierPositionDto>;
```

L'adapter résout : `team_id → roster_id` (déjà disponible) → lookup `is_journalier = true` dans le référentiel.

| Produit par | Consommé par |
|---|---|
| Adapter Teams+References BC | `init_temp_players_use_case` — crée les instances `TempPlayer::Journalier { position_uid }` |

### `ITeamDataPort` — extension `TeamInfoDto`

Aucune modification : `roster_id` est déjà présent depuis step 2.

---

## 7. VMs de sortie — templates BC MatchReport

### Page hôte `match-report-actions.html`

```rust
#[derive(Template)]
#[template(path = "match-report-actions.html")]
pub struct ActionsStepTemplate {
    pub app_routes:               AppRoutes,
    pub space_id:                 String,
    pub match_report_id:          String,
    pub team_name:                String,
    pub step:                     u8,     // 3 (home) | 4 (away)
    pub turn_selector_url:        String,
    pub player_selector_url:      String, // URL widget BC Players, baked avec team_id
    pub temp_player_selector_url: String,
    pub action_panel_url:         String,
    pub action_log_url:           String,
    pub prev_url:                 String,
    pub next_url:                 String,
}
```

| Produit par | Consommé par |
|---|---|
| `actions_step_controller::get_step()` | Template Askama `match-report-actions.html` |

### Widget turn-selector

```rust
pub struct TurnButtonVm {
    pub number:     u8,
    pub half:       u8,         // 1 (tours 1-8) ou 2 (tours 9-16)
    pub has_events: bool,       // true si ≥ 1 action enregistrée sur ce tour
}

#[derive(Template)]
#[template(path = "widgets/turn-selector.html")]
pub struct TurnSelectorTemplate {
    pub turns: Vec<TurnButtonVm>,  // 16 éléments, toujours
}
```

| Produit par | Consommé par |
|---|---|
| `turn_selector_widget::get_turn_selector()` — compte les actions par tour depuis l'agrégat | Template `turn-selector.html` · Alpine `x-data` (selectedTurn, emit `turnSelected`) |

### Widget temp-player-selector

```rust
pub struct TempPlayerVm {
    pub id:           String,  // TempPlayerId
    pub display_name: String,  // Nom de la star, ou "Mercenaire", ou "Journalier"
    pub kind_css:     String,  // "star" | "merc" | "journalier" — classe CSS du chip
}

#[derive(Template)]
#[template(path = "widgets/temp-player-selector.html")]
pub struct TempPlayerSelectorTemplate {
    pub players: Vec<TempPlayerVm>,
}
```

| Produit par | Consommé par |
|---|---|
| `temp_player_selector_widget::get_temp_players()` — lit `home_temp_players` / `away_temp_players` depuis l'agrégat | Template `temp-player-selector.html` · Alpine (selectedPlayerId, emit `playerSelected`) |

### Widget action-panel

```rust
#[derive(Template)]
#[template(path = "widgets/action-panel.html")]
pub struct ActionPanelTemplate {
    pub post_url: String,  // POST /step3/actions ou /step4/actions
}
```

Le panel ne porte pas de données serveur — toute la logique est Alpine (`enabled`, `showInjuryPanel`, etc.). Le `post_url` est baked au rendu pour éviter que le template ait à construire des URLs dynamiquement.

| Produit par | Consommé par |
|---|---|
| `action_panel_widget::get_action_panel()` | Template `action-panel.html` · Alpine (state machine complète) |

### Widget action-log

```rust
pub struct ActionLogEntryVm {
    pub action_id:      String,
    pub turn:           u8,
    pub player_display: String,  // ex. "Grotak (#3)" | "Star Player" | "Journalier"
    pub action_label:   String,  // ex. "Touchdown" | "Blessé · Amoché" | "Blessé · Séquelle (-1 AV)"
    pub delete_url:     String,
}

#[derive(Template)]
#[template(path = "widgets/action-log.html")]
pub struct ActionLogTemplate {
    pub entries: Vec<ActionLogEntryVm>,
}
```

La construction de `player_display` est **triviale** : le champ `player_display_name` est dénormalisé dans l'event `ActionRecorded` au moment de l'enregistrement. Le log lit directement ce champ — aucune requête vers BC Players au rendu.

Résolution du nom au moment de l'enregistrement (dans `record_action_use_case`) :
- Regular → `IPlayerDataPort::find_player_display(player_id)` → `"{personal_name} (#{jersey})"`
- Temp → `TempPlayer::display_name` (déjà dans l'agrégat)

| Produit par | Consommé par |
|---|---|
| `action_log_widget::get_action_log()` | Template `action-log.html` |

---

## 8. VM de sortie — widget BC Players

### Widget match-player-selector

```rust
pub struct MatchPlayerVm {
    pub player_id:     String,
    pub jersey:        Option<u8>,
    pub position_name: String,
    pub personal_name: String,
    pub is_available:  bool,  // toujours true en V1 — chip disabled si false (future use)
}

#[derive(Template)]
#[template(path = "widgets/match-player-selector.html")]
pub struct MatchPlayerSelectorTemplate {
    pub players: Vec<MatchPlayerVm>,
}
```

Construit depuis `IPlayerProjectionRepository::find_by_team_id()` (déjà disponible dans BC Players).

| Produit par | Consommé par |
|---|---|
| `match_player_selector_widget::get_match_player_selector()` (BC Players) | Template `match-player-selector.html` · Alpine (selectedPlayerId, emit `playerSelected`) |
