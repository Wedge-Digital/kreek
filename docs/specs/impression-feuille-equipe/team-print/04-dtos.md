# Page « Imprimer » — Phase 4 : contrats de données

Entrée : `03-back.md`. Toutes les routes sont des `GET` : les DTO d'entrée sont
des **queries**, les primitives y sont admises (cf. `CLAUDE.md`, « principe
CQRS »). Les options à valeurs fermées sont néanmoins des `enum` désérialisés,
pour qu'une valeur inconnue réponde 400 au lieu de se replier en silence.

Les identifiants d'équipe ne sont **jamais** dans une query : `{team_id}` et
`{other_team_id}` sont des paramètres de chemin, contrôlés par `space_scope`.

## 1. Entrées

### Options d'impression — `teams`

```rust
#[derive(Deserialize)]
pub struct PrintOptionsQuery {
    #[serde(default)] pub skills_annex: bool,
    #[serde(default)] pub inducements: bool,
    #[serde(default)] pub star_players: bool,
    #[serde(default)] pub match_sheet: bool,
    #[serde(default)] pub two_copies: bool,
    #[serde(default)] pub orientation: Orientation,      // portrait | landscape
    #[serde(default)] pub density: Density,              // normal | compact
    #[serde(default)] pub ink: Ink,                      // color | mono
    #[serde(default)] pub style: SheetStyle,             // plain | official | pitch
    #[serde(default)] pub unavailable: UnavailableMode,  // show | strike | hide
    #[serde(default)] pub treasury: bool,
    #[serde(default)] pub blank_rows: bool,
    #[serde(default)] pub home: HomeSide,                // own | other (R10)
    pub round_id: Option<String>,                        // journée du prochain match
}
```

| Émis par | Consommé par |
|---|---|
| le formulaire de la page « Imprimer », par `hx-include` | le contrôleur de l'aperçu, qui en transmet **le sous-ensemble utile** à chaque URL de fragment |

### Sous-ensembles transmis aux fragments

| Fragment | Query reçue |
|---|---|
| En-tête de fiche (teams) | `role: SheetRole` (`own` \| `other`), `treasury: bool` |
| Joueurs (players) | `unavailable: UnavailableMode`, `blank_rows: bool`, `density: Density` |
| Annexe des compétences (players) | `orientation`, `density` (pagination) |
| Coups de pouce, star players (match_report) | `inducements: bool`, `star_players: bool`, `orientation`, `density` |
| Feuille de match (match_report) | `home: HomeSide`, `round_id: Option<String>` |

Chaque BC déclare **ses propres** `enum` (`Orientation`, `Density`, …) : pas de
type partagé entre BCs pour un paramètre d'URL.

`round_id` est vérifié par match_report : la journée doit appartenir à la
saison de l'équipe (`find_round_context(season_id, round_id)`), sinon elle est
ignorée et la feuille imprime des lignes à remplir.

## 2. Sorties (VMs)

### Page et aperçu — `teams`

```rust
pub struct TeamPrintPageVm {            // page hôte
    pub team_name: String,
    pub space_name: String,
    pub next_opponent_url: String,      // AppRoutes.competitions
    pub competitions_json_url: String,  // cascade existante
    pub seasons_json_url: String,
    pub teams_json_url: String,
    pub preview_url: String,
    pub back_url: String,               // fiche d'équipe
}

pub struct PrintPreviewVm {
    pub root_class: String,             // "orientation-landscape density-compact ink-mono style-official"
    pub slots: Vec<PrintSlotVm>,
}

pub struct PrintSlotVm {
    pub label: String,                  // « Fiche · Les Tailleurs de Roc »
    pub kind: PrintSlotKind,            // TeamSheet | Fragment
    pub urls: Vec<String>,              // TeamSheet : [en-tête, joueurs] ; Fragment : [url]
}
```

Une fiche d'équipe tient dans une feuille A4 que l'**hôte** ouvre, et dans
laquelle il pose deux emplacements : l'en-tête (teams) puis les joueurs
(players). Les autres fragments rendent eux-mêmes leurs feuilles, une ou
plusieurs (annexes paginées).

### En-tête de fiche — `teams`

```rust
pub struct TeamSheetHeadVm {
    pub role_label: &'static str,       // « Mon équipe » | « Adversaire »
    pub name: String,
    pub initials: String,
    pub logo_url: Option<String>,       // transformé Cloudinary
    pub roster_name: String,
    pub coach_name: String,
    pub competition_name: Option<String>,
    pub season_name: Option<String>,
    pub team_value_kpo: u32,            // team.team_value (R4)
    pub treasury_kpo: Option<u32>,      // Some seulement si role = own et treasury (R8)
    pub dedicated_fans: u8,
    pub alignable: u8,                  // presence.alignable() — domaine
    pub squad_size: u8,
    pub staff: Vec<StaffPrintLineVm>,
}

pub struct StaffPrintLineVm {
    pub label: &'static str,
    pub quantity: u8,
    pub unit_price_kpo: u32,            // catalogue du roster (point E)
}
```

`from_domain(&Team, &RosterCatalogDto, &[SquadMemberDto], role, treasury)` :
co-localisé, mais il lit le catalogue — il vit donc dans `builders.rs`.

### Joueurs — `players`

```rust
pub struct PlayerPrintTableVm {
    pub rows: Vec<PlayerPrintRowVm>,
    pub blank_rows: u8,                 // 16 − lignes affichées, si l'option est cochée
    pub unavailable: UnavailableMode,
}

pub struct PlayerPrintRowVm {
    pub jersey: Option<i16>,
    pub name: String,
    pub position_name: String,
    pub stats: Option<ResolvedPlayerStats>,
    pub base_skills: Vec<String>,
    pub acquired_skills: Vec<String>,
    pub persistent_injuries: u16,
    pub spp_remaining: u32,             // R5
    pub value_kpo: i32,
    pub absence: Option<Absence>,       // « Manque le prochain match », retraite
    pub is_journeyman: bool,            // R6 — à ajouter à la lecture
}
```

Construit depuis `build_player_rows`, qui reçoit `membership` en plus. Le
gabarit d'impression est **distinct** de `player-table-fragment.html`, qui
embarque le formulaire d'édition.

### Annexe des compétences — `players`

```rust
pub struct SkillsAnnexVm { pub pages: Vec<Vec<SkillAnnexEntryVm>> }

pub struct SkillAnnexEntryVm {
    pub name: String,
    pub category_label: String,
    pub description: String,
}
```

Sans doublon, triées par nom, découpées par le serveur selon l'orientation et
la densité. La ligne « portée par… » de la maquette disparaît : players ne
connaît pas le nom des équipes.

### Prochain adversaire — `competitions`

```rust
pub enum NextOpponentOptionVm {
    Available {
        other_team_id: String,          // valeur du bouton radio
        team_name: String,
        roster_name: String,
        round_id: String,
        round_label: String,            // « Journée 6, samedi 25 octobre »
        opponent_is_home: bool,         // R10
    },
    Unavailable { reason: &'static str }, // « Aucun match à venir… » (R2)
}
```

Construit depuis `PairingDisplayDto` : le côté qui n'est pas `team_id`.

### Coups de pouce et star players — `match_report`

```rust
pub struct InducementsPrintVm {
    pub team_names: Vec<String>,         // 1 ou 2 colonnes
    pub inducements: Vec<InducementPrintRowVm>,
    pub star_players: Vec<StarPlayerPrintVm>,
    pub pages: u8,
}

pub struct InducementPrintRowVm {
    pub name: String,
    pub description: String,
    pub price_kpo: Vec<Option<u32>>,     // par équipe ; None = « — » (R7)
    pub max_qty: u8,
}

pub struct StarPlayerPrintVm {
    pub name: String,
    pub player_type: String,
    pub cost_kpo: u32,
    pub ma: u8, pub st: u8, pub ag: String, pub pa: String, pub av: String,
    pub skills: Vec<String>,             // noms résolus
    pub ability_name: String,
    pub ability_description: String,
    pub available_for: Vec<String>,      // noms des équipes imprimées
}
```

### Feuille de match — `match_report`

```rust
pub struct MatchSheetVm {
    pub header: MatchSheetHeaderVm,      // compétition, saison, journée (si round_id valide)
    pub own: MatchSheetTeamVm,
    pub other: Option<MatchSheetTeamVm>, // None : colonne à remplir à la main
    pub reminders: MatchRemindersVm,
}

pub struct MatchSheetTeamVm {
    pub name: String,
    pub roster_name: String,
    pub coach_name: String,
    pub team_value_kpo: u32,
    pub dedicated_fans: u32,
    pub money: MoneyStatusVm,
    pub players: Vec<MatchSheetPlayerVm>,  // alignables, journaliers compris
    pub spp_scale: SppScaleVm,
}

pub enum MoneyStatusVm {
    TopDog { own_tv: u32, other_tv: u32 },
    Underdog { petty_cash_kpo: u32 },      // écart de VE (R11)
    Unknown,                               // pas d'adversaire
}

pub struct MatchSheetPlayerVm { pub jersey: Option<u8>, pub name: String, pub is_journeyman: bool }
pub struct SppScaleVm { pub touchdown: u8, pub casualty: u8, pub interception: u8, pub pass: u8, pub mvp: u8 }
pub struct MatchRemindersVm { pub mvp: Option<String>, pub fans: Option<String>, pub weather: Vec<String> }
```

`MoneyStatusVm` transpose le résultat des fonctions pures du domaine
(`03-back.md`, décision 2) — le gabarit ne compare aucune VE.

## 3. DTO des ports

### match_report — nouveau port `IReferenceCatalogPort` (synchrone, corpus en mémoire)

```rust
pub trait IReferenceCatalogPort: Send + Sync {
    fn find_inducement_labels(&self, uids: &[String]) -> Vec<InducementLabelDto>;
    fn find_star_players(&self, uids: &[String], roster_id: &str) -> Vec<StarPlayerDto>;
    fn find_spp_scale(&self, roster_id: &str) -> SppScaleDto;
    fn find_match_reminders(&self) -> MatchRemindersDto;
}

pub struct InducementLabelDto { pub uid: String, pub name: String, pub description: String }
pub struct StarPlayerDto { /* champs de StarPlayerPrintVm, sauf available_for ; skills résolus en noms */ }
pub struct SppScaleDto { pub touchdown: u8, pub casualty: u8, pub interception: u8, pub pass: u8, pub mvp: u8 }
pub struct MatchRemindersDto { pub mvp: Option<String>, pub fans: Option<String>, pub weather: Vec<String> }
```

`find_star_players` applique le filtre de disponibilité porté au domaine de
references (`03-back.md`, décision 3). Adapter :
`src/infrastructure/match_report/reference_catalog_adapter.rs`.

### match_report — méthodes ajoutées aux ports existants

| Port | Méthode | DTO |
|---|---|---|
| `IPlayerDataPort` | `list_alignable_players(team_id)` | `Vec<AlignablePlayerDto { jersey: Option<u8>, name: String, is_journeyman: bool }>` |
| `ITeamDataPort` | `find_team_season(team_id)` | `Option<String>` — la saison d'inscription, pour le palier et `round_id` |

### teams — rien de nouveau

`ISquadPort::find_squad` (alignables, journaliers) et `IRosterCatalogPort`
(prix du staff, relance) suffisent.

### players — champ ajouté

`PlayerProjection.membership` est déjà lu ; `build_player_rows` le transmet.

### references — fichier du corpus

```rust
#[derive(Deserialize, Default)]
pub struct MatchRemindersFile {
    #[serde(default)] pub mvp: Option<String>,
    #[serde(default)] pub fans: Option<String>,
    #[serde(default)] pub weather: Vec<String>,
}
```

Absent : `MatchRemindersFile::default()`, sans erreur. Présent mais mal formé :
erreur de démarrage nommant le fichier, comme les douze autres.

## Interfaces d'utilisation

| DTO / VM | Émis par | Consommé par |
|---|---|---|
| `PrintOptionsQuery` | formulaire de la page | contrôleur de l'aperçu (teams) |
| `PrintPreviewVm` | contrôleur de l'aperçu | gabarit de l'aperçu |
| `TeamSheetHeadVm` | builder teams | gabarit d'en-tête |
| `PlayerPrintTableVm` | contrôleur players, depuis `build_player_rows` | gabarit d'impression des joueurs |
| `SkillsAnnexVm` | service players (`ISkillCatalogPort`) | gabarit de l'annexe |
| `NextOpponentOptionVm` | contrôleur competitions | gabarit de l'option, dans la page « Imprimer » |
| `InducementsPrintVm` | service match_report (`ICompetitionDataPort` + `IReferenceCatalogPort` + `ITeamDataPort`) | gabarit des coups de pouce |
| `MatchSheetVm` | service match_report (ports ci-dessus + `IPlayerDataPort` + fonctions pures) | gabarit de la feuille de match |
| `MatchRemindersDto` | adapter references | service de la feuille de match |
| `MatchRemindersFile` | chargeur du corpus | adapter references |
