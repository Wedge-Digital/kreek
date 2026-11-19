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

- [x] La migration et ses deux index
- [x] Les quatre méthodes de dépôt
- [x] `IRankingAdminPort`, son adapter, l'injection dans le contexte
- [x] Les deux use cases, instrumentés
- [x] Les sept tests — **dix-neuf**, cf. ci-dessous
- [x] `make lint && make test && make check-arch`

## Le nom de table, tranché sur les données

La carte proposait `ranking__manual_points`. Les 31 tables de la base ont été
relues avant d'écrire la migration : **toutes** portent le préfixe de leur BC
propriétaire, sans exception. Un `manual_ranking_points` aurait été le premier à
mettre le qualificatif devant — il se serait rangé entre `match_report_proj` et
`players__customisation_baskets`, orphelin de `ranking_lines`, et un `\dt`
n'aurait plus dit quel BC le possède.

Le séparateur, lui, est partagé : `auth__`, `spaces__`, `players__`, `teams__` en
double ; `ranking_lines`, `competition_seasons`, `*_proj` en simple. Le `__` est
la direction récente. Le BC porte donc les deux formes côte à côte, assumé.

## Trois écarts de mise en œuvre

### SQL en ligne, pas en fichiers

Le `CLAUDE.md` demande des `.sql` dédiés, mais `ranking_repository.rs` utilise
les **macros** `query!`/`query_as!`, vérifiées à la compilation — et ces macros
n'acceptent pas `include_str!`. `competitions` a fait l'autre choix
(`query_as::<_, Row>(include_str!(…))`) et **perd la vérification**. Le style du
fichier a été conservé : la vérification vaut plus ici que l'uniformité de
rangement, et le `CLAUDE.md` recommande précisément la macro.

Conséquence de séquencement : la migration doit être appliquée **avant** que le
code compile, puis `make prepare_db` pour le cache hors-ligne.

### Trois fichiers de use case, pas deux

La carte prescrit `award_manual_points_use_case` et
`revoke_manual_points_use_case`. Les deux partagent `ManualPointsError` et le
contrôle d'autorisation ; ces pièces vivent dans `manual_points.rs`, **sans
suffixe** — la convention réserve `_use_case.rs` aux orchestrations, et un
fichier qui n'en porte aucune ne doit pas s'en réclamer.

Un premier jet les avait réunis dans un `manual_points_use_cases.rs` au pluriel :
un nom qui n'existe pas dans la convention, et que rien de mécanique n'aurait
refusé.

### `autorise` est instrumentée, pas déclarée `arch:no-instrument`

L'axe 11 la refusait — une `pub async fn` de `use_cases/` sans instrumentation.
Le marqueur d'exception était disponible, mais **le port sépare les deux sources
d'autorisation précisément pour qu'on sache laquelle a répondu** : un journal
muet lui ôterait la moitié de sa valeur. Un `Forbidden` seul ne dit pas *par
où* l'accès a été refusé — ni, en cas d'accès inattendu, par où il est passé.

Le `||` a été défait en deux lectures explicites : un court-circuit sauterait le
second appel dès que le premier répond vrai, et la trace ne dirait plus que
l'accès était de toute façon acquis par l'autre porte.

## Un trou latent dans l'axe 11 de `check-arch`

L'axe ne lit que **la ligne immédiatement précédant** la signature. Un attribut
`#[tracing::instrument(…)]` que `cargo fmt` replie sur plusieurs lignes finit par
`)]`, que le contrôle ne reconnaît pas : **une fonction correctement instrumentée
est alors refusée**.

Rencontré ici en écrivant un attribut à trois champs. Contourné en le
raccourcissant pour qu'il tienne sur une ligne, avec un commentaire qui dit
pourquoi. Aucun autre attribut multiligne n'existe aujourd'hui dans `src/app` —
le trou est latent, pas actif. Une carte serait justifiée : le symptôme est un
échec de `check-arch` sur du code juste, ce qui pousse à ajouter un marqueur
`arch:no-instrument` mensonger pour débloquer.

## Falsification

| Mutation | Constaté |
|---|---|
| `AND season_id` retiré du `DELETE` | 1 rouge : `delete_d_une_autre_saison_ne_supprime_rien` |
| Contrôle d'inscription retiré | 1 rouge : `une_equipe_non_inscrite_est_refusee` |
| Autorisation réduite à la porte « compétition » | 2 rouges, un `l_admin_d_espace_seul_suffit` par use case |
| Autorisation réduite à la porte « espace » | 2 rouges, un `l_admin_de_competition_seul_suffit` par use case |
| `ORDER BY` de `list_manual_points` cassé | 1 rouge : `list_rend_les_lignes_ordonnees` |

Les deux mutations d'autorisation sont la leçon de la carte 426 appliquée
d'avance : chaque porte a son test, dans chaque use case, et supprimer l'une
n'est jamais silencieux.

## Douze tests au-delà des sept prescrits

Trois d'intégration : une équipe sans ligne est **absente** des totaux (ce qui
rend le `unwrap_or(0)` du service légitime plutôt que défensif), les deux
lectures sont cloisonnées par saison.

Six d'autorisation : les deux portes séparément, dans chacun des deux use cases,
plus le fait que le refus d'autorisation **précède** celui d'inscription — un
non-admin n'apprend pas quelles équipes sont inscrites en essayant.

Trois de transmission : le motif et l'auteur atteignent le dépôt tels quels
(l'apostrophe comprise), et la saison accompagne l'identifiant jusqu'au `DELETE`
— la moitié applicative d'un contrôle dont l'autre moitié est le `WHERE`.
