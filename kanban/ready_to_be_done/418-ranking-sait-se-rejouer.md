# `ranking` sait se rejouer

**Épic :** E14 · **Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/03-back.md`

## Objectif

Donner au BC `ranking` la capacité de reconstruire tout le classement d'une
saison avec le barème courant. Aucun écran, aucune route : un use case
appelable et prouvé.

## Ce qui rend la chose possible

Une ligne de `ranking_lines` est **cumulative** : elle porte les totaux de
l'équipe après le match, pas les statistiques du match. On croirait donc devoir
les redemander à `match_report`. Il n'en est rien — elles se retrouvent par
**différence de deux lignes consécutives** de la même équipe :

| `MatchStats` | Se retrouve par |
|---|---|
| `own_td` | `td_for(n) − td_for(n−1)` |
| `opponent_td` | `td_against(n) − td_against(n−1)` |
| `casualties_inflicted` | `casualties(n) − casualties(n−1)` |
| `fouls` | `fouls(n) − fouls(n−1)` |
| `completions` | `completions(n) − completions(n−1)` |

Le **résultat** ne se lit pas : `record_match` le redérive des deux scores
(`derive_outcome`). Les colonnes `wins` / `draws` / `losses` sont un produit du
rejeu, pas une entrée.

**Conséquence : aucun port vers `match_report`.** Le BC est autosuffisant, et le
rejeu ne peut pas diverger d'un rapport modifié entre-temps puisqu'il ne relit
rien.

## Conception

### La fonction inverse, dans le domaine

```rust
// ranking/domain/ranking_line.rs
impl RankingLine {
    /// L'inverse exact de `record_match`.
    pub fn stats_between(
        previous: Option<&CumulativeTotals>,
        current: &RankingLine,
    ) -> Result<MatchStats, DomainError>;
}
```

**Dans le domaine, à côté de `record_match`** : les deux doivent être modifiées
ensemble. Un champ ajouté à `MatchStats` sans être ajouté ici produirait un
recalcul qui perd cette statistique — silencieusement, la ligne restant bien
formée.

**Elle rend un `Result`** : `td_for` est un cumul `u32`, `MatchScore` un
`MatchScore(pub u8)` nu, sans constructeur intelligent. La conversion passe par
`u8::try_from`, jamais par `as` — qui replierait un écart aberrant en un score
parfaitement plausible. Seule une corruption de lignes le produit, et c'est
précisément le cas qu'il ne faut pas maquiller.

### Deux méthodes de dépôt

```rust
/// Toutes les lignes de la saison, dans l'ordre du cumul :
/// `recorded_at` croissant, départagé par `match_report_id`.
async fn find_all_lines_for_season(&self, season_id: &str)
    -> Result<Vec<RankingLineRow>, RankingRepositoryError>;

/// Suppression et insertion dans une seule transaction.
async fn replace_lines_for_season(&self, season_id: &str, lines: &[RankingLine])
    -> Result<(), RankingRepositoryError>;
```

`replace_lines_for_season` et non un `delete` suivi d'`insert_lines` : deux
transactions, et l'échec de la seconde laisse la saison **sans classement du
tout**. Le `DELETE` porte sur `season_id`, pas sur une liste d'identifiants —
une ligne orpheline non relue serait sinon conservée par un rejeu qui prétend
l'avoir remplacée.

### Le use case

```rust
// ranking/use_cases/recompute_season_ranking_use_case.rs
pub async fn execute(
    season_id: &SeasonId,
    repo: &dyn IRankingRepository,
    competition_port: &dyn IRankingCompetitionPort,
) -> Result<RecomputeReport, RecomputeSeasonRankingError>

pub struct RecomputeReport { pub matches_replayed: u32, pub teams: u32 }
```

1. lire toutes les lignes de la saison, dans leur ordre d'origine
2. grouper par équipe, différencier chaque ligne avec la précédente
3. lire le barème courant par `IRankingCompetitionPort::find_ranking_rules`
4. rejouer `RankingLine::record_match` depuis zéro, par équipe, dans l'ordre
5. `replace_lines_for_season`

**Instrumenté** — `#[tracing::instrument(skip_all, fields(season_id = ?season_id))]`,
c'est un use case async (axe 11 de `check-arch`).

Il rend un compte-rendu et non `()` : « recalculé » sans chiffre ne se distingue
pas de « rien à recalculer », et c'est ce que l'écran devra dire.

## Tests

| Test | Ce qu'il prouve |
|---|---|
| `stats_between_est_l_inverse_de_record_match` | **propriété** : `stats_between(record_match(p, ctx, s, r)) == s`, sur plusieurs jeux |
| `stats_between_sur_la_premiere_ligne` | `previous = None`, cumuls à zéro |
| `stats_between_echoue_sur_un_ecart_aberrant` | `u8::try_from`, pas de repliement |
| `rejeu_idempotent_a_bareme_inchange` | rejoué sans changer le barème, les lignes sont identiques |
| `rejeu_applique_le_nouveau_bareme` | victoire à 3 points au lieu de 2, les totaux suivent |
| `replace_lines_for_season_est_atomique` | intégration : un échec en cours ne laisse pas de saison à moitié rejouée |

Le premier protège le couple : écrit sur plusieurs jeux de statistiques, il
échoue dès qu'un champ est ajouté d'un côté sans l'autre. Le quatrième est le
filet du recalcul entier.

## Checklist

- [ ] `RankingLine::stats_between` + ses trois tests
- [ ] Les deux méthodes de `IRankingRepository` et leur implémentation sqlx
- [ ] `recompute_season_ranking_use_case`, instrumenté
- [ ] Les trois tests de rejeu
- [ ] `make lint && make test && make check-arch`
