# Départages — Câblage du classement ordonné

**Priorité : haute**
**Dépend de :** cartes 214 (config via ACL) **et** 217 (comparaison domaine)
**Contexte :** `src/app/ranking/use_cases/standings_service.rs` (nouveau), `src/app/ranking/use_cases/record_match_ranking_use_case.rs`, `src/app/ranking/io/web/widgets/classement_widget.rs`, `src/app/ranking/io/web/builders.rs`
**Spec :** `docs/specs/ranking/tiebreakers/tiebreak-calc/{05-use-cases,07-integration}.md`

## Objectif

Brancher l'ordonnancement sur le classement affiché. **C'est la carte qui rend le
départage visible** — et celle qui change le comportement de l'onglet Classement existant.

Carte **atomique** : sortir le tri de `builders.rs` casse sa signature, celle de
`build_group_vm` et l'appel du widget.

## Conception

### 1. `standings_service.rs` — nouveau (cf. `05-use-cases.md`)

```rust
/// Mapping DTO de port → domaine. Ne peut pas vivre dans le domaine (il ignore les
/// types du port) ni rester privé au use case d'écriture (la lecture en a besoin).
pub fn to_tiebreak_order(settings: &[TiebreakSettingInfo]) -> TiebreakOrder;

pub fn build_ordered_standings(
    lines: Vec<RankingLineRow>,
    order: &TiebreakOrder,
) -> Vec<(TeamStanding, Rank)>;
```

`to_tiebreak_order` : ne garde que `activated == true`, résout via
`TiebreakCriterion::from_code`, **saute** un code inconnu avec `tracing::warn!` incluant le
code fautif (décision D2), préserve l'ordre d'entrée.

`build_ordered_standings` : convertit, appelle `order_standings` puis `assign_ranks`.
**Aucune logique de comparaison** dans le service — il charge, délègue, retourne.

`to_domain_rules` (use case d'écriture) appelle la même `to_tiebreak_order` : un seul
chemin de mapping pour les deux sens.

### 2. `classement_widget::build_vm`

Fait **17 lignes** ; il charge déjà les règles via `find_ranking_rules`. Pour rester sous
20 lignes, la construction de l'ordre est extraite :

```rust
fn tiebreak_order_of(rules: &Option<RankingRulesInfo>) -> TiebreakOrder
```

Aucune route, aucun widget, aucun endpoint nouveau.

### 3. `builders.rs`

| Fonction | Changement |
|---|---|
| `build_classement_rows` (`:79`, **33 lignes**) | Reçoit des `(TeamStanding, Rank)` ordonnés ; **supprime** le `sort_by` (`:106`) et la boucle de rangs. Repasse sous 20 lignes — dette préexistante résorbée |
| `build_group_vm`, `build_classement_groups` | Signature adaptée ; découpage par groupe **inchangé** |

L'ordonnancement s'applique **par groupe** : chaque groupe est un classement autonome.

Les tests existants de `builders.rs` qui vérifiaient le tri et les rangs
(`sorts_by_points_descending_and_assigns_rank`,
`multiple_groups_split_classement_and_rank_independently_per_group`) doivent être
**déplacés ou réécrits** : le tri n'est plus de leur ressort. Ne pas les supprimer sans que
le comportement soit couvert côté domaine (carte 217).

## Changements d'affichage à assumer (cf. `07-integration.md`)

1. Des rangs **identiques sur des lignes consécutives** (1, 2, 2, 4) — règle 20.
2. `classement-widget.html:39` affiche `{% if row.rank == 1 %}🏆{% endif %}` : deux équipes
   ex æquo au rang 1 recevront **chacune** le trophée. Correct, mais c'est un changement.

Rien à modifier dans le template.

## Attention en développement

Les lignes déjà en base ont leurs compteurs à 0 (migration de la carte 216 sans backfill) :
deux équipes à égalité de points y seront ex æquo sur tous les critères, quels que soient
les matchs joués. Un `make reset_db` avant de tester évite de conclure à un bug de
comparaison. **Ne pas réinitialiser la base de l'utilisateur sans le lui demander.**

## Checklist

- [ ] `standings_service.rs` avec `to_tiebreak_order` et `build_ordered_standings`
- [ ] Code inconnu sauté avec `warn!`, vérifié par un test
- [ ] `to_domain_rules` réutilise `to_tiebreak_order` (un seul chemin de mapping)
- [ ] `build_vm` ≤ 20 lignes, ordre extrait dans `tiebreak_order_of`
- [ ] `builders.rs` sans `sort_by` ni boucle de rangs, `build_classement_rows` ≤ 20 lignes
- [ ] Tests de tri de `builders.rs` déplacés côté domaine, pas supprimés
- [ ] Ordonnancement appliqué par groupe
- [ ] `make test` + `make check-arch` passent
