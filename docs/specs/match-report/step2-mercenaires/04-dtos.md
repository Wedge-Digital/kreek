# Step 2 — Mercenaires — Contrats de données (DTOs)

## DTOs d'entrée (command side)

### InducementsForm (modifié — inducements_controller.rs)

```rust
#[derive(Deserialize)]
pub struct InducementsForm {
    #[serde(default)]
    pub selection: String,      // existant — JSON [{uid, qty, unit_cost}]
    #[serde(default)]
    pub mercenaries: String,    // NOUVEAU — JSON [{position_uid, level}]
}
```

**Émis par** : formulaire HTML (`<form method="post">` dans `inducements.html`)
**Consommé par** : handler POST `inducements_controller::post_inducements`

---

### MercenaryPurchaseCmd (nouveau — record_inducements_use_case.rs)

```rust
pub struct MercenaryPurchaseCmd {
    pub position_id: PositionId,    // SUlid — validé ULID au parsing
    pub level:       MercenaryLevel,
}
```

**Émis par** : handler POST (parse du champ `mercenaries` JSON)
**Consommé par** : `RecordInducementsCommand`, puis `record_inducements_use_case::execute`

---

### PositionId (type existant — shared_kernel/common_types.rs)

```rust
pub type PositionId = EntityId;  // = SUlid — validation ULID (longueur + Crockford base32)
```

Déjà défini. Le handler parse la chaîne reçue via `PositionId::try_new(s)` qui valide le format ULID.

**Émis par** : handler POST (parsing via `PositionId::try_new`)
**Consommé par** : `MercenaryPurchaseCmd`, use case, `collect_mercs`

---

### MercenaryLevel (nouveau enum — use_cases/record_inducements_use_case.rs)

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MercenaryLevel { Base, Lvl1 }

impl MercenaryLevel {
    pub fn extra_cost(&self) -> u32 {
        match self { Self::Base => 30, Self::Lvl1 => 80 }
    }
    pub fn as_str(&self) -> &'static str {
        match self { Self::Base => "base", Self::Lvl1 => "lvl1" }
    }
    pub fn try_from_str(s: &str) -> Result<Self, &'static str> {
        match s { "base" => Ok(Self::Base), "lvl1" => Ok(Self::Lvl1), _ => Err("niveau inconnu") }
    }
}
```

**Émis par** : handler POST (parse JSON `"level"`)
**Consommé par** : `MercenaryPurchaseCmd`, use case (calcul coût + encodage UID)

---

### RecordInducementsCommand (modifié)

```rust
pub struct RecordInducementsCommand {
    pub match_report_id:      MatchReportId,
    pub team_id:              TeamId,
    pub purchases:            Vec<InducementPurchaseCmd>,     // existant
    pub mercenary_purchases:  Vec<MercenaryPurchaseCmd>,      // NOUVEAU
    pub recorded_by:          CoachId,
}
```

**Émis par** : handler POST
**Consommé par** : `record_inducements_use_case::execute`

---

## DTOs de port (query side — match_report/ports.rs)

Ces DTOs sont des lectures pures. Les primitives sont acceptées.

### RosterPositionDto (nouveau)

```rust
pub struct RosterPositionDto {
    pub position_uid:  String,
    pub position_name: String,
    pub base_cost:     u32,
    pub max_qty:       u8,
    pub is_journalier: bool,
}
```

**Émis par** : `RefTeamDataAdapter::find_roster_positions`
**Consommé par** : handler GET `mercenary_selector_widget` (via `build_position_grid`)

---

### PositionCountDto (nouveau)

```rust
pub struct PositionCountDto {
    pub position_uid: String,
    pub count:        u8,
}
```

**Émis par** : `PlayerDataAdapter::find_player_counts_by_position`
**Consommé par** : handler GET `mercenary_selector_widget` (via `build_position_grid`)

---

## DTOs de sortie — View Models (query side)

Les view models sont des types de lecture. Les primitives sont acceptées.

### PositionCardVm (nouveau — mercenary_selector_widget.rs)

```rust
pub struct PositionCardVm {
    pub uid:           String,
    pub name:          String,
    pub base_cost:     u32,
    pub price_base:    u32,   // base_cost + 30
    pub price_lvl1:    u32,   // base_cost + 80
    pub count_in_team: u8,
    pub max_qty:       u8,
    pub disabled:      bool,  // count_in_team >= max_qty
}
```

**Construit par** : fonction `build_position_grid(positions, counts)` dans le handler GET
**Consommé par** : template `mercenary-selector-widget.html`

---

### MercenarySelectorTemplate (nouveau — mercenary_selector_widget.rs)

```rust
#[derive(Template)]
#[template(path = "widgets/mercenary-selector-widget.html")]
pub struct MercenarySelectorTemplate {
    pub positions: Vec<PositionCardVm>,
}
```

**Émis par** : handler GET `mercenary_selector_widget::get_mercenary_selector`
**Consommé par** : template Askama

---

### InducementsTemplate (modifié — inducements_controller.rs)

```rust
pub struct InducementsTemplate {
    // ... champs existants ...
    pub mercenary_selector_url: String,   // NOUVEAU
}
```

**Émis par** : handler GET `inducements_controller::get_inducements`
**Consommé par** : template `inducements.html`

---

## Tableau récapitulatif — interfaces d'utilisation

| DTO / Type | Émis par | Consommé par |
|---|---|---|
| `InducementsForm` (étendu) | Formulaire HTML | Handler POST `inducements_controller` |
| `MercenaryPurchaseCmd` | Handler POST (parse JSON) | `RecordInducementsCommand` |
| `PositionId` | Handler POST (parse via `PositionId::try_new`) | `MercenaryPurchaseCmd`, use case |
| `MercenaryLevel` | Handler POST | `MercenaryPurchaseCmd`, use case, `collect_mercs` |
| `RecordInducementsCommand` (étendu) | Handler POST | `record_inducements_use_case::execute` |
| `RosterPositionDto` | `RefTeamDataAdapter::find_roster_positions` | Handler GET widget (`build_position_grid`) |
| `PositionCountDto` | `PlayerDataAdapter::find_player_counts_by_position` | Handler GET widget (`build_position_grid`) |
| `PositionCardVm` | `build_position_grid` (handler GET widget) | Template `mercenary-selector-widget.html` |
| `MercenarySelectorTemplate` | Handler GET widget | Template `mercenary-selector-widget.html` |
| `InducementsTemplate` (étendu) | Handler GET `inducements_controller` | Template `inducements.html` |

---

## Règles métier identifiées à cette étape

- `PositionId` valide : ULID valide — `PositionId::try_new(s)` rejette toute chaîne malformée (longueur, charset Crockford base32) → erreur 400 au handler
- `MercenaryLevel` valide : `"base"` ou `"lvl1"` uniquement — tout autre valeur = erreur 400
- `price_base` et `price_lvl1` calculés une seule fois (dans `PositionCardVm`) — réutilisés dans le template sans recalcul
- Le VM précalcule `disabled` côté serveur pour rendre le template déclaratif
