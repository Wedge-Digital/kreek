# Phase 5 — Use cases (`tiebreak-calc`)

Deux chemins distincts, un par sens de lecture/écriture. **Aucun use case nouveau** :
l'écriture étend l'existant, la lecture passe par un domain service.

## Écriture — `record_match_ranking_use_case::execute`

Signature **inchangée** :

```rust
pub async fn execute(
    cmd:              RecordMatchRankingCommand,
    repo:             &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<(), RecordMatchRankingError>
```

C'est la **commande** qui change de forme (cf. `04-dtos.md`), pas l'orchestration, qui
reste : charger les règles via le port → charger les totaux précédents des deux équipes →
appeler `record_match` → insérer les deux lignes dans une transaction.

Trois points de modification internes :

| Fonction | Modification |
|---|---|
| `to_domain_rules` | Construit en plus le `TiebreakOrder` (cf. mapping ci-dessous) |
| `to_totals` | Reporte les 5 compteurs cumulés depuis `RankingLineRow` |
| `record_home` / `record_away` | Construisent `MatchStats` en croisant `cmd.home` et `cmd.away` — les TD adverses viennent de l'autre struct, les fautes et réussites de la sienne |

**Attention à la symétrie du croisement** : `own_td` et `opponent_td` se croisent entre
les deux équipes, mais `casualties`, `fouls` et `completions` sont **propres à l'équipe**
et ne se croisent pas. Le regroupement en `TeamMatchStats` rend l'erreur visible à la
lecture ; un test de symétrie la verrouille (phase 6).

`RecordMatchRankingError` est **inchangé** : un code de départage inconnu est sauté, pas
une erreur (décision D2 de la phase 3).

## Mapping de la configuration — où il vit

Le mapping `TiebreakSettingInfo` (DTO de port) → `TiebreakOrder` (domaine) **ne peut pas
vivre dans le domaine** : celui-ci ne connaît pas les types du port. Il ne peut pas non
plus être privé au use case d'écriture, car le chemin de lecture en a besoin aussi.

Il vit donc dans le domain service, exposé aux deux chemins :

```rust
// ranking/use_cases/standings_service.rs
pub fn to_tiebreak_order(settings: &[TiebreakSettingInfo]) -> TiebreakOrder
```

Comportement :

1. Ne garde que les entrées `activated == true`.
2. Résout chaque code via `TiebreakCriterion::from_code(&str) -> Option<Self>` — nouvelle
   fonction du domaine, symétrique de `code()`, seule autorité sur le catalogue.
3. Un code non résolu est **sauté** avec `tracing::warn!` incluant le code fautif.
4. L'ordre du `Vec` d'entrée est préservé : c'est la priorité.

Un `TiebreakOrder` vide est un état valide — l'ordonnancement se réduit alors aux points
et toutes les égalités deviennent des ex æquo.

## Lecture — `standings_service`

Le classement n'est pas une mutation : pas de use case, un **domain service** appelé par
le widget, seul point de conversion des DTOs du port vers le domaine.

```rust
pub fn build_ordered_standings(
    lines: Vec<RankingLineRow>,
    order: &TiebreakOrder,
) -> Vec<(TeamStanding, Rank)>
```

Orchestration :

1. `RankingLineRow` → `TeamStanding` (réutilise `to_totals`, à rendre partagé).
2. `order_standings` — tri par points décroissants puis critères actifs (domaine).
3. `assign_ranks` — rang = `1 + nombre d'équipes strictement devant` (règle 20).

Le service ne contient **aucune logique de comparaison** : il charge, délègue au domaine,
retourne. La règle « qui est devant ? » vit dans `standings.rs`.

### Conséquence sur `builders.rs`

`build_group_vm` reçoit désormais des `(TeamStanding, Rank)` déjà ordonnés au lieu de
`Vec<RankingLineRow>` bruts, et perd son `sort_by` (`builders.rs:106`) ainsi que sa
boucle d'attribution de rang. Il ne fait plus que résoudre le nom d'équipe, construire le
lien, et remplir le VM.

Le découpage par groupe reste dans `builders.rs` : c'est de la composition d'affichage.
**L'ordonnancement s'applique par groupe**, chaque groupe étant un classement autonome.

## Chemin d'appel complet

```
match_report_published_listener  (compte le payload → TeamMatchStats)
   └─► record_match_ranking_use_case::execute
          ├─ to_domain_rules ──► to_tiebreak_order ──► TiebreakOrder
          ├─ to_totals (+ compteurs)
          └─ RankingLine::record_match ──► insert_lines

classement_widget::build_vm
   ├─ find_ranking_rules ──► to_tiebreak_order ──► TiebreakOrder
   └─ standings_service::build_ordered_standings ──► builders (VM)
```

## Règles métier — état

Aucune règle nouvelle. La phase précise que la règle 12 (accumuler tous les compteurs)
et la règle 20 (rangs standards) se matérialisent respectivement dans `record_match` et
dans `assign_ranks`, et que le saut d'un code inconnu (D2) n'est pas une erreur
applicative.
