# Onglet Paramètres · Phase 5 : use cases

**Phase 4** : `04-dtos.md`

## Cinq mutations, cinq use cases

Un par panneau, aucun fourre-tout. Tous portent le suffixe `_use_case.rs` — la
convention du `CLAUDE.md`, que les use cases existants de `competitions`
(`save_competition_rules.rs`) précèdent sans la suivre : elle s'applique au
nouveau code, on ne renomme pas l'ancien pour l'occasion.

```
competitions/use_cases/settings/
├── update_general_settings_use_case.rs
├── update_ranking_settings_use_case.rs
├── update_pools_settings_use_case.rs
├── update_tiers_settings_use_case.rs
└── update_visibility_settings_use_case.rs
```

Un sous-dossier `settings/`, comme `admin/` : cinq fichiers de plus à plat dans
`use_cases/` noieraient les douze qui s'y trouvent déjà.

## Le geste commun : lire, remplacer une part, réécrire

Les cinq partagent la même forme, parce que les cinq écrivent une **part** d'un
document JSONB dont l'écriture remplace le tout (phase 3) :

```
1. charger le document courant          ← s'il est absent, c'est une erreur, pas un défaut
2. y remplacer la seule part éditée
3. réécrire le document entier
```

**L'étape 1 n'est pas une précaution, c'est l'ordre de marche.** Un use case qui
construirait le document depuis sa seule commande effacerait tout ce que la
commande ne porte pas. C'est le risque central de cet onglet, et il est le même
cinq fois.

**Un document absent est une erreur** — `SeasonNotFound` — jamais un document
neuf construit par défaut. Une saison sans règles enregistrées n'existe pas :
le magicien en pose dès l'étape 2. Se replier sur un défaut écraserait
silencieusement un état qu'on n'a pas su lire.

---

## 1 · `update_general_settings_use_case`

```rust
pub struct UpdateGeneralSettingsCommand {
    pub competition_id: CompetitionId,
    pub space_id: SpaceId,
    pub season_id: SeasonId,
    pub name: CompetitionName,
    pub season_name: SeasonName,
    pub logo: CloudinaryImage,
}

pub async fn execute(
    cmd: UpdateGeneralSettingsCommand,
    competition_repo: &dyn ICompetitionRepository,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateGeneralSettingsError>
```

**Un seul use case pour deux écritures.** Le nom de compétition vit dans
`competitions`, le nom de saison dans `competition_seasons` — mais l'intention
de l'utilisateur est une : « enregistrer les informations générales ». Le
`CLAUDE.md` interdit au handler d'appeler deux use cases ; c'est donc ici que
les deux écritures se coordonnent.

Orchestration :

1. `competition_repo.find_base_info(&competition_id)` → `CompetitionNotFound`
2. si le nom change, `name_exists_in_space` → `NameAlreadyTaken`
3. `update_base_info(&competition_id, &name, &logo, &current.admin_ids)`
   — **les administrateurs relus**, le panneau ne les édite pas
4. `season_repo.find_rules(&season_id)` → `SeasonNotFound`
5. `save_rules(&season_id, &cmd.season_name, &current_rules)`
   — **les règles relues**, seul le nom change

```rust
pub enum UpdateGeneralSettingsError {
    CompetitionNotFound,
    SeasonNotFound,
    NameAlreadyTaken,
    Repository(String),
}
```

**Ce que ce use case ne garantit pas** : les deux écritures ne partagent pas de
transaction. Elles visent deux tables par deux dépôts, et l'échec de la seconde
laisse le nom de compétition changé sans le nom de saison. Assumé — les deux
sont des libellés, aucun invariant ne les lie, et l'écran renvoie l'état réel
au retour. Ce serait inacceptable si l'un des deux conditionnait une règle ; ce
n'est pas le cas.

---

## 2 · `update_ranking_settings_use_case`

```rust
pub struct UpdateRankingSettingsCommand {
    pub season_id: SeasonId,
    pub ranking_rules: RankingRules,
}

pub struct RankingSettingsOutcome {
    pub matches_replayed: u32,
    pub teams: u32,
}

pub async fn execute(
    cmd: UpdateRankingSettingsCommand,
    season_repo: &dyn ISeasonRepository,
    recompute: &dyn IRankingRecomputePort,
) -> Result<RankingSettingsOutcome, UpdateRankingSettingsError>
```

Orchestration :

1. `find_base_info(&season_id)` → le nom de saison, à repasser tel quel
2. `find_rules(&season_id)` → `SeasonNotFound`
3. `CompetitionRules { ranking_rules: cmd.ranking_rules, tiers: current.tiers }`
   — **les tiers relus**
4. `save_rules(…)`
5. `recompute.recompute_season(&season_id)` → `RecomputeFailed(String)`

**Le recalcul après l'enregistrement, jamais l'inverse** : il lit le barème par
son propre port, donc il doit lire le nouveau.

**Un recalcul en échec ne défait pas l'enregistrement.** Le barème reste écrit,
et l'erreur remonte pour que l'écran le dise. C'est acceptable parce que le
rejeu est **idempotent** (phase 3) : le relancer suffit, et le laisser à
retenter vaut mieux qu'un rollback qui rendrait le barème et le classement
incohérents dans l'autre sens.

```rust
pub enum UpdateRankingSettingsError {
    SeasonNotFound,
    Repository(String),
    RecomputeFailed(String),   // le barème est enregistré, le classement ne l'a pas suivi
}
```

Le commentaire sur la variante fait partie du contrat : c'est la seule erreur de
cet onglet qui laisse le système dans un état à moitié appliqué, et l'appelant
doit le savoir pour rédiger le bon message.

---

## 3 · `update_pools_settings_use_case`

```rust
pub struct UpdatePoolsSettingsCommand {
    pub season_id: SeasonId,
    pub use_pools: UseRankingGroups,
    pub pools: Vec<PoolInput>,          // ordre = ordre d'affichage
}

pub struct PoolInput {
    /// `None` pour une poule neuve — le use case lui donne son identifiant.
    pub id: Option<RankingGroupId>,
    pub name: RankingGroupName,
}

pub struct PoolsSettingsOutcome {
    pub unassigned_teams: u32,
}

pub async fn execute(
    cmd: UpdatePoolsSettingsCommand,
    season_repo: &dyn ISeasonRepository,
    id_service: &dyn IdService,
) -> Result<PoolsSettingsOutcome, UpdatePoolsSettingsError>
```

Orchestration :

1. `find_structure(&season_id)` → `SeasonNotFound`
2. donner un identifiant aux poules neuves (`id_service.generate_id()`), puis
   construire les `RankingGroup` — le type du domaine, pas une copie
3. remplacer `ranking_group`, **conserver `schedule` et `play_offs_phase`**
4. `save_structure_and_prune_groups(&season_id, &structure, &kept_ids)`

**L'identifiant est engendré côté serveur**, et c'est un écart délibéré au
précédent : le magicien laisse le navigateur le fabriquer
(`new-competition-phase-3.html:235`, `genId()`). Un identifiant de domaine
minté par le client est un identifiant qu'on ne contrôle ni en forme, ni en
unicité, ni en provenance. `IdService` existe pour ça
(`shared_kernel/identity/id_service.rs`), on ne réplique pas l'habitude du
magicien.

### L'atomicité est dans le dépôt, pas dans le use case

Les deux écritures — le JSONB et la suppression des poules absentes — doivent
être atomiques (phase 3). Elles ne peuvent pas l'être depuis le use case : une
transaction sqlx ne se partage pas entre deux ports sans faire entrer sqlx dans
une couche qui n'en veut pas.

Le projet règle déjà ce cas de la même façon : `competition_repository.rs:46`
et `match_day_repository.rs:169` ouvrent leur transaction **dans le dépôt**.
Une méthode de plus sur `ISeasonRepository` suit le même chemin :

```rust
/// Écrit la structure et supprime les poules de la saison absentes de
/// `kept_ids`, dans une seule transaction. Rend le nombre d'affectations
/// défaites par la cascade sur `competition_group_teams`.
async fn save_structure_and_prune_groups(
    &self, season_id: &SeasonId, structure: &CompetitionStructure, kept_ids: &[String],
) -> Result<u64, SeasonRepositoryError>;
```

**Sur `ISeasonRepository` et non sur le port des poules** : c'est l'écriture de
la structure qui commande, la suppression n'en est que la conséquence. L'inverse
donnerait un port des poules qui écrit du JSONB de saison.

### Retirer toutes les poules est un cas ordinaire

`kept_ids` vide, tout part, la cascade désaffecte tout le monde (phase 4). Le
use case n'a **aucun cas particulier** à écrire pour ça — et c'est le signe que
la forme est juste.

```rust
pub enum UpdatePoolsSettingsError {
    SeasonNotFound,
    InvalidPools(DomainError),   // doublon de nom ou d'identifiant
    Repository(String),
}
```

> **Corrigé en phase 6.** Le refus des doublons vit dans
> `RankingGroupConfig::try_new()`, pas dans ce use case : le type est encapsulé
> et sa seule porte d'entrée valide. Le use case remonte l'erreur du domaine.

---

## 4 · `update_tiers_settings_use_case`

```rust
pub struct UpdateTiersSettingsCommand {
    pub season_id: SeasonId,
    pub tiers: Vec<TierRule>,
}

pub async fn execute(
    cmd: UpdateTiersSettingsCommand,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateTiersSettingsError>
```

Orchestration :

1. `find_base_info` → le nom de saison
2. `find_rules(&season_id)` → `SeasonNotFound`
3. **vérifier que seuls les coups de pouce et les stars ont changé** (voir
   ci-dessous) → `ImmutableFieldChanged`
4. `CompetitionRules { ranking_rules: current.ranking_rules, tiers: cmd.tiers }`
5. `save_rules(…)`

### Le contrôle des champs non éditables

Le panneau n'édite ni le nom, ni le budget, ni l'XP de départ, ni les rosters,
mais `TierRule` est un tout et les transporte quand même (phase 4). Le use case
compare donc, tier par tier, ce qui doit être identique :

> **Corrigé en phase 6.** Ce contrôle était écrit ici ; il répond à « est-ce
> autorisé ? » et vit donc dans le domaine :
> `CompetitionRules::with_inducements_from(tiers) -> Result<Self, DomainError>`.
> Le use case l'appelle et convertit l'erreur, il ne la calcule pas.

Le nombre de tiers doit être identique, et chaque tier conserver `name`,
`budget`, `starting_xp` et `rosters`. **Un écart est un refus, pas une
correction** : accepter la valeur reçue rendrait modifiable par requête forgée
ce que l'écran n'ouvre pas, et corriger silencieusement ferait croire à un
enregistrement qui n'a pas eu lieu.

**Un tier sans aucun coup de pouce est valide** (phase 4) : `Vec` vide accepté,
aucune borne basse.

```rust
pub enum UpdateTiersSettingsError {
    SeasonNotFound,
    TierCountChanged,
    ImmutableFieldChanged { tier_index: u8, field: &'static str },
    Repository(String),
}
```

`field` en `&'static str` et non en `String` : les seules valeurs possibles sont
les quatre noms de champs, connus à la compilation.

---

## 5 · `update_visibility_settings_use_case`

```rust
pub struct UpdateVisibilitySettingsCommand {
    pub season_id: SeasonId,
    pub access_mode: AccessMode,
    pub requires_validation: RequiresValidation,
}

pub async fn execute(
    cmd: UpdateVisibilitySettingsCommand,
    season_repo: &dyn ISeasonRepository,
) -> Result<(), UpdateVisibilitySettingsError>
```

Orchestration :

1. `find_invitations(&season_id)` → `SeasonNotFound`
2. remplacer les deux champs, **conserver `invited_coaches`,
   `registration_deadline` et `max_participants`**
3. `find_notifications(&season_id)` — `save_invitations` prend les deux
4. `save_invitations(&season_id, &invitations, &notifications)`

Le use case le plus simple des cinq, et **le seul qui doive relire deux
documents** : la signature de `save_invitations` porte aussi les notifications,
héritage de l'étape 4 du magicien où les deux se règlent ensemble. Ne pas les
relire les remettrait à leur valeur par défaut — les rappels d'échéance
s'éteindraient sans que rien ne le dise.

```rust
pub enum UpdateVisibilitySettingsError {
    SeasonNotFound,
    Repository(String),
}
```

---

## Aucun événement, et c'est un choix

Aucun des cinq use cases n'émet de domain event, donc aucun app event ne quitte
`competitions`.

`CompetitionsDomainEvent` compte quatre variantes — `CompetitionCreated`,
`CompetitionReady`, `PairingCreated`, `PairingDeleted` — toutes des **faits de
cycle de vie**. Un réglage modifié n'en est pas un.

Et le critère du `CLAUDE.md` le confirme : un événement sert à **propager un
effet** vers un BC qui doit réagir. Ici, personne n'a à réagir.

- Le classement se recalcule **dans le même POST**, par un port : c'est une
  commande synchrone, pas une propagation (phase 3).
- Les autres BCs ne cachent pas les réglages de compétition, ils les
  **consultent** par leurs ports au moment d'en avoir besoin — `match_report`
  demande les coups de pouce d'un tier par `find_tier_rules_for_roster`, pas une
  copie locale entretenue par événement. Un tier modifié est donc vu au match
  suivant, sans que rien n'ait à être publié.
- L'onglet Poules relit la structure à chaque ouverture.

**La seule raison qui justifierait un événement serait une projection locale
ailleurs.** Il n'y en a aucune, et cette phrase est ce qu'il faudra relire le
jour où quelqu'un en créera une.

---

## Règles métier tranchées

1. **Deux poules du même nom sont refusées** — `DuplicatePoolName`. Rien ne
   l'interdisait jusqu'ici, ni le domaine ni la base
   (`competition_groups` n'a pas d'unicité sur `(season_id, name)`), mais deux
   « Poule A » ne se distinguent pas dans un sélecteur d'affectation.

   Le refus vit dans le use case et non dans une contrainte de base : la base
   porte déjà des doublons possibles sur les saisons existantes, et une
   contrainte les rendrait immodifiables — on refuse d'en créer, on ne casse pas
   ce qui existe.

2. **Le plafond de participants disparaît du panneau.** Il ne réglait rien :
   `team_enrollment.rs` ne le lit pas, aucune inscription n'a jamais été
   refusée pour compétition pleine. Le champ quitte la maquette, le DTO, le VM
   et la commande.

   Le use case doit donc **relire `max_participants` avec le reste** : le
   panneau ne l'édite plus, mais `invitations` le porte toujours. La **carte
   415** le retire du modèle — jusqu'à ce qu'elle passe, ce champ traverse le
   use case sans être touché, comme `invited_coaches`.

## Ce qui reste ouvert

**Le barème peut changer pendant qu'un rapport de match est en cours de
saisie.** Le rapport ne lit le barème qu'à sa publication, par le listener qui
appelle `record_match_ranking_use_case` — un rapport ouvert au moment du
changement sera donc enregistré avec le **nouveau** barème, et le recalcul
l'aurait de toute façon rejoué avec. Aucune incohérence, mais c'est une
conséquence à connaître plutôt qu'à découvrir.
