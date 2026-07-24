# Phase 8 — Cartes kanban (post-match-bonus-calc)

Découpage de l'implémentation en 4 cartes ordonnées par dépendance. Chaque carte est
**compilable, testable et commitable** indépendamment.

## Contrainte de découpage

Le changement de signature de `RankingLine::record_match` (`outcome` → `MatchContext`
+ `MatchStats`) casse la compilation de ses 11 appelants (2 prod + 9 tests). Le
recâblage (signature + use case + listener + adaptation des tests) est donc **atomique**
et regroupé dans une seule carte (206). Les **additions de types** (VOs, structs,
`bonus_points`), qui ne cassent rien, sont isolées en amont (205) pour être testées
seules avant le recâblage.

## Ordre & dépendances

```
204 (ACL ports+adapter) ─┐
                         ├─► 206 (câblage) ─► 207 (E2E)
205 (domaine types)    ──┘
```

- **204** et **205** sont indépendantes l'une de l'autre (peuvent être faites dans
  n'importe quel ordre), toutes deux préalables à **206**.
- **206** consomme les champs bonus de `RankingRulesInfo` (204) et les types domaine (205).
- **207** (E2E navigateur) vient en dernier, une fois la chaîne fonctionnelle.

## Cartes

| # | Carte | Portée | Comportement modifié ? |
|---|---|---|---|
| 204 | `204-ranking-bonus-acl.md` | `ports.rs` (`BonusRuleInfo` + 3 bonus sur `RankingRulesInfo`) ; adapter recopie depuis `competitions` ; fix des littéraux `RankingRulesInfo` dans les tests | Non (données transportées mais pas encore lues) |
| 205 | `205-ranking-bonus-domaine.md` | VOs bonus ; `MatchStats`, `MatchContext`, 3 `*BonusRule` ; `RankingRules` + 3 champs ; `points_for` + `bonus_points` (**additif**, `record_match` inchangé) ; tests unitaires du calcul | Non (code neuf, non appelé) |
| 206 | `206-ranking-bonus-cablage.md` | `record_match(previous, ctx, stats, rules)` (— `#[allow]`) ; use case (`to_domain_rules` mappe les bonus, construit 2 `MatchContext`/`MatchStats`, commande + casualties) ; listener (`count_sorties`) ; adaptation des 11 appelants + pipeline test | **Oui** — les bonus sont calculés de bout en bout |
| 207 | `207-ranking-bonus-e2e.md` | E2E Playwright : match avec sorties > seuil → widget classement affiche le total incluant le bonus | — (vérification) |

## Traçabilité spec → carte

- 204 ← `03-back.md` (fichiers 1-2), `04-dtos.md` (`BonusRuleInfo`/`RankingRulesInfo`), `07-integration.md` (§2).
- 205 ← `06-domaine.md` (B, C, D — parties additives), `04-dtos.md` (VOs/structs).
- 206 ← `05-use-cases.md`, `06-domaine.md` (D — `record_match`), `07-integration.md` (§3).
- 207 ← `07-integration.md` (§5, E2E).
