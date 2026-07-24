# Phase 7 — Effets de bord (post-match-bonus-calc)

Unité **sans UI** : pas de handler HTTP ni de template Askama à créer. Les effets de
bord se concentrent sur l'ACL (adapter), le listener (comptage des sorties) et la
persistance (inchangée). Cette phase est de la conception, pas de l'implémentation.

## 1. Persistance — INCHANGÉE (aucune migration)

Les points bonus se **fondent dans `ranking_points`** (`points_résultat + Σ bonus`,
phase 6). La ligne de classement ne gagne aucun champ.

- `RankingLine` (domaine) : struct **inchangée**.
- `RankingLineRow` (DTO repository) : **inchangé**.
- `IRankingRepository::insert_lines` / SQL de la projection : **inchangés**.
- **Aucune migration** `migrations/`.

C'est un choix délibéré : le classement affiche un total de points ; le détail
« combien viennent des bonus » n'est pas un besoin de cette unité (il pourra se
reconstruire depuis l'event store `match_report` si un jour on veut le ventiler).

## 2. Adapter ACL — recopie des 3 bonus

`infrastructure/ranking/competition_info_adapter.rs` : `find_ranking_rules` enrichit
`RankingRulesInfo` avec les 3 bonus lus sur `competitions::domain::RankingRules`
(livrés par l'unité 1 : `offensive_bonus`/`defensive_bonus`/`aggressive_bonus`, VOs
nutype avec `.into_inner()`).

Mapping (chaque bonus → `BonusRuleInfo { activated, threshold, points }`) :

| Bonus | `threshold` source (competitions) |
|---|---|
| offensive | `offensive_bonus.min_td` (clé JSON `diff_td` préservée côté competitions) |
| defensive | `defensive_bonus.max_td_conceded` |
| aggressive | `aggressive_bonus.min_casualties` |

Souveraineté respectée : `ranking` lit **uniquement** via `competitions` (le port),
jamais une table d'un autre BC. `RankingCompetitionAdapter` est le seul point qui
importe `competitions`.

## 3. Événements — comptage des sorties dans le listener

`io/app_events/match_report_published_listener.rs` (`handle_published`) est le seul
endroit qui touche aux actions du payload (décision A, phase 3).

- Compter les `ActionTypePayload::Sortie` (variante **seule**, pas `Blesse`, pas
  `Agression`) dans `payload.home_actions` → `home_casualties_inflicted`, idem
  `away_actions` → `away_casualties_inflicted`.
- Alimenter la commande enrichie `RecordMatchRankingCommand` avec ces deux
  `CasualtiesInflicted`.
- Extraire le comptage dans une petite fonction dédiée (`count_sorties(&[MatchActionPublishedPayload]) -> CasualtiesInflicted`)
  pour tenir la règle des 20 lignes de `handle_published`.

Le domaine ne voit jamais `MatchActionPublishedPayload`/`ActionTypePayload` — il reçoit
un compteur déjà agrégé.

**Pas de nouvel événement.** `ranking` est une projection append-only : la commande
insère deux lignes, il n'émet aucun domain event ni app event (aucun BC n'écoute
`ranking`). Rien à câbler dans un publisher.

## 4. Handlers / Templates — RIEN

- Aucun handler HTTP : unité sans UI.
- Aucun template : idem.
- **Consommateur existant non impacté** : `classement_widget::build_vm` appelle
  `find_ranking_rules` seulement pour un test `is_none()` — l'enrichissement du DTO
  (nouveaux champs) ne casse ni la compilation ni le comportement. Le widget affiche
  déjà `ranking_points`, qui inclut désormais les bonus, sans changement de template.

## 5. Tests d'intégration & E2E

### Test pipeline d'intégration (bus réel + PgPool réelle) — étendu

`io/app_events/tests/test_match_report_published_pipeline.rs` :
- `FakeCompetitionPort` renvoie des `RankingRulesInfo` avec au moins un bonus activé
  (ex. agressif `min_casualties = 1`, `points = 1`).
- `sample_payload` : injecter des actions `Sortie` (ex. 2 côté home) dans
  `home_actions`.
- Assertion : `home_row.ranking_points == points_victoire + points_bonus` (ex. 3 + 1),
  prouvant la chaîne complète listener → comptage → use case → domaine → projection.

### E2E navigateur (Playwright) — via le widget classement

Scénario réaliste (le seul rendu visible de cette unité) :
1. Compétition avec un bonus activé (ex. agressif, seuil bas).
2. Publier un rapport de match comportant des sorties au-delà du seuil pour une équipe.
3. Ouvrir l'onglet Classement (widget ranking) et vérifier que le total de points de
   l'équipe **inclut** le point bonus (total = V/N/D + bonus).

> Rappel `tests/e2e/README.md` : nécessite le serveur dev lancé (`make e2e`). Ne pas
> démarrer le serveur soi-même — l'utilisateur le gère.

## Règle métier à cette étape

Aucune nouvelle. Rappel du filtrage IO : « Sortie » = `ActionTypePayload::Sortie`
seule ; le comparateur agressif est strict (`>`), appliqué dans le domaine (phase 6).
