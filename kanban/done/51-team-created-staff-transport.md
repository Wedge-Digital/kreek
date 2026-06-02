# BC `teams` — Transport du staff initial depuis `team_creation`

**Priorité : haute**
**Dépend de :** `30-team-created-app-event.md` (carte terminée — ne pas modifier)
**Contexte :** `team_creation` (émetteur) → `teams` (consommateur)

## Objectif

Enrichir le `TeamCreationAppEvent::TeamCreated` et le `TeamCreationDomainEvent::TeamSubmitted`
avec le staff et les relances achetés pendant la phase de construction, afin que le BC `teams`
puisse initialiser l'agrégat avec l'état de staff réel dès sa création.

---

## Value objects à créer dans `teams/domain/value_objects.rs`

```rust
/// Nombre de relances (0–8)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RerollCount(pub u8);
impl RerollCount {
    pub const MAX: u8 = 8;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX { Ok(Self(n)) } else { Err("RerollCount max 8") }
    }
}

/// Nombre d'assistants (0–6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct AssistantCount(pub u8);
impl AssistantCount {
    pub const MAX: u8 = 6;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX { Ok(Self(n)) } else { Err("AssistantCount max 6") }
    }
}

/// Nombre de cheerleaders (0–6)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct CheerleaderCount(pub u8);
impl CheerleaderCount {
    pub const MAX: u8 = 6;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX { Ok(Self(n)) } else { Err("CheerleaderCount max 6") }
    }
}

/// Présence d'un apothicaire (0 ou 1)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct ApothecaryCount(pub u8);
impl ApothecaryCount {
    pub const MAX: u8 = 1;
    pub fn new(n: u8) -> Result<Self, &'static str> {
        if n <= Self::MAX { Ok(Self(n)) } else { Err("ApothecaryCount max 1") }
    }
    pub fn has(&self) -> bool { self.0 == 1 }
}
```

Ces types sont utilisés dans les trois couches (domain event, app event, agrégat).  
Dans les app events (frontière BC), les champs restent `u8` pour la sérialisation —  
la conversion vers les value objects se fait dans le listener.

---

## Conception

### Données à transporter

Au moment de `submit_team`, le `RosterSelectedTeam` expose :
- `hired_staff: Vec<TeamStaff>` — liste des achats (type + quantité implicite par répétition)
- `reroll_count: u8` — nombre de relances

Agréger par type pour obtenir les quantités, construire les value objects :

```rust
// Dans submit_team.rs
let rerolls      = RerollCount::new(team.reroll_count()).unwrap_or_default();
let apothecaries = ApothecaryCount::new(count_staff(&team, StaffKind::Apothecary)).unwrap_or_default();
let assistants   = AssistantCount::new(count_staff(&team, StaffKind::CoachAssistant)).unwrap_or_default();
let cheerleaders = CheerleaderCount::new(count_staff(&team, StaffKind::Cheerleaders)).unwrap_or_default();
```

### `TeamCreationDomainEvent::TeamSubmitted` — champs ajoutés

```rust
rerolls:      RerollCount,
apothecaries: ApothecaryCount,
assistants:   AssistantCount,
cheerleaders: CheerleaderCount,
```

### `TeamCreationAppEvent::TeamCreated` — champs ajoutés (u8 à la frontière BC)

```rust
rerolls:      u8,   // sérialisé — conversion vers RerollCount dans le listener
apothecaries: u8,
assistants:   u8,
cheerleaders: u8,
```

### `TeamDomainEvent::TeamCreated` — champs ajoutés

```rust
rerolls:      RerollCount,
apothecaries: ApothecaryCount,
assistants:   AssistantCount,
cheerleaders: CheerleaderCount,
```

### `team_created_listener.rs` — conversion et propagation

```rust
let TeamCreationAppEvent::TeamCreated { rerolls, apothecaries, assistants, cheerleaders, .. } = app_event;
let domain_event = TeamDomainEvent::TeamCreated {
    rerolls:      RerollCount::new(rerolls).unwrap_or_default(),
    apothecaries: ApothecaryCount::new(apothecaries).unwrap_or_default(),
    assistants:   AssistantCount::new(assistants).unwrap_or_default(),
    cheerleaders: CheerleaderCount::new(cheerleaders).unwrap_or_default(),
    ..
};
```

### `Team::apply(TeamCreated)` — initialisation

```rust
self.rerolls      = *rerolls;
self.apothecaries = *apothecaries;
self.assistants   = *assistants;
self.cheerleaders = *cheerleaders;
```

---

## Checklist

- [ ] Définir `RerollCount`, `AssistantCount`, `CheerleaderCount`, `ApothecaryCount` dans `teams/domain/value_objects.rs`
- [ ] Ajouter les 4 champs (value objects) dans `TeamCreationDomainEvent::TeamSubmitted`
- [ ] Ajouter les 4 champs (`u8`) dans `TeamCreationAppEvent::TeamCreated`
- [ ] `submit_team.rs` : calculer les quantités, construire les value objects, les passer à l'événement
- [ ] Ajouter les 4 champs (value objects) dans `TeamDomainEvent::TeamCreated`
- [ ] `team_created_listener.rs` : convertir `u8` → value objects, propager
- [ ] `Team` agrégat : ajouter les 4 champs value objects + initialisation dans `Team::apply(TeamCreated)`
- [ ] Mettre à jour les tests qui construisent `TeamDomainEvent::TeamCreated`
