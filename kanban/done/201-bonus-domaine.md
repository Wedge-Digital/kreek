# Competitions — Domaine des bonus (agressif + seuil défensif configurable)

**Priorité : haute**
**Dépend de :** `200-bonus-label-helper.md`
**Contexte :** `src/app/competitions/domain/competition_rules.rs`, `io/web/rules_labels.rs`
**Spec :** `docs/specs/ranking-bonus-points/competition-rules-form/06-domaine.md`

## Objectif

Enrichir le domaine des règles de classement : rendre le seuil défensif
configurable, ajouter le bonus agressif, et clarifier le nommage du seuil offensif.
Aucune migration SQL (colonne `rules JSONB`).

## Conception (cf. `06-domaine.md`)

### Value objects (nutype)
- Renommer `TdDiff` → `MinTd` (bornes inchangées `1..=16`).
- `MaxTdConceded(u32)` : `<= 16` (0 autorisé).
- `MinCasualties(u32)` : `<= 16` (0 autorisé).

### Structs
- `OffensiveBonus.diff_td` → `min_td: MinTd` avec `#[serde(rename = "diff_td")]`
  (clé JSON inchangée — rétro-compat JSONB + JS front intacts).
- `DefensiveBonus` : + `#[serde(default = "default_max_td_conceded")] max_td_conceded: MaxTdConceded`.
- Nouvelle struct `AggressiveBonus { activated, points: RankingPoints, min_casualties: MinCasualties }`.
- `RankingRules` : + `#[serde(default = "default_aggressive_bonus")] aggressive_bonus: AggressiveBonus`.
- Fonctions de défaut : `default_max_td_conceded() = 1`, `default_aggressive_bonus()`
  = désactivé (`activated=false`, points=1, min_casualties=2).

### Helper label (mise à jour)
- `format_bonus_label` : seuil défensif **dynamique** (`≤ {max_td_conceded}`) + ligne
  agressif (`+{points} si > {min_casualties} sorties`) quand activé.
- Accès `.diff_td` → `.min_td` (n'existe plus que dans le helper grâce à la carte 200).

## Checklist

- [ ] VOs `MinTd` (renommé), `MaxTdConceded`, `MinCasualties` avec bornes
- [ ] `AggressiveBonus` + champ `aggressive_bonus` + serde defaults
- [ ] `#[serde(rename = "diff_td")]` sur `min_td`
- [ ] `format_bonus_label` : seuil défensif dynamique + ligne agressif
- [ ] Tests unitaires : bornes VO (0/16 ok, 17 rejet ; MinTd 1 ok, 0 rejet)
- [ ] Test : désérialisation JSON legacy sans nouveaux champs → défauts appliqués
- [ ] Test : compat clé `offensive_bonus.diff_td` → champ `min_td`
- [ ] Test : `format_bonus_label` avec les 3 bonus (dont seuils dynamiques)
- [ ] `make test` + `make check-arch` passent
