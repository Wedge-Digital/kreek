# BC `teams` — Tracking du staff sur événements domaine internes

**Priorité : haute**
**Dépend de :** `51-team-created-staff-transport.md`
**Contexte :** `teams` — agrégat + phases post-match

## Objectif

Modéliser l'achat et le renvoi de staff pendant les phases post-match dans le BC `teams` :
- **Phase Recrutement** : achat de relances, assistants, cheerleaders (pas de fans)
- **Phase Renvois** : renvoi des mêmes types (cohérent avec le renvoi de joueurs)

---

## Conception

### Nouvel événement domaine — `StaffDismissed`

```rust
// À ajouter dans TeamDomainEvent
StaffDismissed {
    staff_type: StaffType,   // Reroll | Assistant | Cheerleader uniquement
    quantity:   u8,
    refund_kpo: Kpo,         // remboursement en trésorerie (règles à confirmer)
},
```

**Types autorisés au renvoi :** `Reroll`, `Assistant`, `Cheerleader`.
`Apothecary` et `FansFactor` ne peuvent pas être renvoyés via cette phase.

### Contraintes domaine à valider dans `Team`

**Achat de staff (phase Recrutement)** — garde sur `game_phase == Recruitment` :
- `Reroll`, `Assistant`, `Cheerleader` autorisés
- Pas de `FansFactor` en post-match

**Renvoi de staff (phase Renvois)** — garde sur `game_phase == Dismissals` :
- Vérifier que `quantité_renvoyée <= quantité_détenue`

### `Team` agrégat — mise à jour de `apply()`

```rust
// StaffBought (déjà existant — compléter)
TeamDomainEvent::StaffBought { staff_type, quantity, cost_kpo } => {
    match staff_type {
        StaffType::Reroll      => self.rerolls      += quantity,
        StaffType::Apothecary  => self.apothecaries += quantity,
        StaffType::Assistant   => self.assistants   += quantity,
        StaffType::Cheerleader => self.cheerleaders += quantity,
        StaffType::FansFactor  => {}  // pas de champ dédié
    }
    self.team_value.0 += cost_kpo.0;
    self.treasury.0    = self.treasury.0.saturating_sub(cost_kpo.0);
}

// StaffDismissed (nouveau)
TeamDomainEvent::StaffDismissed { staff_type, quantity, refund_kpo } => {
    match staff_type {
        StaffType::Reroll      => self.rerolls      = self.rerolls.saturating_sub(*quantity),
        StaffType::Assistant   => self.assistants   = self.assistants.saturating_sub(*quantity),
        StaffType::Cheerleader => self.cheerleaders = self.cheerleaders.saturating_sub(*quantity),
        _                      => {}
    }
    self.team_value.0  = self.team_value.0.saturating_sub(refund_kpo.0);
    self.treasury.0   += refund_kpo.0;
}
```

### `Team::buy_staff()` et `Team::dismiss_staff()` — nouvelles commandes

```rust
pub fn buy_staff(
    &self,
    staff_type: StaffType,
    quantity:   u8,
    cost_kpo:   Kpo,
) -> Result<TeamDomainEvent, DomainError>

pub fn dismiss_staff(
    &self,
    staff_type:  StaffType,
    quantity:    u8,
    refund_kpo:  Kpo,
) -> Result<TeamDomainEvent, DomainError>
```

---

## Checklist

- [ ] Ajouter `StaffDismissed` à `TeamDomainEvent` + `type_name()` + `schema_version()`
- [ ] `Team::apply(StaffBought)` : incrémenter le compteur du type correspondant
- [ ] `Team::apply(StaffDismissed)` : décrémenter + ajuster trésorerie / TV
- [ ] `Team::buy_staff()` : garde phase Recrutement + types autorisés
- [ ] `Team::dismiss_staff()` : garde phase Renvois + vérification quantité disponible
- [ ] Use case `buy_staff.rs` + `dismiss_staff.rs` dans `teams/use_cases/`
- [ ] Tests unitaires : achat staff hors phase → erreur, renvoi > quantité détenue → erreur
- [ ] Tests unitaires : buy + dismiss → compteurs et trésorerie cohérents
