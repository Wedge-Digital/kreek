# Phase 5 — Use cases — team-detail

## Dépendance Phase 6 (à noter, pas à résoudre ici)

L'agrégat `Player` (`domain/player.rs`) n'a aujourd'hui **ni** `personal_name`
**ni** `display_order` comme champs — seule la projection les porte
(`personal_name` toujours écrit `""`). Le use case ci-dessous compare l'état
courant de l'agrégat à la commande soumise pour ne muter que ce qui a changé
: la Phase 6 devra donc ajouter ces deux champs à `Player`, avec les
variantes d'événement correspondantes rejouées à l'hydratation (`apply()`).
`jersey: Option<JerseyVo>` existe déjà, rien à ajouter pour ce champ-là.

## Dépendance Phase 7 (port)

Nouvelle méthode sur `IPlayerRepository`, décidée pour honorer l'atomicité du
batch actée en Phase 2 :

```rust
async fn append_batch(
    &self,
    entries: Vec<(PlayerId, TeamId, PlayerDomainEvent, i32)>,
) -> Result<(), RepositoryError>;
```

Implémentation par défaut (trait `async_trait`) : boucle d'`append()`
séquentiels — **aucune fausse implémentation de test à modifier**. Seule
`PgPlayerRepository` la surcharge (Phase 7) pour ouvrir une transaction
unique enveloppant tous les inserts (mêmes fonctions déjà existantes
`insert_player_event`/`upsert_player_projection`, appelées en boucle sur un
`&mut Transaction` partagé au lieu d'un par joueur).

## Signature

```rust
// players/use_cases/update_roster_use_case.rs

pub enum UpdateRosterError {
    /// Un player_id soumis ne correspond à aucun joueur Active de l'équipe.
    UnknownOrInactivePlayer,
    /// Deux lignes du batch soumis portent le même numéro de maillot.
    DuplicateJersey,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: UpdateRosterCommand,
    player_repo: &dyn IPlayerRepository,
    event_bus: &EventBus,
) -> Result<Vec<Player>, UpdateRosterError>
```

Le résultat `Vec<Player>` (effectif actif à jour) sert directement au handler
pour reconstruire `PlayerTableTemplate` sans second aller-retour repository.

## Orchestration

1. **Charger** l'effectif actif de l'équipe : `player_repo.find_by_team_id(&cmd.team_id)`, filtré `membership == Active`.
2. **Valider l'appartenance** : chaque `row.player_id` du batch doit correspondre à un joueur de cet effectif — sinon `UnknownOrInactivePlayer`, tout le batch est rejeté (rien n'est persisté).
3. **Valider l'unicité** des numéros de maillot **au sein du batch soumis** (niveau use case, cf. 03-back.md) — sinon `DuplicateJersey`, tout le batch est rejeté.
4. **Diff par joueur** : pour chaque ligne du batch, comparer `personal_name`/`jersey`/`display_order` soumis à l'état actuel de l'agrégat. N'appeler la méthode de domaine correspondante (Phase 6) que pour les champs réellement différents — un joueur inchangé ne produit aucun événement.
5. **Persister** : accumuler tous les événements produits (potentiellement plusieurs par joueur) dans un seul appel `append_batch`, avec version incrémentée localement par joueur (`player.version + 1`, `+2`, ... selon le nombre de champs modifiés pour ce joueur).
6. **Émettre** chaque événement sur l'`event_bus` (même pattern que `increase_stat_use_case`), après le succès de la persistance.
7. **Retourner** l'effectif rechargé (ou construit en mémoire à partir des mutations appliquées — détail d'implémentation Phase 8).

## Règle métier assumée à cette étape

**Un joueur actif absent du batch soumis est laissé inchangé, ce n'est pas une erreur.** Le batch n'est pas comparé à l'effectif complet pour détecter des lignes manquantes — seules les lignes présentes sont traitées. Cohérent avec la maquette (toute la grille est toujours soumise en pratique, puisque le formulaire encadre tout `#roster-tbody`), mais le use case ne l'impose pas strictement.
