# Panneau « Poules »

**Épic :** E14 · **Ordre :** 3 · **Dépend de :** 417, 420
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/`
(`03-back.md`, `05-use-cases.md`)

## Objectif

Renommer, ajouter et retirer des poules sur une saison en cours. Retirer une
poule désaffecte ses équipes ; leurs points ne bougent pas.

## Le piège que cette carte doit refermer

**Les poules vivent à deux endroits.**

| Où | Quoi |
|---|---|
| `structure.ranking_group.ranking_groups` (JSONB) | la déclaration |
| `competition_groups` (table) | la même liste, matérialisée |
| `competition_group_teams` (table) | **l'affectation des équipes** |

La table est alimentée **paresseusement** : `groups_widgets.rs:158` projette le
JSONB à chaque ouverture de l'onglet Poules. Et cette projection est un
`INSERT … ON CONFLICT DO UPDATE` — **elle ne supprime jamais**.

Une poule retirée du JSONB garderait donc sa ligne et ses équipes affectées :
le retrait serait cosmétique. Pire, la projection est gardée par
`if !struct_groups.is_empty()` — retirer **toutes** les poules, ce qui est
autorisé, ne déclencherait aucune écriture du tout.

`competition_group_teams` a un `ON DELETE CASCADE` vers `competition_groups` :
la désaffectation est gratuite **dès qu'on supprime la ligne de poule**, et
uniquement à ce moment.

## Conception

### La méthode de dépôt

```rust
// ISeasonRepository
/// Écrit la structure et supprime les poules absentes de `kept_ids`, dans une
/// seule transaction. Rend le nombre d'affectations défaites par la cascade.
async fn save_structure_and_prune_groups(
    &self, season_id: &SeasonId, structure: &CompetitionStructure, kept_ids: &[String],
) -> Result<u64, SeasonRepositoryError>;
```

**L'atomicité est dans le dépôt, pas dans le use case** : une transaction sqlx
ne se partage pas entre deux ports sans faire entrer sqlx dans une couche qui
n'en veut pas. Le projet fait déjà ainsi — `competition_repository.rs:46`,
`match_day_repository.rs:169`.

**Sur `ISeasonRepository` et non sur le port des poules** : c'est l'écriture de
la structure qui commande, la suppression n'en est que la conséquence.

### Le use case

```rust
pub struct UpdatePoolsSettingsCommand {
    pub season_id: SeasonId,
    pub use_pools: UseRankingGroups,
    pub pools: Vec<PoolInput>,
}
pub struct PoolInput { pub id: Option<RankingGroupId>, pub name: RankingGroupName }
pub struct PoolsSettingsOutcome { pub unassigned_teams: u32 }
```

1. `find_structure` → `SeasonNotFound`
2. identifiant aux poules neuves via `IdService::generate_id()`
3. `RankingGroupConfig::try_new(…)` → `InvalidPools(DomainError)`
4. remplacer `ranking_group`, **conserver `schedule` et `play_offs_phase`**
5. `save_structure_and_prune_groups(…)`

**L'identifiant est engendré côté serveur**, à rebours du magicien qui le laisse
fabriquer par le navigateur (`new-competition-phase-3.html:235`, `genId()`). Un
identifiant de domaine minté par le client n'est contrôlé ni en forme, ni en
unicité, ni en provenance.

**Le refus des doublons vient du domaine** (carte 417), pas de ce use case.

**Retirer toutes les poules n'est pas un cas particulier** : `kept_ids` vide,
tout part, la cascade désaffecte tout le monde. Aucune branche à écrire — et
c'est le signe que la forme est juste.

### Le handler

```rust
GET  …/settings/pools  → get_settings_pools
POST …/settings/pools  → post_settings_pools   (axum_extra::extract::Form)
```

```rust
#[derive(Deserialize)]
pub struct PoolsSettingsForm {
    #[serde(default)] pub use_pools: bool,
    #[serde(default)] pub pool_id: Vec<String>,     // vide = poule neuve
    #[serde(default)] pub pool_name: Vec<String>,
}
```

**`axum_extra` et non `axum`** : celui d'axum s'appuie sur `serde_urlencoded`,
qui refuse les clés répétées et rend un 422 (`invalid type: string, expected a
sequence`). Précédent et commentaire : `roster_edition_controller.rs:20`.

**Invariant à vérifier dans le handler** : `pool_id.len() == pool_name.len()`.
Un écart est un `400`, jamais un `zip` — qui s'arrête sur la plus courte et
perdrait une poule sans rien dire.

**Aucune liste parallèle ne vient d'une case à cocher** : une case décochée
n'est pas soumise, et les deux `Vec` se désynchroniseraient dès la première.
C'est une contrainte sur le gabarit autant que sur le DTO.

### Le VM

```rust
pub struct PoolsVm { pub use_pools: bool, pub pools: Vec<PoolRowVm> }
pub struct PoolRowVm { pub id: String, pub name: String, pub assigned_teams: u32 }
```

`assigned_teams` vient de `competition_group_teams`, **pas du JSONB** — c'est le
seul endroit qui sait qui joue où. Construit dans `builders.rs`. C'est ce
compteur qui alimente le pied de panneau, « 6 équipes à réaffecter », et il
serait faux s'il était lu dans la déclaration.

### Le template

Une poule marquée « à retirer » n'est **pas** supprimée côté serveur : elle est
barrée dans le formulaire et n'est pas soumise. Tant qu'on n'a pas enregistré,
rien n'est défait, et « Rétablir » n'est qu'un changement d'état local. C'est ce
qui permet de montrer la conséquence sans l'avoir provoquée.

## Tests

- Unitaires : le calendrier préservé, le retrait total autorisé, l'identifiant
  engendré pour une poule neuve, le refus d'un doublon remonté du domaine.
- Intégration : `save_structure_and_prune_groups` est atomique, et la cascade
  rend le bon compte d'affectations défaites.
- E2E : retirer une poule désaffecte ses équipes (vérifié dans l'onglet Poules) ;
  retirer toutes les poules ; **le calendrier survit à l'enregistrement**.

Le dernier est le plus important : son échec ne produirait aucune erreur, juste
un calendrier vide découvert des jours plus tard.

## Checklist

- [ ] `save_structure_and_prune_groups` + son test d'intégration
- [ ] Le use case et ses tests
- [ ] Les deux handlers, `require_admin_access`, l'invariant de longueur
- [ ] Le VM, `builders.rs`, le template et son état « à retirer »
- [ ] `make lint && make test && make check-arch`
