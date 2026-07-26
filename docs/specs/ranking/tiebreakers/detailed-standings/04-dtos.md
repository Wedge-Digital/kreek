# Phase 4 — Contrats de données (`detailed-standings`)

Entrée : `03-back.md` validé. L'onglet ne mute rien — il n'y a donc **aucune commande**,
et par conséquent aucun value object d'entrée à définir.

## DTO d'entrée

```rust
Path<(String, String, String)>   // space_id, competition_id, season_id
```

Extrait par Axum, pour les deux handlers (coquille d'onglet côté `competitions`, widget
côté `ranking`). Rien d'autre à désérialiser : ni body, ni query string.

Les primitives sont ici admises par la règle CQRS du CLAUDE.md — on est côté lecture
(query), et ces identifiants ne portent aucun invariant à protéger à ce stade. Le handler
de la widget les convertit là où le domaine l'exige, comme le fait déjà
`classement_widget`.

## DTOs de sortie — view models

```rust
// ranking/io/web/widgets/detailed_standings_widget.rs

pub struct DetailedStandingsVm {
    pub rules_missing: bool,
    /// En-têtes des colonnes de départage — **partagés par toutes les poules**.
    pub columns: Vec<TiebreakColumnVm>,
    pub groups: Vec<DetailedGroupVm>,
}

pub struct TiebreakColumnVm {
    /// Numérotation affichée (1, 2, 3…) — la priorité, pas l'index du catalogue.
    pub position: u32,
    /// « Δ TD » — en-tête de colonne.
    pub short_label: &'static str,
    /// « Différence de touchdowns (marqués − encaissés) » — attribut `title`.
    pub long_label: &'static str,
}

pub struct DetailedGroupVm {
    pub title: Option<String>,
    pub has_enrolled_teams: bool,
    pub rows: Vec<DetailedRowVm>,
}

pub struct DetailedRowVm {
    pub rank: u32,
    pub team_name: String,
    pub team_link: String,
    pub played: u32,
    pub wins: u32,
    pub draws: u32,
    pub losses: u32,
    /// Déjà formaté et signé (« +2 », « +0 ») — cf. formatage ci-dessous.
    pub bonus: String,
    pub total: u32,
    /// Une cellule par colonne de `DetailedStandingsVm::columns`, dans le même ordre.
    pub tiebreaks: Vec<TiebreakCellVm>,
}

pub struct TiebreakCellVm {
    /// Déjà formatée — « +14 », « −3 », « 24 ».
    pub value: String,
    pub state: CellState,
}

/// État d'une cellule de départage vis-à-vis du groupe d'équipes à égalité de
/// points auquel appartient sa ligne (règles 21 et 22).
pub enum CellState {
    /// Le critère qui a départagé le groupe.
    Decisive,
    /// Critère de priorité supérieure au décisif, ou groupe totalement ex æquo :
    /// toutes les équipes du groupe y ont la même valeur.
    Tied,
    /// Aucune égalité à résoudre pour cette ligne, ou critère situé après le décisif.
    Neutral,
}
```

### Trois choix de structure

**`columns` est porté par le VM racine, pas par chaque groupe.** L'ordre de départage est
celui de la compétition : il est identique pour toutes les poules. Le dupliquer par groupe
ouvrirait la possibilité d'en-têtes divergents d'une poule à l'autre, ce qui n'a aucun
sens métier. Askama accède à `vm.columns` depuis l'intérieur de la boucle sur les groupes.

**`CellState` est un enum, pas une chaîne de classe CSS.** Le VM nomme l'état sémantique ;
une méthode `CellState::css_class()` fait la correspondance en un seul endroit. Stocker
`"sd-decisive"` directement dans le VM enfouirait des noms de classes CSS dans du Rust,
disséminés dans le builder — et rendrait un renommage CSS invisible au compilateur.

**Pas de champ `is_leader`.** Le trophée se décide par `rank == 1` dans le template,
exactement comme le fait déjà `classement-widget.html`. Un booléen redondant finirait tôt
ou tard par diverger du rang qu'il est censé refléter.

## Formatage

| Colonne | Format | Motif |
|---|---|---|
| **Bonus** | toujours signé — `+2`, `+0` | Se lit comme une contribution au total, pas comme une valeur autonome |
| **Δ TD** (`diff_td`) | signé — `+14`, `−3` | Seul critère pouvant être négatif |
| Autres critères | brut — `24`, `11` | Ce sont des dénombrements, toujours positifs |
| Rang, MJ, G, N, D, Total | brut | — |

**Le signe moins est le signe typographique `−` (U+2212), pas le trait d'union ASCII
`-`.** C'est ce qu'utilise la maquette. Écrit ici parce que c'est exactement le genre de
détail qui se dégrade silencieusement à la première réécriture.

Le formatage vit dans le VM et non dans le template : `value_of` retourne un `i64` nu et
le domaine n'a pas à connaître de convention d'affichage, mais dupliquer la mise en forme
dans chaque cellule du template serait pire.

## DTOs de port

**Aucun DTO nouveau.** Les quatre existants couvrent le besoin :

| DTO | Fournit |
|---|---|
| `RankingLineRow` | Compteurs cumulés, points, part bonus |
| `EnrolledTeamInfo` | Nom d'équipe |
| `RankingGroupInfo` | Poules et leurs équipes |
| `RankingRulesInfo` | Barème et configuration de départage (`tiebreakers`) |

## Interfaces d'utilisation

Qui produit, qui consomme — obligatoire à cette phase :

| DTO | Émis par | Consommé par |
|---|---|---|
| `Path<(space_id, competition_id, season_id)>` | Extracteur Axum | Handlers `detailed_standings_widget` et `get_tab_detailed_standings` |
| `RankingRulesInfo` | Adapter `IRankingCompetitionPort` | Handler → `standings_service::to_tiebreak_order` |
| `EnrolledTeamInfo`, `RankingGroupInfo` | Adapter `IRankingCompetitionPort` | Handler → `builders.rs` |
| `RankingLineRow` | `PgRankingRepository` | `standings_service` |
| `(TeamStanding, Rank)` | `standings_service::build_ordered_standings` | `builders.rs` |
| `TiedGroup` | `domain/standings.rs::tied_groups` | `builders.rs` |
| `DetailedStandingsVm` et ses composants | `builders.rs` | Template Askama de la widget |
| `DetailedStandingsTabTemplate` | Handler `get_tab_detailed_standings` (`competitions`) | Navigateur, en fragment HTMX |

**Aucun DTO de port n'atteint le template** : `builders.rs` est le seul point de
traduction port → VM, conformément à la règle « Domain services pour données inter-BCs »
du CLAUDE.md.

## Règles métier

**Aucune règle nouvelle à cette étape.** Le formatage signé et le trophée relèvent de la
présentation, pas du métier. Les règles 21 et 22 identifiées en phase 2 se matérialisent
ici dans `CellState`, dont les trois variantes en sont la traduction directe.
