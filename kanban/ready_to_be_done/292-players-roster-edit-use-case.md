# BC `players` — Use case : update_roster

**Priorité : haute**
**Dépend de :** `290-players-roster-edit-domain.md`, `291-players-roster-edit-persistence.md`
**Contexte :** `players` — use case

## Objectif

Orchestrer la mise à jour en batch de l'effectif : validation
d'appartenance, unicité (numéro et ordre) sur l'effectif actif complet — pas
seulement le batch soumis —, diff par champ (un champ inchangé n'émet aucun
événement), persistance atomique.

**Spec de référence :** `docs/specs/player-edition/team-detail/05-use-cases.md`.

---

## Conception

### Commande (`players/use_cases/commands.rs`)

```rust
pub struct UpdateRosterCommand {
    pub team_id: TeamId,
    pub space_id: SpaceId,
    pub rows: Vec<RosterRowCommand>,
}
pub struct RosterRowCommand {
    pub player_id: PlayerId,
    pub personal_name: Option<PersonalName>,
    pub jersey: Option<JerseyVo>,
    pub display_order: DisplayOrder,
}
```

### Signature (`players/use_cases/update_roster_use_case.rs`)

```rust
pub enum UpdateRosterError {
    UnknownOrInactivePlayer,
    DuplicateJersey,
    DuplicateDisplayOrder,
    Domain(DomainError),
    Repository(RepositoryError),
}

pub async fn execute(
    cmd: UpdateRosterCommand,
    player_repo: &dyn IPlayerRepository,
    event_bus: &EventBus,
) -> Result<Vec<Player>, UpdateRosterError>
```

### Orchestration

1. Charger l'effectif actif : `find_by_team_id`, filtré `membership == Active`.
2. Valider l'appartenance : chaque `row.player_id` doit être dans cet
   effectif — sinon `UnknownOrInactivePlayer`, rien n'est persisté.
3. Valider l'unicité (numéro **et** display_order) sur l'**état résultant
   complet** : pour chaque joueur actif, sa valeur soumise si présente dans
   le batch, sinon sa valeur actuelle inchangée. Un joueur `Dismissed`
   n'entre jamais dans ce calcul (déjà exclu à l'étape 1). Conflit →
   `DuplicateJersey`/`DuplicateDisplayOrder`, rien n'est persisté.
4. Diff par joueur : n'appeler `rename`/`change_jersey`/`reorder` que pour
   les champs réellement différents de l'état courant.
5. Accumuler tous les événements produits dans un seul `append_batch`,
   version incrémentée localement par joueur selon le nombre de champs
   modifiés pour ce joueur.
6. Émettre chaque événement sur l'`event_bus` (même pattern que
   `increase_stat_use_case`), après succès de la persistance.
7. Retourner l'effectif à jour.

---

## Checklist

- [ ] `UpdateRosterCommand` / `RosterRowCommand` dans `commands.rs`
- [ ] `update_roster_use_case::execute()`
- [ ] Test : `update_roster_rejects_unknown_or_inactive_player_and_persists_nothing`
- [ ] Test : `update_roster_rejects_duplicate_jersey_against_untouched_active_player`
- [ ] Test : `update_roster_rejects_duplicate_display_order_against_untouched_active_player`
- [ ] Test : `update_roster_ignores_dismissed_player_when_checking_uniqueness`
- [ ] Test : `update_roster_only_emits_events_for_changed_fields`
- [ ] Test : `update_roster_leaves_players_absent_from_batch_untouched`
- [ ] Test : `update_roster_propagates_concurrent_write_as_is`
