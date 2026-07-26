# Ranking — Séparation des points bonus dans la ranking line

**Priorité : haute**
**Dépend de :** —
**Contexte :** `src/app/ranking/domain/ranking_line.rs`, `src/app/ranking/ports.rs`, `src/app/ranking/io/repository/ranking_repository.rs`, `src/app/ranking/use_cases/record_match_ranking_use_case.rs`, `migrations/`
**Spec :** `docs/specs/ranking/tiebreakers/README.md` § unité `detailed-standings`

## Objectif

Conserver les points bonus **cumulés** à part du total de points de classement.
Prérequis de l'unité `detailed-standings`, dont la colonne `Bonus` n'a aujourd'hui
aucune source de données.

**Additif** : le total reste identique, aucun ordre de classement ne change, aucun
affichage n'est modifié.

## Pourquoi

`RankingLine::record_match` (`ranking_line.rs:247`) calcule
`points + match_points + rules.bonus_points(&stats)` et ne persiste que la somme. La
table `ranking_lines` n'a pas de colonne pour les bonus : l'information est perdue à
l'écriture et ne peut pas être reconstituée à l'affichage.

## Conception

### 1. Migration

Nouveau fichier `migrations/<ts>_ranking_lines_bonus_points.sql` :

```sql
ALTER TABLE ranking_lines ADD COLUMN bonus_points INTEGER NOT NULL DEFAULT 0;
```

Un `ALTER` plutôt qu'une modification de
`20260723000001_create_ranking_lines.sql` : sqlx valide le checksum des migrations
déjà appliquées, éditer le fichier d'origine ferait échouer `sqlx migrate run` sur
toute base existante.

Le projet n'étant pas en production, **aucun backfill n'est prévu** : les lignes
existantes gardent `bonus_points = 0`. La projection est de toute façon rebuildable
depuis l'event store.

### 2. Domaine

`bonus_points: RankingPoints` ajouté à **`RankingLine`** (ligne du match) et à
**`CumulativeTotals`** (cumul, y compris `ZERO`).

`record_match` calcule le bonus **une seule fois** dans une variable locale, puis
l'utilise deux fois — ajouté au total, et accumulé séparément :

```rust
let bonus = rules.bonus_points(&stats);
// …
ranking_points: points + match_points + bonus,
bonus_points:   bonus_total + bonus,
```

Une seule source de calcul : les deux champs ne peuvent pas divergier.

### 3. Port et repository

- `bonus_points: u32` sur `RankingLineRow` (`ranking/ports.rs:10`).
- Colonne ajoutée aux deux `SELECT` et à l'`INSERT` de `ranking_repository.rs`
  (`find_latest_line_for_team`, `find_latest_lines_for_season`, `insert_lines`).
- `to_totals` (`record_match_ranking_use_case.rs:106`) reporte le champ dans les
  totaux, sinon le cumul repartirait de zéro à chaque match.

### 4. Hors périmètre

Aucun affichage. La colonne `Bonus` de l'onglet « Classement détaillé » arrive avec
l'unité `detailed-standings`. Le widget Classement existant n'est pas touché.

## Invariant à verrouiller par test, pas en base

`bonus_points ≤ ranking_points`, et le total reste égal aux points de résultat plus les
bonus. Vérifié par test unitaire plutôt que par une contrainte SQL : la ranking line est
une projection append-only, une contrainte de cohérence en base ferait échouer une
écriture pour une raison métier — ce n'est pas son rôle.

## Checklist

- [ ] Migration `ALTER TABLE` créée
- [ ] `bonus_points` sur `RankingLine`, `CumulativeTotals` et `CumulativeTotals::ZERO`
- [ ] `record_match` calcule le bonus une fois et l'utilise deux fois
- [ ] `RankingLineRow` + les deux SELECT + l'INSERT + `to_totals`
- [ ] Tests unitaires : cumul sur plusieurs matchs, total inchangé, bonus désactivé ⇒ 0
- [ ] Test `#[sqlx::test]` : la colonne est écrite et relue
- [ ] `make test` + `make check-arch` passent
