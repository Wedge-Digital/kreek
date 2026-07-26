# Phases 5 et 6 — Use cases et domaine (`detailed-standings`)

Les deux phases sont réunies dans ce fichier : l'onglet ne comporte aucune mutation, la
phase 5 se réduirait à un fichier n'énonçant qu'une absence.

## Phase 5 — Use cases

**Aucun use case créé.** L'onglet est en lecture seule : pas de commande, pas d'erreur
applicative, aucune variante ajoutée à un enum d'erreur.

Le chemin de lecture emprunte les fonctions livrées par `tiebreak-calc` :

```
handler detailed_standings_widget
   ├─ find_ranking_rules ──► standings_service::to_tiebreak_order ──► TiebreakOrder
   ├─ find_latest_lines_for_season ──► standings_service::build_ordered_standings
   ├─ domain::standings::tiebreak_outcomes                    (nouveau, cf. phase 6)
   └─ builders.rs ──► DetailedStandingsVm
```

Même conclusion que la phase 5 de `tiebreak-calc` : le classement n'est pas une mutation,
il n'a pas de use case.

## Phase 6 — Domaine

### Question reportée depuis la phase 2 — tranchée

La phase 2 avait laissé ouverte la question des **sous-groupes imbriqués**, et la phase 3
avait esquissé une API par groupe (`TiedGroup { from, len, decisive }`). Cette esquisse ne
survit pas au cas qui l'a motivée.

Trois équipes à égalité de points, différences de touchdowns `+5 / +2 / +2` : la
différence de TD « n'est pas constante » dans le groupe, elle serait donc décisive — et
mise en évidence **sur les trois lignes**, y compris les deux affichant `+2` qu'elle n'a
pas départagées. On désignerait au lecteur un critère « décisif » en pointant deux valeurs
identiques.

La maquette dessine déjà le comportement correct : lignes 4 et 5 (13 points), `Δ TD` à
`+2` des deux côtés est **grisé**, et c'est `TD+` qui est mis en évidence. La résolution
se fait donc **par sous-groupes successifs** : à chaque critère, les équipes qu'il isole
sont réglées ; celles qui restent à égalité passent au critère suivant.

Le résultat n'est pas attaché à un groupe mais à **chaque ligne**.

### API

```rust
// ranking/domain/standings.rs

/// Ce qui a décidé de la position d'une équipe parmi celles qui partagent son
/// total de points de classement.
pub enum RowTiebreak {
    /// Seule à son total : aucune égalité à résoudre.
    Alone,
    /// Départagée par le critère d'index donné dans l'ordre de priorité ; tous
    /// ceux qui le précèdent étaient égaux au sein de son sous-groupe (règle 21).
    DecidedBy(usize),
    /// Tous les critères actifs sont égaux — ex æquo assumé (règles 19 et 22).
    FullyTied,
}

/// Un élément par équipe, dans l'ordre du tableau reçu. Attend un tableau **déjà
/// ordonné** par `order_standings` : les équipes à égalité de points y sont
/// consécutives, les points étant la clé de tri primaire.
pub fn tiebreak_outcomes(ordered: &[TeamStanding], order: &TiebreakOrder) -> Vec<RowTiebreak>;
```

Le `Vec` retourné est parallèle au tableau ordonné. Le VM en déduit l'état de chaque
cellule :

| `RowTiebreak` | Colonnes avant l'index | Colonne à l'index | Colonnes après |
|---|---|---|---|
| `DecidedBy(k)` | `Tied` | `Decisive` | `Neutral` |
| `FullyTied` | `Tied` | — | `Tied` |
| `Alone` | `Neutral` | — | `Neutral` |

### Algorithme

Une descente sur les critères, par sous-groupes :

1. Découper le tableau en suites d'équipes à `ranking_points` égal. Une suite de longueur
   1 donne `Alone`.
2. Pour une suite de 2 équipes ou plus, au critère d'index `k` :
   - si `k` dépasse le dernier critère actif, toutes les équipes de la suite sont
     `FullyTied` ;
   - sinon, découper la suite en sous-suites de même valeur pour le critère `k` ;
   - une sous-suite d'une seule équipe est `DecidedBy(k)` : ce critère l'a isolée ;
   - une sous-suite de 2 équipes ou plus repart à l'étape 2 avec `k + 1` — ce critère ne
     les a pas départagées.

Le domaine dit **quel critère a tranché** ; la présentation décide d'une couleur.

Aucune évolution de `compare`, `order_standings` ou `assign_ranks` : `tiebreak_outcomes`
lit `TiebreakCriterion::value_of` sur des données déjà produites.

### Value objects et erreurs

**Aucun value object nouveau.** `Rank` et `TiebreakCriterion` existent ; `CellState` est
de la présentation et vit dans le VM (cf. `04-dtos.md`).

**Aucune variante de `DomainError`.** Comme pour le reste de `standings.rs`, il n'y a
aucun invariant à violer — seulement des valeurs à comparer.

## Récapitulatif exhaustif des règles métier

### Implémentées par cette unité

| # | Règle |
|---|---|
| **21** | Le critère mis en évidence pour une équipe est celui qui l'a séparée des équipes **encore à égalité avec elle** à ce stade de la résolution. Les critères de priorité supérieure, égaux au sein de ce sous-groupe, sont marqués comme égaux. |
| **22** | Lorsque tous les critères actifs sont égaux, aucun n'est mis en évidence et l'ex æquo est signalé comme tel. |

> **R21 a été révisée à cette phase.** Sa formulation de la phase 2 — « le premier critère
> dont les valeurs ne sont pas toutes égales au sein du groupe » — décrit une résolution
> **à plat**. Les deux formulations coïncident sur un groupe de deux équipes, ce qui a
> masqué la différence ; elles divergent dès trois équipes dont deux restent à égalité.

### Rendues visibles par cette unité, implémentées ailleurs

| # | Règle | Implémentée dans |
|---|---|---|
| 12 | Les compteurs sont accumulés pour tous les critères ; l'activation ne joue qu'à l'ordonnancement | `record_match` |
| 13 | `diff_td` est dérivé (`td_for − td_against`), jamais stocké | `value_of` |
| 17 | Sens décroissant partout sauf les TD encaissés | `direction` |
| 18 | Points d'abord, puis les critères actifs dans l'ordre jusqu'au premier qui départage | `compare` |
| 19 | Les ex æquo résiduels sont assumés, aucun départage ultime | `compare` |
| 20 | Numérotation standard après ex æquo (1, 2, 2, 4) | `assign_ranks` |

La feature compte donc **22 règles**, dont deux propres à cet onglet.

## Tests unitaires prévus

| Test | Règle |
|---|---|
| Une équipe seule à son total ⇒ `Alone` | — |
| Deux équipes séparées par le 1ᵉʳ critère ⇒ `DecidedBy(0)` des deux côtés | 21 |
| 1ᵉʳ critère égal, le 2ᵉ sépare ⇒ `DecidedBy(1)` | 21 |
| Tous les critères actifs égaux ⇒ `FullyTied` | 22 |
| **Sous-groupe** : `+5 / +2 / +2` ⇒ la 1ʳᵉ est `DecidedBy(0)`, les deux autres se résolvent au critère suivant | 21 révisée |
| Ordre de départage vide et égalité de points ⇒ `FullyTied` | 19 |
| Deux totaux de points distincts ⇒ les groupes ne se mélangent pas | 18 |

Le **test de sous-groupe** est le seul que la formulation à plat de R21 ne passerait pas :
c'est lui qui verrouille la révision, et il doit être écrit avant l'implémentation pour
qu'on vérifie qu'il échoue sur la variante naïve.
