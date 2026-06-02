# BC `teams` — Exclusion d'équipe par l'admin → "Renvoyée"

**Priorité : moyenne**
**Dépend de :** `32-team-enrollment.md`
**Contexte :** `teams` — action admin

## Objectif

Permettre à un admin d'exclure une équipe inscrite dans une ligue, faisant passer son statut de `Enrolled` à `Dismissed`.

---

## Conception

### Commande et use case

```rust
pub struct DismissTeamCommand {
    pub team_id:  TeamId,
    pub space_id: SpaceId,
    pub admin_id: UserId,
}

pub enum DismissTeamError {
    TeamNotFound,
    Domain(DomainError),
    Repository(RepositoryError),
}
```

### Pattern event sourcing

Le use case :
1. Charge l'agrégat via `team_repo.find_by_id()` (rejeu des événements)
2. Appelle `team.dismiss()` → retourne `TeamDomainEvent::TeamDismissed` ou `DomainError`
3. Appende l'événement via `team_repo.append(&team_id, &event, team.version)`

```rust
pub fn dismiss(&self) -> Result<TeamDomainEvent, DomainError> {
    match self.participation_status {
        ParticipationStatus::Enrolled => Ok(TeamDomainEvent::TeamDismissed),
        ParticipationStatus::Dismissed => Err(DomainError::AlreadyDismissed),
        _ => Err(DomainError::InvalidTransition { ... }),
    }
}
```

### Route et handler

```
POST /app/{space_id}/teams/{team_id}/dismiss
```

Accessible uniquement aux admins. Réponse : `HX-Refresh: true`.

---

## Checklist

- [ ] `DismissTeamCommand` dans `commands.rs`
- [ ] Use case `dismiss_team.rs` : load → `team.dismiss()` → `append(TeamDismissed)`
- [ ] `Team::dismiss()` retourne `Result<TeamDomainEvent, DomainError>`
- [ ] Route `DISMISS_TEAM` dans `routes.rs` + handler POST
- [ ] Vérification rôle admin dans le handler
- [ ] Test unitaire : `Team::dismiss()` sur agrégat hydraté
- [ ] Test d'intégration : use case complet → événement en base
