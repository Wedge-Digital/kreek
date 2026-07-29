# Recrutement — Phase 5 : use cases

**Entrée** : `04-dtos.md` validé.

Ce document couvre l'orchestration **commune aux deux pages** — hydratation,
transaction de lot, garde anti-double-application — puis les use cases du
recrutement. `renvois/05-use-cases.md` ne consigne que ses écarts.

## 1. Contrainte technique découverte

`TeamRepository::append` **ouvre sa propre transaction** (`self.pool.begin()`) et la
committe. Appliquer un lot de N événements atomiquement est donc impossible en
l'état : N appels = N transactions.

### Nouvelle méthode de repository

```rust
async fn append_batch(
    &self,
    team_id: &str,
    events: &[TeamDomainEvent],
    expected_version: u64,
) -> Result<u64, RepositoryError>;
```

Une seule transaction : insertion des N événements à versions croissantes, mise à jour
de la projection **et du grand livre de trésorerie** pour chacun, commit. La règle des
projections event-sourcées est respectée — projection et événement dans la même
transaction.

`append` reste tel quel pour les mutations unitaires existantes.

### La suppression du brouillon n'a pas besoin d'être dans la transaction

Elle relève d'un autre repository ; l'y inclure obligerait à faire porter la
transaction par le use case, donc à exposer des types `sqlx` dans `ports.rs`.

**Ce n'est pas nécessaire, parce que l'agrégat se protège déjà.** Le dernier événement
du lot est `RecruitmentPhaseValidated` : l'équipe passe en `Dismissals`. Si la
suppression du brouillon échoue et que le coach revalide,
`validate_recruitment_phase()` appelle `expect_phase(GamePhase::Recruitment)` et
**refuse** — la double application est impossible.

Un brouillon résiduel est alors inatteignable (la page refuse hors de sa phase) et
sera purgé à l'entrée suivante en `ReadyToPlay`, par la décision D6.

## 2. Domain service d'hydratation

```rust
// use_cases/draft_hydration_service.rs
pub async fn hydrate_recruitment_draft(
    team: &Team,
    draft_repo: &dyn IPhaseDraftRepository,
    catalog:    &dyn IRosterCatalogPort,
    squad:      &dyn ISquadPort,
) -> Result<RecruitmentDraft, HydrationError>;
```

Charge les lignes persistées, le catalogue du roster et l'effectif courant, puis
construit l'agrégat. C'est **le seul endroit** où les DTOs de port sont manipulés ;
au-delà, tout est domaine.

Appelé par les quatre use cases de la page. Un brouillon absent n'est pas une erreur :
on hydrate un brouillon vide.

## 3. Use cases de mutation

Les trois ont la même forme. Exemple :

```rust
// use_cases/add_draft_player_use_case.rs
pub enum AddDraftPlayerError {
    TeamNotFound,
    WrongPhase,
    ConcurrentWrite,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd:        AddDraftPlayerCommand,
    team_repo:  &dyn ITeamRepository,
    draft_repo: &dyn IPhaseDraftRepository,
    catalog:    &dyn IRosterCatalogPort,
    squad:      &dyn ISquadPort,
) -> Result<RecruitmentDraft, AddDraftPlayerError>
```

Orchestration :

1. charger `Team`, vérifier la phase `Recruitment`
2. hydrater le brouillon
3. `draft.add_player(cmd.roster_line_id)?` — **toutes les gardes sont ici**, pures
4. `draft_repo.save(&draft, cmd.expected_version)?`
5. retourner le brouillon, que le handler transforme en VM

Le use case ne décide de rien : il charge, appelle, persiste. Les quotas, les limites
croisées, le plafond de 16 et la trésorerie sont évalués par l'agrégat.

**`remove_draft_line_use_case` est partagé** avec les renvois : retirer une ligne d'un
brouillon par son identifiant est la même opération, quelle que soit la phase.

## 4. Le use case de validation

C'est le seul qui soit réellement complexe.

```rust
pub enum ValidateRecruitmentPhaseError {
    TeamNotFound,
    WrongPhase,
    ConcurrentWrite,
    DraftNoLongerValid(Vec<RejectedLine>),   // refus en bloc — décision D5
    Domain(DomainError),
    Repository(RepositoryError),
}
```

Orchestration :

1. charger `Team`, vérifier la phase
2. hydrater le brouillon **contre l'état du jour** — prix, effectif et trésorerie
   rechargés, jamais ceux de la création du brouillon
3. `draft.validate_all()` → `Result<Vec<AppliedLine>, Vec<RejectedLine>>`
   **Refus en bloc** : une seule ligne invalide et rien n'est appliqué
4. construire le lot d'événements :
   - un `PlayerRecruited` **par joueur** — pas d'événement de lot
   - un `StaffBought` par ligne de staff
   - `RecruitmentPhaseValidated` en dernier
5. `team_repo.append_batch(&team_id, &events, team.version)`
6. `draft_repo.delete(&team_id, Phase::Recruitment)` — hors transaction, cf. §1
7. les app events partent par le publisher, à partir des domain events du lot

### Un événement par ligne, pas un événement de lot

L'event store reste lisible — « ce joueur a été recruté tel jour » — et le grand livre
de trésorerie en découle directement, une ligne par mouvement. Un événement de lot
obligerait à déplier son contenu à chaque rejeu et à chaque projection.

### L'ordre d'application n'a pas d'importance

La trésorerie est vérifiée **en total** par `draft.validate_all()` avant toute
émission. Aucune ligne ne peut donc échouer en cours de lot par manque d'argent, et
l'ordre est libre.

## 5. Le numéro de maillot est attribué par `players`

`PlayerRecruited` ne porte que `position_id`, `base_value_kpo` et `cost_kpo` :
**`teams` ne connaît pas les numéros de maillot**, et n'a aucune raison de les
connaître.

C'est `players` qui les possède (`Player.jersey`) et qui attribuera **le premier
numéro disponible** à la réception de l'app event, comme son `team_created_listener`
le fait déjà à la création.

Deux bénéfices : le port de `teams` n'a pas à transporter les numéros, et deux
recrutements dans un même lot ne peuvent pas réserver le même numéro puisque le
listener les traite séquentiellement, chacun voyant l'état laissé par le précédent.

## 6. Le nouvel app event

Recruter dans `teams` ne crée aucun joueur : il faut le dire à `players`.

```
Lot appliqué (teams)
   └─► domain event PlayerRecruited        (event store teams)
         └─► publisher teams (couche IO)
               └─► app event PlayerRecruited
                     └─► listener players
                           └─► domain event PlayerCreated
                                 └─► players_proj
```

Le listener de `players` réutilise la logique de `team_created_listener` : résolution
des compétences de base, valeur de départ, attribution du maillot. **À factoriser
plutôt qu'à dupliquer.**

## 7. Erreurs applicatives — traduction

| Erreur | Réponse HTTP | Fragment |
|---|---|---|
| `TeamNotFound` | 404 | — |
| `WrongPhase` | 422 | message de phase |
| `ConcurrentWrite` | 200 | catalogue reconstruit + bandeau de resynchronisation |
| `DraftNoLongerValid` | 422 | lignes fautives nommées |
| `Domain(_)` | 422 | message domaine |
| `Repository(_)` | 500 | — |

`ConcurrentWrite` répond **200 avec l'état à jour**, pas une erreur : le geste n'est
pas appliqué mais l'utilisateur reçoit une page cohérente.

## 8. Règles métier identifiées à cette étape

- **La garde de phase de l'agrégat rend la double application impossible**, ce qui
  autorise à sortir la suppression du brouillon de la transaction.
- **Le brouillon est validé contre l'état du jour**, pas celui de sa constitution.
- **La trésorerie n'est vérifiée qu'en total**, une seule fois, avant émission.
- **`teams` ignore les numéros de maillot** et doit continuer de les ignorer.

## 9. Points ouverts pour la phase 6

- `RejectedLine` doit-il porter un motif structuré (quota, trésorerie, limite croisée)
  ou un message déjà formulé ? Un motif structuré laisse la couche web choisir la
  formulation, mais duplique l'énumération des causes.
- `append_batch` doit-il refuser un lot vide, ou l'accepter en n'appendant que
  `RecruitmentPhaseValidated` ? Le coach peut terminer sa phase sans rien acheter.
