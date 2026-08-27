# Points de classement manuels · Phase 3 : architecture back

**Phase 2** : `02-front.md`

## La question du recalcul n'en est pas une

La phase 2 la laissait ouverte : le classement se recalcule-t-il à
l'attribution, comme il le fait à un changement de barème (carte 422) ?

**Non, et rien ne se recalcule — parce que le classement n'est stocké nulle
part.**

```rust
// detailed_standings_widget.rs:109
let (rules, teams, lines, groups) = tokio::join!(
    …find_ranking_rules(season_id),
    …find_enrolled_teams(season_id),
    …find_latest_lines_for_season(season_id),   ← les cumuls, un par équipe
    …find_groups(season_id),
);
…
build_detailed_groups(space_id, lines, &teams, &groups, &order)
```

`build_ordered_standings` **ordonne à chaque lecture** : il prend la dernière
ligne cumulée de chaque équipe, applique les critères de départage, attribue les
rangs. Il n'existe aucune table de classement ordonné.

Conséquence directe : **un point manuel attribué est visible au rechargement
suivant, sans aucune propagation.** Pas d'événement, pas de listener, pas de
rejeu, pas de cache à invalider.

C'est le contraire du changement de barème, qui doit rejouer parce qu'il change
les **cumuls eux-mêmes**, lesquels sont stockés.

## Persistance

```sql
CREATE TABLE ranking__manual_points (
    id          BIGSERIAL PRIMARY KEY,
    season_id   TEXT NOT NULL,
    team_id     TEXT NOT NULL,
    points      INTEGER NOT NULL,        -- signé : négatif pour une pénalité
    reason      TEXT,                     -- facultatif (phase 1)
    awarded_by  TEXT NOT NULL,
    awarded_at  TIMESTAMPTZ NOT NULL DEFAULT now()
);
CREATE INDEX ON ranking__manual_points (season_id);
CREATE INDEX ON ranking__manual_points (season_id, team_id);
```

**Une table à part de `ranking_lines`** (tranché en phase 2). C'est ce qui les
fait survivre au rejeu de la carte 418, qui recalcule les points depuis zéro à
partir des cumuls.

**`points` est signé**, une seule colonne plutôt qu'un montant et une direction.
Le grand livre de trésorerie fait l'inverse — `direction` + `amount_kpo` — parce
qu'il distingue deux natures de mouvement. Ici il n'y en a qu'une : un
ajustement. Le signe **est** l'information, et deux colonnes obligeraient à les
tenir cohérentes.

**`reason` est nullable**, et c'est la règle 6.

**Pas d'`updated_at`** : une ligne ne se modifie pas (règle 5). En ajouter un
laisserait croire le contraire.

**Aucune suppression logique.** Une ligne retirée est retirée — c'est un
`DELETE`. Un drapeau `deleted_at` compliquerait chaque lecture pour conserver
une trace que personne n'a demandée.

## La lecture — deux formes, deux besoins

```rust
// ranking/ports.rs — sur IRankingRepository
/// Le total par équipe, pour le classement. Une ligne par équipe qui en a.
async fn find_manual_totals_for_season(&self, season_id: &str)
    -> Result<HashMap<String, i32>, RankingRepositoryError>;

/// Le détail, pour la page de gestion. Ordonné par équipe puis par date.
async fn list_manual_points(&self, season_id: &str)
    -> Result<Vec<ManualPointRow>, RankingRepositoryError>;
```

**Deux méthodes et non une.** Le classement veut un total par équipe — un
`GROUP BY` que la base fait mieux que le Rust — et la page de gestion veut
chaque ligne avec son motif et son auteur. Servir la seconde au classement le
ferait sommer en mémoire des lignes dont il n'a que faire.

```rust
pub struct ManualPointRow {          // DTO de lecture, primitives assumées
    pub id: i64,
    pub team_id: String,
    pub points: i32,
    pub reason: Option<String>,
    pub awarded_by: String,
    pub awarded_at: DateTime<Utc>,
}
```

## Où le point manuel entre dans le calcul

C'est le cœur de la phase, et la règle 2 en décide : **avant les départages**.

```rust
// standings_service.rs
pub fn build_ordered_standings(
    lines: Vec<RankingLineRow>,
    manual: &HashMap<String, i32>,   // ← nouveau
    order: &TiebreakOrder,
) -> Vec<(TeamStanding, Rank)>
```

L'ajout se fait dans `to_standing`, **avant** `order_standings` :

```
1. to_standing(row)                 les cumuls tels que stockés
2. + manual[team_id]                le point manuel entre dans le total
3. order_standings(…)               l'ordre se fait sur le total ajusté
4. assign_ranks(…)                  les égalités portent sur le total ajusté
```

**Une équipe à 3 points + 2 manuels est à égalité avec une équipe à 5 sans
manuel**, et ce sont les départages qui tranchent — c'est littéralement la règle
que tu as posée, et elle tient à ce seul ordre d'opérations.

### `TeamStanding` porte les deux nombres, pas un seul

```rust
pub struct TeamStanding {
    pub team_id: String,
    pub totals: CumulativeTotals,
    pub manual_points: i32,      // ← nouveau
}
```

**Le total ajusté n'est pas écrasé dans `totals`.** Le classement doit afficher
les deux séparément — la colonne « Man. » à côté du total — et fondre l'un dans
l'autre rendrait la décomposition impossible à retrouver.

C'est aussi ce qui garde `CumulativeTotals` fidèle à ce qui est en base : il
décrit les cumuls de match, et rien d'autre.

### Le total affiché

```rust
impl TeamStanding {
    /// Le total qui ordonne et qui s'affiche : cumuls + points manuels.
    pub fn total_points(&self) -> i32;
}
```

**`i32` et non `u32`** : le total peut devenir négatif (règle 4). Une pénalité de
5 sur une équipe à 3 points donne −2, et c'est un état valide.

`RankingPoints` est un nutype `u32` — il **ne peut pas** porter ce total. Le
laisser en `i32` nu dans un type de lecture est admis ; ce qui compte est que
personne ne tente de le remettre dans `RankingPoints`.

## Les mutations

Deux, dans `ranking/use_cases/` :

```rust
award_manual_points_use_case::execute(cmd, repo, admin) -> Result<(), …>
revoke_manual_points_use_case::execute(id, season_id, repo, admin) -> Result<(), …>
```

**Aucun use case de modification** : une ligne se supprime (règle 5).

**Aucun événement.** Rien à propager : le classement se recalcule à chaque
lecture, et aucun autre BC ne connaît les points manuels.

### Le contrôle d'accès

Admin de compétition ou d'espace, comme le reste de l'administration. `ranking`
n'a aucun port pour le vérifier — il lui en faut un, sur le modèle de
`match_report::ISpaceAdminPort` :

```rust
#[async_trait]
pub trait IRankingAdminPort: Send + Sync {
    async fn is_competition_admin(&self, user_id: &str, competition_id: &str) -> bool;
    async fn is_space_admin(&self, user_id: &str, space_id: &str) -> bool;
}
```

L'adapter vit dans `src/infrastructure/ranking/`, à côté de
`competition_info_adapter.rs`.

**La lecture est ouverte à tout membre** : les points manuels sont publics
(règle 3), et la page de gestion est consultable par tous. Seules les deux
mutations sont réservées.

## Les routes

```
GET    /app/{space_id}/ranking/{competition_id}/{season_id}/manual-points
GET    …/manual-points/form
GET    …/manual-points/list
POST   …/manual-points
DELETE …/manual-points/{point_id}
```

La page, ses deux fragments, et les deux mutations.

**`{point_id}` dans le chemin, jamais dans le corps.** C'est la leçon de la
carte 416 : `delete_match` prend sa cible dans le corps, hors de portée de
`space_scope`.

`space_scope` couvre `{season_id}`, dont `competitions` déclare le résolveur —
une saison d'un autre espace rend `404` avant le handler. Le `{point_id}`, lui,
n'est pas résolu : **le use case vérifie que la ligne appartient bien à la
saison du chemin**, sans quoi on supprimerait la ligne d'une autre compétition
en postant sur une URL qu'on a le droit d'atteindre.

## Ce que le back ne fait pas

- **Aucun rejeu, aucun événement, aucun listener.**
- **Aucune migration de données** : la table naît vide.
- **Aucune modification de `ranking_lines`**, ni de son écriture.

## Règles métier

**Aucune à préciser.** Les six de la phase 1 tiennent, et cette phase confirme
la plus structurante : les points manuels entrant avant les départages, il
suffit de les ajouter au bon endroit d'une fonction qui ordonne déjà à chaque
lecture.
