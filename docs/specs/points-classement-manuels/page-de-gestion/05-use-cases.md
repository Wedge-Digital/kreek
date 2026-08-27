# Points de classement manuels · Phase 5 : use cases

**Phase 4** : `04-dtos.md`

## Deux mutations, deux use cases

```
ranking/use_cases/
├── award_manual_points_use_case.rs
└── revoke_manual_points_use_case.rs
```

**Aucun use case de modification** : une ligne se supprime, elle ne se modifie
pas (règle 5). Un `update_*` laisserait croire le contraire à qui ouvre le
dossier.

Le dossier existe déjà — `ranking` a `record_match_ranking_use_case` et
`revert_match_ranking_use_case`. Cette fonctionnalité ne lui apporte aucune
structure neuve, contrairement au roster personnalisé qui devait tout équiper.

## 1 · Attribuer

```rust
pub struct AwardManualPointsCommand {
    pub season_id: SeasonId,
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub team_id: TeamId,
    pub points: ManualPoints,
    pub reason: Option<ManualPointsReason>,
    pub awarded_by: CoachId,
}

#[tracing::instrument(skip_all, fields(cmd = ?cmd))]
pub async fn execute(
    cmd: AwardManualPointsCommand,
    repo: &dyn IRankingRepository,
    admin: &dyn IRankingAdminPort,
    teams: &dyn IRankingCompetitionPort,
) -> Result<(), ManualPointsError>
```

1. **autorisation** — admin de compétition **ou** d'espace → `Forbidden`
2. **l'équipe est-elle inscrite à cette saison ?** → `TeamNotEnrolled`
3. `repo.insert_manual_points(…)`

### L'étape 2 n'est pas une formalité

`find_enrolled_teams(season_id)` existe déjà sur `IRankingCompetitionPort` — le
widget de classement s'en sert pour nommer les équipes.

Sans ce contrôle, on peut attribuer des points à **n'importe quel identifiant
d'équipe**, y compris celui d'une équipe d'une autre compétition. La ligne
s'écrirait, n'apparaîtrait dans aucun classement — puisque le classement ne
liste que les inscrits — et resterait invisible dans la table.

C'est le genre de donnée orpheline qu'on découvre deux ans plus tard sans savoir
d'où elle vient.

### Aucune vérification de doublon

Attribuer deux fois trois points à la même équipe est **légitime** : deux
décisions distinctes, deux motifs, deux lignes. Le relevé les montre séparément
et leur somme fait foi.

Refuser le second serait confondre « la même valeur » avec « la même décision ».

### Aucun événement

Le classement se recalcule à chaque lecture (phase 3). Rien à propager, aucun
autre BC ne connaît les points manuels.

## 2 · Retirer

```rust
#[tracing::instrument(skip_all, fields(point_id = %point_id, season_id = ?season_id))]
pub async fn execute(
    point_id: i64,
    season_id: &SeasonId,
    space_id: &SpaceId,
    competition_id: &CompetitionId,
    revoked_by: &CoachId,
    repo: &dyn IRankingRepository,
    admin: &dyn IRankingAdminPort,
) -> Result<(), ManualPointsError>
```

1. **autorisation** → `Forbidden`
2. `repo.delete_manual_points(point_id, season_id)` — **la saison dans le
   `WHERE`**, pas seulement l'identifiant

### La saison dans la clause, et c'est le cœur de cette carte

```sql
DELETE FROM ranking__manual_points WHERE id = $1 AND season_id = $2
```

`space_scope` couvre `{season_id}` du chemin — une saison d'un autre espace rend
`404` avant le handler. Mais **`{point_id}` n'est résolu par personne** : rien
n'empêche de poster l'identifiant d'une ligne d'une autre compétition sur une
URL qu'on a le droit d'atteindre.

Le `AND season_id = $2` referme ça **par construction**, sans une ligne de
contrôle à écrire ni à oublier. Zéro ligne supprimée devient alors `NotFound`.

C'est la leçon de la carte 416, où `delete_match` prend sa cible dans le corps
et agit sur elle sans vérifier qu'elle appartient au chemin.

### Idempotent

Un second appel supprime zéro ligne et rend `NotFound`. L'écran a pu être
rechargé, la ligne déjà retirée par un autre administrateur — ce n'est pas une
erreur du demandeur, mais il doit savoir que la ligne n'y est plus.

## Les erreurs

```rust
pub enum ManualPointsError {
    Forbidden,
    TeamNotEnrolled,
    NotFound,
    Repository(String),
}
```

**Pas de variante `Invalid`.** Les value objects de la phase 4 valident à la
construction : `ManualPoints` refuse zéro et hors bornes, `ManualPointsReason`
refuse le vide et le trop long. Un use case qui revaliderait ferait le travail
deux fois, et les deux se mettraient à diverger.

C'est le handler qui construit les value objects, et son échec est un `422`
avant que le use case existe.

## La lecture n'a pas de use case

Les deux widgets lisent par le dépôt et le service, comme le classement le fait
déjà. Écrire un `_use_case.rs` pour une lecture serait une erreur de nommage :
le prochain qui ouvre `use_cases/` doit pouvoir supposer qu'un fichier ainsi
nommé mute quelque chose.

## Ce que les use cases ne font pas

- **Aucune transaction** : une écriture par mutation.
- **Aucun recalcul, aucune invalidation** — le classement s'ordonne à la lecture.
- **Aucune revalidation** de ce que les value objects garantissent déjà.

## Règles métier

**Aucune à préciser.** Cette phase en confirme deux au passage, qui n'avaient
pas été dites :

- **Une équipe non inscrite ne reçoit pas de points.** Le contrôle est nouveau,
  la règle allait de soi.
- **Deux lignes identiques sont légitimes.** Deux décisions, deux motifs, deux
  lignes.
