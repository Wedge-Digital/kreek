# Phase 8 — Cartes kanban (`competition-rules-form`)

## Découpage

| Carte | Portée | Nature |
|---|---|---|
| `208-tiebreak-catalogue-ranking` | `TiebreakCriterion` (7 variantes, ordre canonique, codes) + libellés dans `ranking` | **additive** |
| `209-tiebreak-acl-catalogue` | `ITiebreakCatalogPort` + DTO + adapter sans état + injection contexte/`main.rs` | **additive** |
| `210-tiebreak-domaine-competitions` | `domain/error.rs` + `TiebreakCode` / `TiebreakSetting` / `TiebreakConfig` + `serde(try_from)` + tests | **additive** |
| `211-tiebreak-cablage` | Bascule `additionnal_ranking_points` → `tiebreakers`, use case + port, découpe des 2 handlers, formulaire, CSS, 4 sites de test | **atomique** |
| `212-tiebreak-e2e` | Tests E2E navigateur, 4 scénarios | — |

## Ordre

```
208 ──┐
      ├──► 209 ──┐
210 ──────────────┼──► 211 ──► 212
```

`208` puis `209` (qui en dépend). `210` est indépendante et peut être menée en parallèle.
`211` exige `209` **et** `210`. `212` clôt l'unité.

## Pourquoi trois cartes additives avant une carte atomique

Les cartes 208, 209 et 210 ne changent aucun comportement : elles ajoutent du code que
personne ne consomme encore. Chacune est compilable, testable et commitable seule — trois
points de reprise sûrs.

La carte 211 est **atomique par nécessité** : remplacer `additionnal_ranking_points` par
`tiebreakers` casse simultanément l'agrégat, le use case, les deux handlers, le template
et quatre sites de test. La découper produirait un état intermédiaire non compilable.
C'est le même schéma que la feature `ranking-bonus-points` (cartes 204/205 additives,
206 atomique, 207 E2E).

## Dette payée au passage

La carte 211 embarque deux nettoyages qui ne sont pas de la feature mais que la règle
« tout fichier modifié suit les conventions » rend obligatoires :

- **Découpe des deux handlers** — `get_new_competition_phase_2` (38 lignes) et
  `post_competition_rules` (45 lignes) sont hors de la limite des 20 lignes *avant* nos
  ajouts.
- **Suppression de `.tiebreak-remove`** dans `new-competition-phase-2.css` (lignes 71-72)
  — code mort, le template n'a jamais eu de bouton ✕.

La carte 210 crée `competitions/domain/error.rs`, module d'erreurs domaine dont ce BC
était le seul dépourvu.

## Reste après cette unité

L'unité `tiebreak-calc` (règles 11 à 19) : compteurs cumulés sur la ranking line,
propagation de la configuration via l'ACL `competition_info_adapter`, comparateurs et
ordonnancement du classement. Elle démarrera à la phase 3 du workflow — pas d'UI propre,
donc pas de maquette ni de phase 2.
