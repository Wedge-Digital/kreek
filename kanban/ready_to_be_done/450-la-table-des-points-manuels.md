# La table des points manuels

**Ordre :** 1 · **Dépend de :** rien
**Conception :** `docs/specs/points-classement-manuels/page-de-gestion/`
(`03-back.md`, `05-use-cases.md`)

## Objectif

Écrire, lire et supprimer un point manuel. Aucun écran.

## Conception

### 1. La table

```sql
-- migrations/<date>_ranking_manual_points.sql
CREATE TABLE ranking__manual_points (
    id          BIGSERIAL PRIMARY KEY,
    season_id   TEXT NOT NULL,
    team_id     TEXT NOT NULL,
    points      INTEGER NOT NULL,
    reason      TEXT,
    awarded_by  TEXT NOT NULL,
    awarded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON ranking__manual_points (season_id);
CREATE INDEX ON ranking__manual_points (season_id, team_id);
```

**Une table à part de `ranking_lines`**, et c'est ce qui les fait survivre au
rejeu : la carte 418 recalcule les points depuis zéro à partir des cumuls. Tout
ce qui vivrait dans ce cumul serait effacé au premier changement de barème.

**`points` est une colonne signée**, pas un montant plus une direction. Le grand
livre de trésorerie fait l'inverse parce qu'il distingue deux natures de
mouvement ; ici il n'y en a qu'une, et deux colonnes obligeraient à les tenir
cohérentes.

**Pas d'`updated_at`** : une ligne ne se modifie pas, en ajouter un laisserait
croire le contraire. **Pas de suppression logique** non plus — un drapeau
compliquerait chaque lecture pour conserver une trace que personne n'a demandée.

Deux index parce que les deux lectures diffèrent : le classement agrège par
saison, la page de gestion groupe par équipe.

### 2. Quatre méthodes de dépôt

```rust
async fn find_manual_totals_for_season(&self, season_id: &str)
    -> Result<HashMap<String, i32>, RankingRepositoryError>;
async fn list_manual_points(&self, season_id: &str)
    -> Result<Vec<ManualPointRow>, RankingRepositoryError>;
async fn insert_manual_points(&self, …) -> Result<(), RankingRepositoryError>;
async fn delete_manual_points(&self, id: i64, season_id: &str)
    -> Result<u64, RankingRepositoryError>;
```

```sql
SELECT team_id, SUM(points)::int AS total
FROM   ranking__manual_points WHERE season_id = $1 GROUP BY team_id
```

**Deux lectures et non une** : le classement veut un total par équipe — un
`GROUP BY` que la base fait mieux que le Rust — et la page de gestion veut
chaque ligne avec son motif.

### 3. Le port d'autorisation

```rust
// ranking/ports.rs
#[async_trait]
pub trait IRankingAdminPort: Send + Sync {
    async fn is_competition_admin(&self, user_id: &str, competition_id: &str) -> bool;
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}
```

Adapter dans `src/infrastructure/ranking/admin_adapter.rs`, à côté de
`competition_info_adapter.rs`. Injecté dans `RankingContext`, qui ne porte
aujourd'hui que `repository` et `competition_port`.

**Deux méthodes et non une `is_admin`** : les autorisations viennent de deux
sources — la compétition porte ses `admin_ids`, l'espace son `SpaceProfile` — et
les fondre cacherait laquelle a répondu.

### 4. Les deux use cases

```rust
award_manual_points_use_case::execute(cmd, repo, admin, teams) -> Result<(), ManualPointsError>
revoke_manual_points_use_case::execute(id, season_id, …)       -> Result<(), ManualPointsError>
```

**Aucun use case de modification** : une ligne se supprime.

L'attribution vérifie **que l'équipe est inscrite** — `find_enrolled_teams`
existe déjà sur `IRankingCompetitionPort`. Sans ce contrôle, une ligne
s'écrirait pour n'importe quel identifiant, n'apparaîtrait dans aucun classement
puisque celui-ci ne liste que les inscrits, et resterait invisible dans la
table : le genre de donnée orpheline qu'on découvre deux ans plus tard.

**Aucune vérification de doublon.** Deux fois trois points à la même équipe,
c'est deux décisions et deux motifs.

La suppression porte **la saison dans le `WHERE`** :

```sql
DELETE FROM ranking__manual_points WHERE id = $1 AND season_id = $2
```

`space_scope` couvre `{season_id}` du chemin, mais **`{point_id}` n'est résolu
par personne**. Le `AND season_id` referme ce trou par construction, sans un
contrôle à écrire et à oublier — la leçon de la carte 416. Zéro ligne supprimée
devient `NotFound`.

```rust
pub enum ManualPointsError { Forbidden, TeamNotEnrolled, NotFound, Repository(String) }
```

**Pas de variante `Invalid`** : les value objects de la carte 449 valident à la
construction, et revalider ferait le travail deux fois — les deux finissant par
diverger.

Les deux use cases sont **instrumentés** (`#[tracing::instrument(skip_all, …)]`),
sans quoi l'axe 11 de `check-arch` refuse.

## Tests

| Test | Règle |
|---|---|
| `insert_puis_totals_somme_les_lignes` | intégration, vraie base |
| `list_rend_les_lignes_ordonnees` | équipe puis date |
| `delete_d_une_autre_saison_ne_supprime_rien` | **le test du `AND season_id`** |
| `delete_deux_fois_rend_zero_la_seconde` | idempotence |
| `un_non_admin_est_refuse` | les deux use cases |
| `une_equipe_non_inscrite_est_refusee` | `TeamNotEnrolled` |
| `deux_lignes_identiques_sont_acceptees` | le cas passant qu'on croirait interdit |

## Checklist

- [ ] La migration et ses deux index
- [ ] Les quatre méthodes de dépôt
- [ ] `IRankingAdminPort`, son adapter, l'injection dans le contexte
- [ ] Les deux use cases, instrumentés
- [ ] Les sept tests
- [ ] `make lint && make test && make check-arch`
