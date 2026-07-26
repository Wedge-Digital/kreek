# Départages — Compteurs cumulés sur la ranking line

**Priorité : haute**
**Dépend de :** carte 215 (commande regroupée)
**Contexte :** `migrations/`, `src/app/ranking/domain/ranking_line.rs`, `src/app/ranking/ports.rs`, `src/app/ranking/io/repository/ranking_repository.rs`, `src/app/ranking/io/app_events/match_report_published_listener.rs`, `src/app/ranking/use_cases/record_match_ranking_use_case.rs`
**Spec :** `docs/specs/ranking/tiebreakers/tiebreak-calc/{04-dtos,06-domaine,07-integration}.md`

## Objectif

Accumuler et persister les 5 compteurs de départage. **Aucun ordonnancement** : la
comparaison arrive en carte 217, son câblage en 218. Le classement affiché ne change pas.

Carte **atomique** : la forme de `MatchStats` et de `CumulativeTotals` casse simultanément
le domaine, le repository, le listener et le use case.

## Conception

### 1. Migration (cf. `07-integration.md`)

```sql
ALTER TABLE ranking_lines
    ADD COLUMN td_for      INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN td_against  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN casualties  INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN fouls       INTEGER NOT NULL DEFAULT 0,
    ADD COLUMN completions INTEGER NOT NULL DEFAULT 0;
```

Pas de colonne `diff_td` : dérivé (règle 13). Pas de backfill.

Après application : `make prepare_db` pour régénérer le cache `.sqlx/`, sinon les macros
`query_as!` ne compilent plus.

### 2. Value objects (cf. `04-dtos.md`)

`TdFor`, `TdAgainst`, `CasualtiesTotal`, `FoulsCommitted`, `CompletionsMade`.

`CasualtiesTotal` est **distinct** de `CasualtiesInflicted` (par match) : deux `u32` nus se
confondraient sans que le compilateur bronche, et confondre les sorties d'un match avec
celles de la saison donnerait un compteur plausible mais faux.

### 3. Domaine

- `MatchStats` : + `fouls: FoulsCommitted`, `completions: CompletionsMade`. Les bonus ne
  les utilisent pas — ils alimentent les compteurs (règle 12 : on accumule tout).
- `CumulativeTotals` : + les 5 compteurs, `ZERO` complété.
- `record_match` : accumule les 5, sans condition d'activation.

### 4. Port, repository, use case, listener

| Élément | Changement |
|---|---|
| `RankingLineRow` | + 5 champs `u32` |
| `find_latest_line`, `find_latest_lines_for_season` | 5 colonnes au `SELECT` |
| `insert_lines` | 5 colonnes à l'`INSERT` (`$14`…`$18`) |
| `to_totals` | Reporte les 5 compteurs, sinon le cumul repart de zéro |
| `TeamMatchStats` | + `fouls`, `completions` |
| Listener | `count_agressions` et `count_passes`, calqués sur `count_sorties` (`:79`) |

## Tests (cf. `06-domaine.md`)

- Les 5 compteurs s'accumulent sur plusieurs matchs
- Ils s'accumulent même quand aucun bonus n'est activé (règle 12)
- **Symétrie** : sur un match 2-1, `td_for`/`td_against` se croisent entre les deux
  équipes, mais `fouls`, `completions` et `casualties` restent ceux de chaque équipe
- `#[sqlx::test]` du pipeline étendu : payload avec `Agression` et `Passe`, les 5 compteurs
  persistés avec les bonnes valeurs par équipe

Le test de symétrie est le plus important : une inversion compile et produit des compteurs
plausibles.

## Checklist

- [ ] Migration créée et appliquée, `.sqlx/` régénéré et commité
- [ ] 5 VOs définis, `CasualtiesTotal` distinct de `CasualtiesInflicted`
- [ ] `MatchStats` +2, `CumulativeTotals` +5, `ZERO` complété
- [ ] `record_match` accumule sans condition d'activation
- [ ] `RankingLineRow`, les 2 SELECT, l'INSERT, `to_totals`
- [ ] `count_agressions` / `count_passes` dans le listener
- [ ] Tests ci-dessus, dont la symétrie et le pipeline
- [ ] Aucun changement d'ordre du classement affiché
- [ ] `make test` + `make check-arch` passent
