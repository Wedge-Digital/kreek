# BC `teams` — Tracking du staff sur événements domaine internes

**Priorité : haute**
**Dépend de :** `51-team-created-staff-transport.md`
**Contexte :** `teams` — agrégat + phases post-match

## Objectif

Modéliser l'achat et le renvoi de staff pendant les phases post-match dans le BC `teams` :
- **Phase Recrutement** : achat de relances, assistants, cheerleaders (pas de fans)
- **Phase Renvois** : renvoi d'assistants et cheerleaders uniquement — **les relances ne peuvent pas être renvoyées**

Les value objects `RerollCount`, `AssistantCount`, `CheerleaderCount`, `ApothecaryCount`
sont définis en carte 51.

---

## Conception

### Nouvel événement domaine — `StaffDismissed`

```rust
// À ajouter dans TeamDomainEvent
StaffDismissed {
    staff_type:  StaffType,      // Reroll | Assistant | Cheerleader uniquement
    quantity:    u8,             // quantité renvoyée (u8 : nombre brut dans l'event store)
    refund_kpo:  Kpo,            // remboursement en trésorerie
},
```

**Note :** `quantity` reste `u8` dans le domain event (valeur brute persistée).
La validation de cohérence (quantity ≤ quantité détenue) est faite **avant** émission,
dans la commande `Team::dismiss_staff()`.

**Types autorisés au renvoi :** `Assistant`, `Cheerleader`, `Apothecary`.
`Reroll` et `FansFactor` ne peuvent pas être renvoyés.

### `Team` agrégat — champs staff (issus de carte 51)

```rust
pub rerolls:      RerollCount,
pub apothecaries: ApothecaryCount,
pub assistants:   AssistantCount,
pub cheerleaders: CheerleaderCount,
```

### `Team::apply(StaffBought)` — compléter l'existant

```rust
TeamDomainEvent::StaffBought { staff_type, quantity, cost_kpo } => {
    match staff_type {
        StaffType::Reroll      => self.rerolls.0      = self.rerolls.0.saturating_add(*quantity),
        StaffType::Apothecary  => self.apothecaries.0 = self.apothecaries.0.saturating_add(*quantity),
        StaffType::Assistant   => self.assistants.0   = self.assistants.0.saturating_add(*quantity),
        StaffType::Cheerleader => self.cheerleaders.0 = self.cheerleaders.0.saturating_add(*quantity),
        StaffType::FansFactor  => {}
    }
    self.team_value.0 += cost_kpo.0;
    self.treasury.0    = self.treasury.0.saturating_sub(cost_kpo.0);
}
```

### `Team::apply(StaffDismissed)` — nouveau

```rust
TeamDomainEvent::StaffDismissed { staff_type, quantity, refund_kpo } => {
    match staff_type {
        StaffType::Apothecary  => self.apothecaries.0 = self.apothecaries.0.saturating_sub(*quantity),
        StaffType::Assistant   => self.assistants.0   = self.assistants.0.saturating_sub(*quantity),
        StaffType::Cheerleader => self.cheerleaders.0 = self.cheerleaders.0.saturating_sub(*quantity),
        _                      => {}   // Reroll, FansFactor : non renvoyables
    }
    self.team_value.0  = self.team_value.0.saturating_sub(refund_kpo.0);
    self.treasury.0   += refund_kpo.0;
}
```

### Commandes domaine

```rust
// Phase Recrutement — types autorisés : Reroll, Assistant, Cheerleader
pub fn buy_staff(
    &self,
    staff_type: StaffType,
    quantity:   u8,
    cost_kpo:   Kpo,
) -> Result<TeamDomainEvent, DomainError>

// Phase Renvois — types autorisés : Reroll, Assistant, Cheerleader
pub fn dismiss_staff(
    &self,
    staff_type:  StaffType,
    quantity:    u8,
    refund_kpo:  Kpo,
) -> Result<TeamDomainEvent, DomainError>
```

Gardes :
- `buy_staff` : `game_phase == Recruitment`, type autorisé, budget suffisant
- `dismiss_staff` : `game_phase == Dismissals`, type ∈ {`Assistant`, `Cheerleader`, `Apothecary`}, `quantity <= quantité détenue`

---

## Checklist

- [ ] Ajouter `StaffDismissed` à `TeamDomainEvent` + `type_name()` + `schema_version()`
- [ ] `Team::apply(StaffBought)` : incrémenter le compteur via `value_object.0`
- [ ] `Team::apply(StaffDismissed)` : décrémenter + ajuster trésorerie / TV
- [ ] `Team::buy_staff()` : garde phase Recrutement + types autorisés
- [ ] `Team::dismiss_staff()` : garde phase Renvois + vérification quantité disponible
- [ ] Use case `buy_staff.rs` + `dismiss_staff.rs` dans `teams/use_cases/`
- [ ] Tests unitaires : achat staff hors phase → erreur
- [ ] Tests unitaires : renvoi > quantité détenue → erreur
- [ ] Tests unitaires : buy + dismiss → compteurs et trésorerie cohérents
