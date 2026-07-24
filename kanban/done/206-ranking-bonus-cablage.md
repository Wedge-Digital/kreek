# Ranking — Câblage du calcul des bonus (record_match + use case + listener)

**Priorité : haute**
**Dépend de :** `204-ranking-bonus-acl.md`, `205-ranking-bonus-domaine.md`
**Contexte :** `src/app/ranking/domain/ranking_line.rs`, `use_cases/record_match_ranking_use_case.rs`, `io/app_events/match_report_published_listener.rs`, `io/app_events/tests/test_match_report_published_pipeline.rs`
**Spec :** `docs/specs/ranking-bonus-points/post-match-bonus-calc/{05-use-cases,06-domaine,07-integration}.md`

## Objectif

Recâbler la chaîne complète pour que les bonus soient **calculés de bout en bout** :
`record_match` prend `MatchContext` + `MatchStats`, le use case mappe les bonus et
construit les stats, le listener compte les sorties. Changement **atomique** (la
nouvelle signature casse tous les appelants → tout dans un seul commit).

## Conception

### Domaine — `record_match` nouvelle signature (cf. `06-domaine.md` D)
```rust
pub fn record_match(previous: Option<CumulativeTotals>, ctx: MatchContext,
                    stats: MatchStats, rules: &RankingRules) -> RankingLine
```
- Dérive l'outcome en interne (`Self::derive_outcome(stats.own_td, stats.opponent_td)`).
- `ranking_points = previous + match_points(outcome) + rules.bonus_points(&stats)`.
- **Supprimer** `#[allow(clippy::too_many_arguments)]` (4 args).
- Adapter les 9 tests unitaires : helpers `ctx()` (→ `MatchContext`) et
  `stats(own, opp, cas)` (→ `MatchStats`) ; vérifier que `derive_outcome` interne
  produit les mêmes V/N/D qu'avant.

### Use case (cf. `05-use-cases.md`)
- `RecordMatchRankingCommand` : + `home_casualties_inflicted` / `away_casualties_inflicted: CasualtiesInflicted`.
- `to_domain_rules` : mappe les 3 `BonusRuleInfo` → règles domaine (`threshold` →
  `MinTd`/`MaxTdConceded`/`MinCasualties` selon le champ).
- `execute` : construit 2 `MatchContext` (home/away) + 2 `MatchStats` (scores/casualties
  croisés), appelle `record_match`. Ne dérive plus l'outcome lui-même.
- Adapter `sample_cmd` (fournit les casualties) + assertions bonus.

### Listener (cf. `07-integration.md` §3)
- `count_sorties(&[MatchActionPublishedPayload]) -> CasualtiesInflicted` : compte les
  `ActionTypePayload::Sortie` **seules** (fonction dédiée, `handle_published` < 20 lignes).
- Alimente la commande avec `home/away_casualties_inflicted`.

### Pipeline test (intégration, bus + PgPool réels)
- `FakeCompetitionPort` : au moins un bonus activé (ex. agressif `min_casualties=1`, `points=1`).
- `sample_payload` : injecter des actions `Sortie` (ex. 2 côté home).
- Assertion : `home_row.ranking_points == points_victoire + bonus`.

## Checklist

- [ ] `record_match` : signature `MatchContext`+`MatchStats`, outcome dérivé, bonus ajoutés, `#[allow]` retiré
- [ ] 9 tests unitaires `record_match` adaptés (helpers `ctx()`/`stats()`)
- [ ] Commande enrichie ; `to_domain_rules` mappe les 3 bonus
- [ ] `execute` construit 2 `MatchContext` + 2 `MatchStats` croisés
- [ ] `count_sorties` dans le listener ; commande alimentée
- [ ] Pipeline test étendu (actions `Sortie` + bonus configuré → points vérifiés)
- [ ] `make test` + `make check-arch` passent
