# Step 3 & 4 — Actions match — Domaine

---

## Règles métier — récapitulatif exhaustif validé

| # | Règle |
|---|---|
| R1 | Un tour est valide si `1 ≤ turn ≤ 16` → sinon `InvalidTurn` |
| R2 | Plusieurs actions peuvent être enregistrées pour le même joueur dans le même tour |
| R3 | Aucune limite sur le nombre de MVP par équipe |
| R4 | Les événements sont indépendants : supprimer un action n'entraîne pas la suppression d'un autre |
| R5 | Pour l'action `Blessé`, `injury_type` est requis — résolu avant d'appeler le domaine |
| R6 | Pour `InjuryType::Sequel`, `sequel_stat` est requis — résolu avant d'appeler le domaine |
| R7 | `BlessureSerieuse` (11-12) = niggling — tracking dans BC Players (future feature, pas de règle domaine ici) |
| R8 | Les TempPlayers d'une équipe sont initialisés après l'enregistrement de ses inducements |
| R9 | Ré-enregistrer les inducements d'une équipe réinitialise ses TempPlayers (reset + init) |
| R10 | Journaliers = `max(0, 11 - joueurs_disponibles)` — calculé hors domaine (use case via port) |
| R11 | Les actions sont enregistrables tant que le match report est en état `PreMatch` |

---

## Nouveaux champs de l'agrégat `MatchReportPreMatch`

```rust
pub struct MatchReportPreMatch {
    // … champs existants …

    // Joueurs temporaires (créés en fin d'étape 2)
    pub home_temp_players: Vec<TempPlayer>,
    pub away_temp_players: Vec<TempPlayer>,

    // Actions enregistrées (étapes 3 et 4)
    pub home_actions: Vec<MatchAction>,
    pub away_actions: Vec<MatchAction>,
}
```

Valeurs initiales (dans `from_draft`) : `Vec::new()` pour les quatre champs.

---

## Nouveaux value objects — `value_objects.rs`

```rust
// Identifiant d'une action (ULID, généré par le use case)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ActionId(pub String);

// Identifiant d'un joueur temporaire (ULID, généré par le use case)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TempPlayerId(pub String);

// Tour de jeu
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TurnNumber(u8);

impl TurnNumber {
    pub fn try_new(value: u8) -> Result<Self, DomainError> {
        if (1..=16).contains(&value) { Ok(Self(value)) }
        else { Err(DomainError::InvalidTurn(value)) }
    }
    pub fn value(&self) -> u8 { self.0 }
}

// Côté de l'équipe dans ce rapport
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TeamSide { Home, Away }

// Joueur qui réalise l'action
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ActionPlayer {
    Regular(PlayerId),   // PlayerId du BC Players (shared_kernel)
    Temp(TempPlayerId),  // Star / Merc / Journalier
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
    Commotion,
    Amoche,
    BlessureSerieuse,
    Sequel { stat: SequelStat },
    Mort,
}

// Pénalité de séquelle
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SequelStat { MinusAv, MinusMa, MinusPa, MinusAg, MinusSt }

// Entité joueur temporaire
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TempPlayer {
    pub id:           TempPlayerId,
    pub team_id:      TeamId,
    pub kind:         TempPlayerKind,
    pub display_name: Option<String>,  // Some pour les stars
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TempPlayerKind {
    StarPlayer { ref_uid: String, position_uid: String },
    Mercenary  { position_uid: String },
    Journalier { position_uid: String },
}

// Action persistée dans l'agrégat
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchAction {
    pub id:                  ActionId,
    pub turn:                TurnNumber,
    pub player:              ActionPlayer,
    pub action:              MatchActionType,
    pub player_display_name: String,
}
```

---

## Nouveaux domain events

À ajouter dans `MatchReportDomainEvent` :

```rust
TempPlayersInitialized {
    team_id: TeamId,
    players: Vec<TempPlayer>,
},

TempPlayersReset {
    team_id: TeamId,
},

ActionRecorded {
    action_id:           ActionId,
    team_side:           TeamSide,
    turn:                TurnNumber,
    player:              ActionPlayer,
    action:              MatchActionType,
    player_display_name: String,
    recorded_by:         CoachId,
},

ActionDeleted {
    action_id:  ActionId,
    team_side:  TeamSide,
    deleted_by: CoachId,
},
```

---

## Nouvelles méthodes domaine

### `init_temp_players`

```rust
pub fn init_temp_players(
    &self,
    team_id: &TeamId,
    players: Vec<TempPlayer>,
) -> (Self, MatchReportDomainEvent)
```

Met à jour `home_temp_players` ou `away_temp_players` selon `team_id`. Émet `TempPlayersInitialized`.
Ne valide pas la liste — la construction est faite par le use case.

---

### `reset_temp_players`

```rust
pub fn reset_temp_players(
    &self,
    team_id: &TeamId,
) -> (Self, MatchReportDomainEvent)
```

Vide `home_temp_players` ou `away_temp_players` selon `team_id`. Émet `TempPlayersReset`.

---

### `record_action`

```rust
pub fn record_action(
    &self,
    team_side: TeamSide,
    turn: TurnNumber,
    player: ActionPlayer,
    action: MatchActionType,
    player_display_name: String,
    action_id: ActionId,
) -> (Self, MatchReportDomainEvent)
```

Aucune validation métier à ce stade (R2, R3, R4 : pas de contraintes). Construit un `MatchAction`
et l'ajoute à `home_actions` ou `away_actions`. Émet `ActionRecorded`.

`TurnNumber` est déjà validé en amont (smart constructor). Le domaine ne revalide pas.

---

### `delete_action`

```rust
pub fn delete_action(
    &self,
    action_id: &ActionId,
) -> Result<(Self, MatchReportDomainEvent), DomainError>
```

Cherche `action_id` dans `home_actions` puis `away_actions`. Si trouvé : retire l'entrée et émet
`ActionDeleted { action_id, team_side, deleted_by }`. Sinon : `Err(DomainError::ActionNotFound)`.

---

### Méthodes de lecture (non mutantes)

```rust
// Permet au use case de résoudre le display_name des joueurs Temp
pub fn temp_players_for(&self, side: TeamSide) -> &[TempPlayer]

// Filtre les StarPlayerEngaged pour une équipe (needed by init_temp_players use case)
pub fn star_player_uids_for(&self, team_id: &TeamId) -> Vec<InducementId>

// Accès aux inducements d'une équipe (needed by init_temp_players use case)
pub fn purchases_for(&self, team_id: &TeamId) -> &[InducementPurchase]

// Accès aux actions pour le rendu du turn-selector (has_events par tour)
pub fn actions_for(&self, side: TeamSide) -> &[MatchAction]
```

---

## Nouvelles erreurs domaine

```rust
// dans DomainError
InvalidTurn(u8),        // tour hors 1..=16
ActionNotFound(String), // action_id introuvable dans home_actions + away_actions
```

Ajout dans `fmt::Display` :
```rust
Self::InvalidTurn(v)        => write!(f, "tour invalide : {v} (attendu 1..=16)"),
Self::ActionNotFound(id)    => write!(f, "action introuvable : {id}"),
```

---

## Rehydratation — nouveaux apply

Dans `MatchReportState::apply()` (ou équivalent), ajouter les branches pour les nouveaux events :

| Event | Effet sur l'agrégat |
|---|---|
| `TempPlayersInitialized { team_id, players }` | `home_temp_players = players` si team_id == home, sinon `away_temp_players` |
| `TempPlayersReset { team_id }` | Vide `home_temp_players` ou `away_temp_players` |
| `ActionRecorded { team_side, … }` | Push `MatchAction` dans `home_actions` ou `away_actions` |
| `ActionDeleted { action_id, team_side }` | Retire l'entrée correspondante de `home_actions` ou `away_actions` |

---

## Tests unitaires prévus

| Test | Règle couverte |
|---|---|
| `turn_number_rejects_0` | R1 |
| `turn_number_rejects_17` | R1 |
| `turn_number_accepts_1_and_16` | R1 |
| `record_action_pushes_to_home_actions` | R11 |
| `record_action_pushes_to_away_actions` | R11 |
| `record_two_actions_same_player_same_turn` | R2 |
| `record_two_mvp_same_team` | R3 |
| `delete_action_removes_entry` | R4 |
| `delete_action_fails_when_not_found` | R4 |
| `init_temp_players_sets_list` | R8 |
| `reset_temp_players_clears_list` | R9 |
| `star_player_uids_for_returns_engaged_uids` | — |
| `purchases_for_returns_team_inducements` | — |
| `actions_for_returns_correct_side` | — |
