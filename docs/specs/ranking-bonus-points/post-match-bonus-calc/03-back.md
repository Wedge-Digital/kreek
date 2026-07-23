# Phase 3 — Architecture back (post-match-bonus-calc)

Unité **sans UI** : le workflow démarre en Phase 3 (pas de maquette ni de Phase 2
front). Objectif : propager la config des bonus jusqu'au BC `ranking` et calculer
les points bonus par équipe à partir des actions du match.

## Contexte technique

- L'app event `MatchReportPublished` porte déjà `home_actions`/`away_actions`
  (variantes `Sortie`, `Touchdown`, …) + `home_score`/`away_score` (TD comptés).
  **Aucune modification du BC `match_report`.**
- Le BC `ranking` a sa **propre** `RankingRules` (copie lecture,
  `ranking_line.rs:62`), alimentée par le port ACL `RankingRulesInfo` et mappée par
  `to_domain_rules`. Aujourd'hui : win/draw/lose seulement.
- Le calcul atterrit dans `RankingLine::record_match`.

## Données nécessaires par équipe

| Bonus | Donnée | Source |
|---|---|---|
| Offensif | TD marqués ≥ seuil | `own_score` |
| Défensif | TD encaissés ≤ seuil | `opponent_score` |
| Agressif | sorties infligées > Y | compte des `Sortie` dans les actions de l'équipe |

## Plan de fichiers

| # | Fichier | Changement |
|---|---|---|
| 1 | `app/ranking/ports.rs` | Étendre `RankingRulesInfo` avec la config des 3 bonus (activation + seuils + points ; primitifs, DTO query) |
| 2 | `infrastructure/ranking/competition_info_adapter.rs` | Recopier les bonus depuis `competitions::RankingRules` dans `RankingRulesInfo` |
| 3 | `app/ranking/domain/ranking_line.rs` | Étendre `RankingRules` (VOs bonus) ; `MatchStats` ; calcul via `RankingRules::bonus_points` appelé par `record_match` (détail Phase 6) |
| 4 | `app/ranking/use_cases/record_match_ranking_use_case.rs` | Étendre `RecordMatchRankingCommand` (sorties infligées/équipe) ; `to_domain_rules` (mapping bonus) ; construire `MatchStats` et le passer à `record_match` |
| 5 | `app/ranking/io/app_events/match_report_published_listener.rs` | Compter les `Sortie` dans `home_actions`/`away_actions` et alimenter la commande |

## Décisions d'architecture (validées)

### A. Comptage des sorties dans le listener (couche IO)
Le listener traduit l'app event : il compte les `ActionTypePayload::Sortie` de
`home_actions`/`away_actions` et passe des **nombres** dans la commande. Le domaine
ne connaît jamais les types du payload (`MatchActionPublishedPayload`, etc.).
Alternative écartée : pré-agréger un `casualties_count` dans le payload — modifierait
le BC `match_report` sans nécessité (toutes les actions sont déjà transmises).

### B. `MatchStats` remplace `outcome` dans `record_match`
`record_match` reçoit un `MatchStats { own_td, opponent_td, casualties_inflicted }`
(value objects) et dérive l'outcome en interne via `derive_outcome` (existant). Ça
réduit le nombre d'arguments (`#[allow(clippy::too_many_arguments)]` peut sauter) et
co-localise toute la logique de calcul dans le domaine. Le use case ne dérive plus
l'outcome lui-même : il construit deux `MatchStats` (home / away, scores croisés).

### C. Calcul des bonus dans le domaine ranking
La règle « combien de points bonus ? » vit dans le domaine (comparaison de seuils,
gate d'activation). Helper `RankingRules::bonus_points(&MatchStats) -> RankingPoints`,
appelé par `record_match`, qui reste sous 20 lignes :
```
ranking_points = previous + match_points(outcome) + rules.bonus_points(&stats)
```

## Ports / adapters / domain services

- Port ACL existant `IRankingCompetitionPort::find_ranking_rules` réutilisé — seul son
  DTO `RankingRulesInfo` s'enrichit (pas de nouvelle méthode).
- Pas de nouveau domain service : le mapping DTO port → domaine (`to_domain_rules`)
  existe déjà dans le use case et sera étendu.
- Souveraineté des données respectée : `ranking` ne lit que via `competitions`
  (le port), jamais les tables d'un autre BC.

## Impacts hors périmètre direct (à traiter dans les cartes)

- Fixtures de test : `FakeCompetitionPort` / `RankingRulesInfo` et
  `RecordMatchRankingCommand` dans le use case.
- `test_match_report_published_pipeline.rs` (pipeline listener → projection).
- Tests de `record_match` (nouvelle signature `MatchStats`).
- Consommateur `classement_widget::build_vm` : appelle `find_ranking_rules` seulement
  pour tester `is_none()` — l'enrichissement du DTO ne le casse pas.

## Règle métier à cette étape

Aucune nouvelle (rappel Phase 6 : bonus calculé seulement si `activated`, cumulables,
indépendants du résultat, sorties = `Sortie` seule, comparateurs ≥/≤/> stricts selon
le bonus).
