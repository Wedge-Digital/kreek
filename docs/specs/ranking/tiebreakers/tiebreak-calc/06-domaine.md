# Phase 6 — Domaine (`tiebreak-calc`)

## Récapitulatif exhaustif des règles métier — validé

La feature compte **20 règles**. Les 19 premières sont listées dans
`../competition-rules-form/06-domaine.md` ; la 20ᵉ a été tranchée en phase 3 de cette
unité.

Règles implémentées par **cette** unité :

| # | Règle | Où elle atterrit |
|---|---|---|
| 11 | Un compteur cumulé par critère et par équipe, mis à jour à chaque match publié | `CumulativeTotals` + `record_match` |
| 12 | Compteurs accumulés pour **tous** les critères ; l'activation ne joue qu'à l'ordonnancement | `record_match` |
| 13 | `diff_td` dérivé (`td_for − td_against`), pas stocké | `value_of` |
| 14 | `nb_cas` = actions `Sortie` strictement | listener (déjà en place) |
| 15 | `nb_reu` = actions `Passe` uniquement | `count_passes` |
| 16 | `nb_fouls` = actions `Agression` | `count_agressions` |
| 17 | Sens décroissant partout sauf `nb_td_conceded` | `direction()` |
| 18 | `ranking_points` d'abord, puis les critères actifs dans l'ordre jusqu'au premier départage | `compare` |
| 19 | Ex æquo résiduels assumés, aucun départage ultime | `compare` renvoie `Equal` |
| 20 | Numérotation standard après ex æquo (1, 2, 2, 4) | `assign_ranks` |

Règles 9 et 10 (catalogue possédé par `ranking`, 7 critères sans les cartons rouges) :
implémentées depuis la carte 208. Règles 1 à 8 : côté saisie, livrées.

## Aucune erreur domaine

Ni `compare`, ni `order_standings`, ni `assign_ranks`, ni l'accumulation des compteurs ne
peuvent échouer : il n'y a pas d'invariant à violer, seulement des valeurs à comparer.
**Aucune variante ajoutée à un `DomainError`**, contrairement à l'unité 1.

## Value objects

```rust
pub struct TdFor(pub u32);
pub struct TdAgainst(pub u32);
pub struct CasualtiesTotal(pub u32);   // cumul, distinct de CasualtiesInflicted (par match)
pub struct FoulsCommitted(pub u32);
pub struct CompletionsMade(pub u32);
pub struct Rank(pub u32);
```

Les compteurs cumulés portent des types distincts de leurs équivalents par match : le
compilateur empêche de confondre « sorties de ce match » et « sorties de la saison ».

## `tiebreak.rs` — trois fonctions ajoutées

```rust
pub enum Direction { Asc, Desc }

impl TiebreakCriterion {
    /// Symétrique de `code()`. Seule autorité de résolution du catalogue.
    pub fn from_code(code: &str) -> Option<Self>;

    /// Règle 17. Décroissant partout sauf NbTdConceded : le moins est le mieux.
    pub fn direction(&self) -> Direction;

    /// Valeur comparable du critère pour une équipe. **`i64` et non `u32`** :
    /// `diff_td` peut être négatif, et un underflow non signé transformerait −3
    /// en un nombre gigantesque, plaçant la pire équipe en tête.
    pub fn value_of(&self, totals: &CumulativeTotals) -> i64;
}
```

| Critère | `value_of` | `direction` |
|---|---|---|
| `DiffTd` | `td_for − td_against` (signé) | Desc |
| `NbTd` | `td_for` | Desc |
| `NbTdConceded` | `td_against` | **Asc** |
| `NbCas` | `casualties` | Desc |
| `NbWins` | `wins` | Desc |
| `NbFouls` | `fouls` | Desc |
| `NbReu` | `completions` | Desc |

## `standings.rs` — nouveau

```rust
pub struct TeamStanding { pub team_id: TeamId, pub totals: CumulativeTotals }
pub struct TiebreakOrder(Vec<TiebreakCriterion>);   // actifs, ordre de priorité

/// Règle 18 : points d'abord, puis chaque critère actif jusqu'au premier qui
/// départage. Règle 19 : `Equal` si tous sont égaux.
pub fn compare(a: &TeamStanding, b: &TeamStanding, order: &TiebreakOrder) -> Ordering;

/// Tri **stable** (`sort_by`) : deux équipes strictement ex æquo conservent leur
/// ordre d'entrée. Sans cette garantie, le classement pourrait permuter d'un
/// affichage à l'autre sans qu'aucun match ait été joué.
pub fn order_standings(standings: &mut [TeamStanding], order: &TiebreakOrder);

/// Règle 20. « Même rang que le précédent si `compare` renvoie `Equal`, sinon
/// `idx + 1` » — produit la numérotation standard sans cas particulier ni
/// compteur d'ex æquo.
pub fn assign_ranks(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<Rank>;
```

Un `TiebreakOrder` vide est un état valide : l'ordre se réduit aux points et toutes les
égalités deviennent des ex æquo.

## `ranking_line.rs` — extensions

`MatchStats` gagne `fouls: FoulsCommitted` et `completions: CompletionsMade`.
`CumulativeTotals` gagne les 5 compteurs, `ZERO` est complété.

`record_match` accumule, sans condition d'activation (règle 12) :

```rust
td_for:      TdFor(td_for.0 + u32::from(stats.own_td.0)),
td_against:  TdAgainst(td_against.0 + u32::from(stats.opponent_td.0)),
casualties:  CasualtiesTotal(casualties.0 + stats.casualties_inflicted.0),
fouls:       FoulsCommitted(fouls.0 + stats.fouls.0),
completions: CompletionsMade(completions.0 + stats.completions.0),
```

`diff_td` n'apparaît pas : dérivé (règle 13).

## Tests unitaires prévus

### `tiebreak.rs`

| Test | Règle |
|---|---|
| `from_code` résout les 7 codes et renvoie `None` sur un code inconnu | D2 |
| `from_code` est l'inverse exact de `code()` pour les 7 critères | 9 |
| `direction` : `NbTdConceded` est le seul `Asc` | 17 |
| `value_of` de chaque critère lit le bon compteur (7 cas) | 11 |
| `value_of(DiffTd)` renvoie une valeur **négative** quand l'équipe encaisse plus qu'elle ne marque | 13 |

### `standings.rs`

| Test | Règle |
|---|---|
| Les points priment sur tous les critères, même défavorables | 18 |
| À égalité de points, le premier critère départage | 18 |
| Premier critère égal ⇒ le deuxième départage | 18 |
| Tous les critères égaux ⇒ `Equal` (ex æquo) | 19 |
| `TiebreakOrder` vide ⇒ seuls les points comptent | D2 |
| `NbTdConceded` : l'équipe qui encaisse **le moins** passe devant | 17 |
| `order_standings` est stable : deux ex æquo gardent leur ordre d'entrée | — |
| `assign_ranks` produit 1, 2, 2, 4 | 20 |
| `assign_ranks` : toutes les équipes ex æquo ⇒ toutes au rang 1 | 19, 20 |
| `assign_ranks` sur une seule équipe ⇒ rang 1 | — |

### `ranking_line.rs`

| Test | Règle |
|---|---|
| Les 5 compteurs s'accumulent sur plusieurs matchs | 11 |
| Les compteurs s'accumulent même quand aucun bonus n'est activé | 12 |
| **Symétrie du croisement** : sur un match 2-1, `td_for`/`td_against` se croisent entre les deux équipes, mais `fouls`, `completions` et `casualties` restent ceux de chaque équipe | 11 |

Le test de symétrie est le plus important de la liste : inverser `cmd.home.fouls` et
`cmd.away.fouls` compile sans broncher et produit des compteurs plausibles mais faux.

## Règles métier — état

Aucune règle nouvelle à cette étape.
