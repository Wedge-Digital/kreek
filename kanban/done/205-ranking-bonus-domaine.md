# Ranking — Domaine du calcul des bonus (types + bonus_points, additif)

**Priorité : haute**
**Dépend de :** — (indépendante de 204)
**Contexte :** `src/app/ranking/domain/ranking_line.rs`
**Spec :** `docs/specs/ranking-bonus-points/post-match-bonus-calc/{06-domaine,04-dtos}.md`

## Objectif

Ajouter au domaine `ranking` les types et le calcul des 3 bonus, en **additif** :
`record_match` n'est **pas** encore modifié (recâblage en carte 206). Tout le code de
cette carte est neuf et non appelé → compile et se teste isolément. Sépare le risque du
calcul (ici, testé seul) de celui du recâblage (206).

## Conception (cf. `06-domaine.md` B, C, D)

### Value objects (style `pub` newtype, sans invariant)
`CasualtiesInflicted(u32)`, `MinTd(u32)`, `MaxTdConceded(u32)`, `MinCasualties(u32)`,
`BonusActivated(bool)`.

### Structs
- `MatchStats { own_td: MatchScore, opponent_td: MatchScore, casualties_inflicted: CasualtiesInflicted }`.
- `MatchContext { team_id, competition_id, season_id, round_id, match_report_id, recorded_at }` (`#[derive(Debug, Clone)]`).
- `OffensiveBonusRule { activated, min_td, points }`, `DefensiveBonusRule { activated, max_td_conceded, points }`, `AggressiveBonusRule { activated, min_casualties, points }`.
- `RankingRules` : + `offensive_bonus` / `defensive_bonus` / `aggressive_bonus`.

### Méthodes (chaque règle porte son comparateur, < 20 lignes)
- `OffensiveBonusRule::points_for` : `activated && own_td ≥ min_td` (large).
- `DefensiveBonusRule::points_for` : `activated && opponent_td ≤ max_td_conceded` (large).
- `AggressiveBonusRule::points_for` : `activated && casualties_inflicted > min_casualties` (**strict**).
- `RankingRules::bonus_points(&MatchStats)` = somme des 3 `points_for`.

> Ajouter les 3 champs à `RankingRules` casse ses littéraux existants (tests du module,
> `to_domain_rules`). Les compléter avec des bonus désactivés — **sans** toucher
> `record_match` ni `to_domain_rules`'s logique (mapping bonus = carte 206).

## Checklist

- [ ] VOs + structs (`MatchStats`, `MatchContext`, 3 `*BonusRule`) définis
- [ ] `RankingRules` enrichi ; littéraux existants complétés (bonus désactivés)
- [ ] `points_for` (×3) + `bonus_points` implémentés
- [ ] Tests par bonus : activé+rempli → points ; activé+non rempli → 0 ; désactivé → 0
- [ ] Tests frontières : off `==min_td` (oui), def `==max_td_conceded` (oui), agg `==min_casualties` (non) / `+1` (oui)
- [ ] Test cumul des 3 bonus + indépendance du résultat
- [ ] `make test` + `make check-arch` passent
