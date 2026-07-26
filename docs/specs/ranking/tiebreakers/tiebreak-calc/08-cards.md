# Phase 8 — Cartes kanban (`tiebreak-calc`)

## Découpage

| Carte | Portée | Nature |
|---|---|---|
| `214-tiebreak-acl-config` | `TiebreakSettingInfo` + `RankingRulesInfo.tiebreakers` + recopie par l'adapter | **additive** |
| `215-ranking-command-team-stats` | Regroupement des stats par équipe dans la commande, **sans champ nouveau** | **refacto pure** |
| `216-tiebreak-compteurs` | Migration 5 colonnes + VOs + `MatchStats`/`CumulativeTotals` + `record_match` + repository + listener | **atomique** |
| `217-tiebreak-comparaison-domaine` | `from_code`/`direction`/`value_of` + `standings.rs` (compare, order, ranks) | **additive** |
| `218-tiebreak-cablage-lecture` | `standings_service` + `build_vm` + `builders.rs` sans `sort_by` | **atomique** |
| `219-tiebreak-e2e` | 3 scénarios navigateur | — |

## Ordre

```
214 ──────────────────────┐
                          ├──► 218 ──► 219
215 ──► 216 ──► 217 ──────┘
```

`214` et `215` sont indépendantes et peuvent démarrer en parallèle. `216` a besoin de la
commande regroupée, `217` des compteurs, `218` de la configuration **et** de la comparaison.

## Pourquoi six cartes

**`215` est une refacto pure isolée** parce que mêler un renommage de huit champs et un
ajout de logique dans le même diff rend la relecture pénible : on ne sait plus ce qui est
déplacé et ce qui est nouveau. Séparée, elle se vérifie d'un coup d'œil — aucune assertion
métier ne bouge.

**`216` et `218` sont atomiques par nécessité.** Changer la forme de `MatchStats` et de
`CumulativeTotals` casse simultanément le domaine, le repository, le listener et le use
case ; sortir le tri de `builders.rs` casse sa signature et celle de ses appelants.

**`214` et `217` sont additives** : elles ajoutent du code que personne n'appelle encore,
donc deux points de reprise sûrs. C'est le même schéma que l'unité 1 (208/209/210 additives,
211 atomique, 212 E2E) et que la feature `ranking-bonus-points`.

## Dette payée au passage

`build_classement_rows` fait **33 lignes** aujourd'hui, hors de la limite des 20. En perdant
son `sort_by` et sa boucle de rangs en carte 218, elle repasse sous la limite sans effort
dédié.

## Points de vigilance transverses

- **`.sqlx/` à régénérer** après la migration de la carte 216 (`make prepare_db`), sinon les
  macros `query_as!` ne compilent plus.
- **Base de développement** : les lignes antérieures à 216 ont leurs compteurs à 0 et
  produiront des ex æquo partout. `make reset_db` avant de tester le départage — à demander
  à l'utilisateur, jamais d'office.
- **Tests de tri de `builders.rs`** : à déplacer côté domaine en carte 218, pas à supprimer.
- **Changement d'affichage** de l'onglet Classement existant : rangs répétés, et trophée sur
  chaque équipe ex æquo au rang 1.

## Reste après cette unité

L'unité `detailed-standings` : onglet « Classement détaillé », dont la maquette est validée
et dont les deux prérequis de données seront alors satisfaits — `bonus_points` (carte 213,
livrée) et les compteurs (carte 216).
