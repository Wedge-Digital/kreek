# Phase 3 — Architecture back (`tiebreak-calc`)

Unité sans UI propre : le workflow démarre en phase 3. L'affichage détaillé des
compteurs relève de l'unité `detailed-standings`.

## Constats de départ

1. **L'ordonnancement vit dans la couche présentation.** `builders.rs:106` fait
   `rows.sort_by(|a, b| b.points.cmp(&a.points))`. Ordonner selon des critères
   configurés répond à « qui est devant ? » — de la logique métier.
2. **Tous les compteurs sont calculables.** Le payload `MatchReportPublished` porte
   `Agression` et `Passe` (`shared_kernel/app_events/match_report_app_events.rs:58`),
   comme il portait déjà `Sortie` pour le bonus agressif.
3. **Les règles sont déjà chargées à l'affichage.** `classement_widget::build_vm`
   appelle `find_ranking_rules(season_id)` : la configuration de départage arrivera par
   ce chemin, **aucun port nouveau**.
4. `bonus_points` est déjà séparé du total (carte 213).

## Chaîne à câbler

```
competitions::RankingRules.tiebreakers
   → competition_info_adapter (ACL)   recopie vers RankingRulesInfo
   → to_domain_rules                  codes → TiebreakCriterion
   → record_match                     accumule les 5 compteurs
   → ranking_lines (5 colonnes)       persistance
   → standings_service + domaine      comparaison et rangs
   → builders                         mapping VM uniquement
```

## Décisions

### D1 — L'ordonnancement descend dans le domaine

Un type domaine dédié porte une équipe complète ; une fonction de comparaison
consomme la configuration de départage ; un **domain service** dans `use_cases/`
construit ces objets à partir des DTOs du port.

```rust
// ranking/domain/standings.rs
pub struct TeamCounters { td_for, td_against, casualties, fouls, completions }
pub struct TeamStanding { team_id, totals: CumulativeTotals, counters: TeamCounters }
pub struct TiebreakOrder(Vec<TiebreakCriterion>);   // actifs, dans l'ordre de priorité

pub fn order_standings(standings: &mut [TeamStanding], order: &TiebreakOrder)
pub fn assign_ranks(standings: &[TeamStanding], order: &TiebreakOrder) -> Vec<Rank>
```

`builders.rs` ne fait plus que du mapping VM : il reçoit une liste déjà ordonnée et des
rangs déjà attribués. Le tri devient testable unitairement, sans HTTP ni base.

**Valeurs de comparaison signées.** `diff_td = td_for − td_against` peut être négatif :
la valeur exposée pour la comparaison est un entier **signé**, alors que les compteurs
persistés sont non signés.

**Sens de comparaison porté par le critère** — `TiebreakCriterion::direction()`
(décroissant partout sauf `nb_td_conceded`, cf. README) et
`TiebreakCriterion::value_of(&TeamCounters, &CumulativeTotals) -> i64`. Le sens vit avec
le critère, pas dans le comparateur : ajouter un critère ne demande pas de toucher au
tri.

### D2 — Un code inconnu du catalogue est ignoré, avec un log

`to_domain_rules` mappe les codes persistés vers `TiebreakCriterion`. Un code absent du
catalogue (renommage, donnée trafiquée) est **sauté** avec un `tracing::warn!`, les
critères suivants s'appliquent. Un code obsolète ne doit pas priver toute la ligue de
son classement.

Conséquence : si **aucun** code n'est reconnu, l'ordre se réduit aux points, et les
égalités restent des ex æquo.

## Plan de fichiers

| Fichier | Nature | Rôle |
|---|---|---|
| `ranking/domain/standings.rs` | **nouveau** | `TeamCounters`, `TeamStanding`, `TiebreakOrder`, comparaison, attribution des rangs |
| `ranking/domain/tiebreak.rs` | modifié | `direction()` et `value_of()` sur `TiebreakCriterion` |
| `ranking/domain/ranking_line.rs` | modifié | `MatchStats` + `fouls`/`completions` ; `CumulativeTotals` + 5 compteurs ; `record_match` les accumule |
| `ranking/ports.rs` | modifié | `RankingLineRow` + 5 compteurs ; `RankingRulesInfo` + `tiebreakers: Vec<TiebreakSettingInfo>` |
| `infrastructure/ranking/competition_info_adapter.rs` | modifié | recopie la configuration depuis `competitions::RankingRules.tiebreakers` |
| `ranking/use_cases/standings_service.rs` | **nouveau** | Domain service : `RankingLineRow` (port) → `Vec<TeamStanding>` ordonnés + rangs |
| `ranking/use_cases/record_match_ranking_use_case.rs` | modifié | `to_totals` reporte les compteurs ; `to_domain_rules` mappe la configuration |
| `ranking/io/app_events/match_report_published_listener.rs` | modifié | `count_agressions`, `count_passes` (sur le modèle de `count_sorties`) |
| `ranking/io/repository/ranking_repository.rs` | modifié | 5 colonnes dans les deux SELECT et l'INSERT |
| `ranking/io/web/builders.rs` | modifié | consomme les standings ordonnés ; plus de `sort_by` |
| `migrations/<ts>_ranking_lines_tiebreak_counters.sql` | **nouveau** | 5 colonnes `NOT NULL DEFAULT 0` |

## Compteurs

| Compteur | Source | Persisté |
|---|---|---|
| `td_for` | `MatchStats.own_td` | oui |
| `td_against` | `MatchStats.opponent_td` | oui |
| `casualties` | `MatchStats.casualties_inflicted` (`Sortie` strictement) | oui |
| `fouls` | actions `Agression` — nouveau `count_agressions` | oui |
| `completions` | actions `Passe` — nouveau `count_passes` | oui |
| `diff_td` | `td_for − td_against` | **non, dérivé** |
| `nb_wins` | `CumulativeTotals.wins` (existant) | déjà là |

Les compteurs sont accumulés **pour tous les critères, indépendamment de
l'activation** (règle 12) : le calcul reste découplé de la configuration et la
projection rejouable.

## Domain services

`standings_service` est le seul point de conversion `RankingLineRow` → `TeamStanding`.
Aucun handler ni template ne manipule les DTOs du port pour ordonner, conformément à la
règle « Domain services pour données inter-BCs » du CLAUDE.md.

## Ports

Aucun port nouveau. `RankingRulesInfo` s'enrichit :

```rust
pub struct TiebreakSettingInfo { pub code: String, pub activated: bool }
// RankingRulesInfo : + pub tiebreakers: Vec<TiebreakSettingInfo>   // ordonnés
```

Primitives assumées : DTO de lecture.

## Événements

Aucun. Le classement est une projection alimentée par un app event existant
(`MatchReportPublished`) ; la configuration est consultée via l'ACL. Rien de nouveau à
publier.

## Changement de comportement à assumer

L'onglet **Classement existant** change d'affichage : aujourd'hui `rank = idx + 1`
attribue les rangs 2 et 3 à deux équipes à égalité. La règle 19 impose l'ex æquo — même
rang affiché quand tous les critères actifs sont égaux. Ce n'est donc pas un ajout
invisible.

## Règles métier — règle 20, validée en phase 3

**Numérotation après un ex æquo : rangs standards.** Deux équipes qui partagent le rang 2
sont suivies d'une 4ᵉ, pas d'une 3ᵉ — on saute autant de rangs qu'il y a d'ex æquo
(1, 2, 2, 4). Convention sportive courante.

`assign_ranks` en découle : le rang d'une équipe est `1 + le nombre d'équipes
strictement devant elle`, ce qui produit la numérotation standard sans cas particulier.
