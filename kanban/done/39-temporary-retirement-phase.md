# BC `teams` — Phase de retraite temporaire

**Priorité : haute**
**Dépend de :** `38-dismissals-phase.md`
**Contexte :** `teams` — action coach

## Objectif

Permettre au coach de mettre un ou plusieurs joueurs en retraite temporaire pendant la phase `TemporaryRetirement`, puis de valider pour déclencher le calcul automatique des erreurs couteuses.

---

## Règles métier

- Un joueur en retraite temporaire est **indisponible pour le reste de la saison** (pas uniquement le prochain match)
- Il **compte toujours dans les quotas** de joueurs tout au long de la saison — on ne peut pas en recruter un autre à sa place
- Il peut être **renvoyé** pendant la phase de renvois (carte 38) pour libérer sa place dans le quota
- Sa situation est réexaminée en **repos hors-saison** (carte 43) : le coach choisit alors de le réengager ou non pour la saison suivante

---

## Conception

### Événements domaine produits

```rust
TeamDomainEvent::PlayerRetiredTemporarily { player_id: String }
TeamDomainEvent::RetirementPhaseValidated
```

### Commandes et use cases

```rust
pub struct SetTemporaryRetirementCommand {
    pub team_id:   TeamId,
    pub player_id: PlayerId,
    pub retired:   bool,   // true = mise en retraite, false = annulation
}

pub struct ValidateRetirementPhaseCommand {
    pub team_id: TeamId,
}
// → déclenche apply_costly_mistakes() automatiquement (carte 40)
```

### Route et UI

```
POST /app/{space_id}/teams/{team_id}/set-retirement
POST /app/{space_id}/teams/{team_id}/validate-retirement
GET  /app/{space_id}/teams/{team_id}/retirement-phase
```

L'UI doit clairement indiquer que la retraite temporaire vaut pour le reste de la saison, et que le joueur continuera à occuper un slot de quota. Elle doit aussi rappeler que le renvoyer reste possible (renvoi effectué à l'étape précédente).

### Enchaînement automatique après validation

La validation déclenche directement le calcul des erreurs couteuses (carte 40) puis repasse l'équipe en `ReadyToPlay`.

---

## Points en suspens

- Y a-t-il des contraintes sur qui peut partir en retraite temporaire (ex. : uniquement les joueurs blessés, ou tout joueur) ?

---

## Checklist

- [ ] `TeamDomainEvent::PlayerRetiredTemporarily` + `RetirementPhaseValidated`
- [ ] `SetTemporaryRetirementCommand` + use case (garde : phase correcte, joueur dans l'équipe)
- [ ] `ValidateRetirementPhaseCommand` + use case → enchaîne vers carte 40
- [ ] `Team::apply()` mis à jour pour les deux nouveaux variants
- [ ] `update_projection_in_tx()` mis à jour (carte 42) — version uniquement, état joueur géré par BC players
- [ ] Routes GET + POST
- [ ] Fragment UI avec avertissement quota + possibilité de renvoi rappelée
