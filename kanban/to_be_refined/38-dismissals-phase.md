# BC `teams` — Phase de renvois

**Priorité : haute**
**Dépend de :** `37-recruitment-phase.md`
**Contexte :** `teams` — action coach

## Objectif

Permettre au coach de renvoyer des joueurs pendant la phase `Dismissals`, puis de valider pour passer à la phase de retraite temporaire.

---

## Conception

### Règles BB2020

- Le coach peut renvoyer n'importe quel joueur de son effectif
- Un joueur renvoyé libère son salaire mais ne rapporte rien à la trésorerie
- On ne peut pas revenir en arrière sur un renvoi une fois validé

### Commandes et use cases

```rust
pub struct DismissPlayerCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
}

pub struct ValidateDismissalsPhaseCommand {
    pub team_id: TeamId,
}
// → team.advance_game_phase() → GamePhase::TemporaryRetirement
```

### Route et UI

```
POST /app/{space_id}/teams/{team_id}/dismiss-player
POST /app/{space_id}/teams/{team_id}/validate-dismissals
GET  /app/{space_id}/teams/{team_id}/dismissals-phase
```

L'UI affiche l'effectif complet avec un bouton "Renvoyer" par joueur et un bouton "Valider les renvois" en bas.

---

### Calcul de `value_kpo_at_firing`

Le use case `DismissPlayer` calcule la valeur courante du joueur **depuis l'event store de BC `teams`** avant d'appender l'event :

```
value_kpo_at_firing = base_value_kpo (PlayerRecruited)
                    + Σ value_delta  (PlayerImprovementApplied pour ce player_id)
```

Aucune requête vers BC `players` — BC `teams` est auto-suffisant pour ce calcul.

```rust
pub struct DismissPlayerCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
}
// Use case :
//   1. find_by_id() → rejoue les events
//   2. calcule value_kpo_at_firing depuis les events filtrés par player_id
//   3. append PlayerFired { player_id, value_kpo_at_firing }
```

---

## Points en suspens

- Un joueur renvoyé génère-t-il un app event vers BC `players` (pour archiver sa fiche) ?
- Les joueurs gravement blessés sont-ils mis en avant dans l'UI pour aide à la décision ?

---

## Checklist

- [ ] `DismissPlayerCommand` + use case : calcule `value_kpo_at_firing` depuis events → append `PlayerFired { value_kpo_at_firing }`
- [ ] `ValidateDismissalsPhaseCommand` + use case → `advance_game_phase()`
- [ ] Routes GET + POST dans `router.rs`
- [ ] Fragment UI renvois avec liste joueurs
- [ ] Publication app event `PlayerDismissed` vers BC `players` (TBD)
