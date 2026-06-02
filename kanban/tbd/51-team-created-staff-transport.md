# BC `teams` — Transport du staff initial depuis `team_creation`

**Priorité : haute**
**Dépend de :** `30-team-created-app-event.md` (carte terminée — ne pas modifier)
**Contexte :** `team_creation` (émetteur) → `teams` (consommateur)

## Objectif

Enrichir le `TeamCreationAppEvent::TeamCreated` et le `TeamCreationDomainEvent::TeamSubmitted`
avec le staff et les relances achetés pendant la phase de construction, afin que le BC `teams`
puisse initialiser l'agrégat avec l'état de staff réel dès sa création.

---

## Conception

### Données à transporter

Au moment de `submit_team`, le `RosterSelectedTeam` expose :
- `hired_staff: Vec<TeamStaff>` — liste des achats (type + quantité implicite par répétition)
- `reroll_count: u8` — nombre de relances

Agréger par type pour obtenir les quantités :

```rust
// Dans submit_team.rs — calcul avant émission de l'événement
let rerolls     = team.reroll_count();
let apothecaries = count_staff(&team, StaffKind::Apothecary);
let assistants  = count_staff(&team, StaffKind::CoachAssistant);
let cheerleaders = count_staff(&team, StaffKind::Cheerleaders);
// fans_factor : non géré en recrutement post-match — mais initialisé à la création
// via dedicated_fans (déjà dans TeamCreated)
```

### `TeamCreationDomainEvent::TeamSubmitted` — champs ajoutés

```rust
rerolls:      u8,
apothecaries: u8,
assistants:   u8,
cheerleaders: u8,
```

### `TeamCreationAppEvent::TeamCreated` — champs ajoutés

```rust
rerolls:      u8,
apothecaries: u8,
assistants:   u8,
cheerleaders: u8,
```

### `TeamDomainEvent::TeamCreated` — champs ajoutés

```rust
rerolls:      u8,
apothecaries: u8,
assistants:   u8,
cheerleaders: u8,
```

### `team_created_listener.rs` — propagation

Lire les nouveaux champs de l'app event et les injecter dans `TeamDomainEvent::TeamCreated`.

### `Team::apply(TeamCreated)` — initialisation

```rust
self.rerolls      = rerolls;
self.apothecaries = apothecaries;
self.assistants   = assistants;
self.cheerleaders = cheerleaders;
```

---

## Checklist

- [ ] Ajouter les 4 champs dans `TeamCreationDomainEvent::TeamSubmitted`
- [ ] Ajouter les 4 champs dans `TeamCreationAppEvent::TeamCreated`
- [ ] `submit_team.rs` : calculer les quantités depuis `RosterSelectedTeam` et les passer à l'événement
- [ ] Ajouter les 4 champs dans `TeamDomainEvent::TeamCreated`
- [ ] `team_created_listener.rs` : propager les 4 champs vers `TeamDomainEvent::TeamCreated`
- [ ] `Team` agrégat : ajouter les 4 champs + initialisation dans `Team::apply(TeamCreated)`
- [ ] Mettre à jour les tests qui construisent `TeamDomainEvent::TeamCreated` (ajout de champs)
