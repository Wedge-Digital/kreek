# Step 2.1 — Coups de pouce — Contrats de données

Chaque DTO est décrit avec son **interface d'utilisation** : qui le produit, qui le consomme.

---

## 1. DTOs d'entrée — GET page

### Paramètre de path

```
{team_id} : String  — identifiant de l'équipe qui achète
```

| Produit par | Consommé par |
|---|---|
| URL construite par le handler précédent (redirect post fan-factor ou post-inducements) | `get_inducements` (handler) → smart constructor `TeamId::try_new()` |

---

## 2. DTOs d'entrée — POST soumission achats

### Form HTTP

```rust
#[derive(Deserialize)]
pub struct RecordInducementsForm {
    pub selection: String,  // JSON sérialisé par Alpine : "[{uid, qty, unit_cost}]"
}
```

| Produit par | Consommé par |
|---|---|
| Alpine (cart widget) via un `<input type="hidden" name="selection">` mis à jour à chaque changement | `post_inducements` (handler) → désérialise en `Vec<InducementPurchaseInput>` |

### Item désérialisé depuis `selection`

```rust
#[derive(Deserialize)]
pub struct InducementPurchaseInput {
    pub uid: String,
    pub qty: u8,
}
```

| Produit par | Consommé par |
|---|---|
| Désérialisé par le handler depuis le champ `selection` | Handler → construit `RecordInducementsCommand` via `InducementId::try_new()` |

---

## 3. Commandes use case

### `RecordFanFactorCommand` (modifiée)

La commande reste inchangée. Les TeamValues sont fetchées **à l'intérieur du use case** via `ITeamDataPort.find_team_value()` — elles ne font pas partie de la commande entrante.

```rust
pub struct RecordFanFactorCommand {
    pub match_report_id: MatchReportId,
    pub home_fan_roll:   D3Roll,
    pub away_fan_roll:   D3Roll,
    pub recorded_by:     CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `post_pre_match` (handler step 2) | `record_fan_factor_use_case::execute()` |

### `RecordInducementsCommand` (nouveau)

```rust
pub struct RecordInducementsCommand {
    pub match_report_id: MatchReportId,
    pub team_id:         TeamId,
    pub purchases:       Vec<InducementPurchaseCmd>,  // vide si "Passer"
    pub recorded_by:     CoachId,
}

pub struct InducementPurchaseCmd {
    pub uid: InducementId,
    pub qty: u8,
}
```

| Produit par | Consommé par |
|---|---|
| `post_inducements` (handler) après validation des value objects | `record_inducements_use_case::execute()` |

---

## 4. Value objects nouveaux

### `TeamValue`

```rust
#[nutype(
    validate(greater_or_equal = 0),
    derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Display)
)]
pub struct TeamValue(u32);
```

| Produit par | Consommé par |
|---|---|
| `record_fan_factor_use_case` (via port) | `MatchReportPreMatch` (stocké dans `home_team_value` / `away_team_value`) · méthodes `topdog_team_id()`, `inducement_budget_for()` |

### `InducementPurchase` (domain)

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InducementPurchase {
    pub uid:       InducementId,
    pub qty:       u8,
    pub unit_cost: u32,  // conservé pour topdog_spending() sans recalcul
}
```

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::record_inducements()` (valide et construit la liste) | Stocké dans `home_inducements` / `away_inducements` · event `InducementsRecorded` · `topdog_spending()` |

---

## 5. Domain events nouveaux

### `TeamValuesRecorded`

```rust
TeamValuesRecorded {
    home_team_value: TeamValue,
    away_team_value: TeamValue,
    recorded_by:     CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::record_team_values()` | `record_fan_factor_use_case` (persiste via `repo.append()`) · `rehydrate()` (met à jour les champs TV de l'agrégat) · projection repository (colonne `home_team_value`, `away_team_value`) |

### `InducementsRecorded`

```rust
InducementsRecorded {
    team_id:     TeamId,
    purchases:   Vec<InducementPurchase>,  // vide si "Passer"
    recorded_by: CoachId,
}
```

| Produit par | Consommé par |
|---|---|
| `MatchReportPreMatch::record_inducements()` | `record_inducements_use_case` (persiste via `repo.append()`) · `rehydrate()` (met à jour `home_inducements` / `away_inducements`) · projection repository |

---

## 6. DTOs de port (BC MatchReport)

### `ITeamDataPort` — nouvelles méthodes

```rust
// Méthode existante — extension de TeamInfoDto
pub struct TeamInfoDto {
    pub team_name:    String,
    pub coach_name:   String,
    pub roster_name:  String,
    pub roster_id:    String,   // NOUVEAU — nécessaire pour paramétrer le widget
}

// Nouvelle méthode
async fn find_team_value(&self, team_id: &str) -> Option<u32>;
async fn find_team_treasury(&self, team_id: &str) -> Option<u32>;
```

| DTO | Produit par | Consommé par |
|---|---|---|
| `TeamInfoDto.roster_id` | Adapter Teams BC | `get_inducements` handler (passe `roster_id` au widget) |
| `find_team_value` | Adapter Teams BC | `record_fan_factor_use_case` (capture les deux TV au POST step 2) |
| `find_team_treasury` | Adapter Teams BC | `record_inducements_use_case` (calcul du budget disponible) |

### `ICompetitionDataPort` — nouvelle méthode

```rust
pub struct TierRulesDto {
    pub allowed_inducements:  Vec<InducementSpecDto>,
    pub allowed_star_players: Vec<InducementSpecDto>,
}

pub struct InducementSpecDto {
    pub uid:       String,
    pub max_qty:   u8,
    pub unit_cost: u32,
}

async fn find_tier_rules_for_roster(
    &self,
    season_id: &str,
    roster_id: &str,
) -> Option<TierRulesDto>;
```

L'adapter infrastructure assemble ces données depuis Competitions BC (UIDs autorisés) + References BC (détails per UID : `max_qty`, `unit_cost`). Les UIDs seuls sont extraits pour construire l'URL du widget.

| Produit par | Consommé par |
|---|---|
| Adapter Competitions+References BC | `get_inducements` handler (extrait les UIDs pour l'URL du widget) · `record_inducements_use_case` (passe `allowed_specs` au domaine pour validation `maxQty` + coût) |

---

## 7. VM de sortie — template `inducements.html` (BC MatchReport)

```rust
#[derive(Template)]
#[template(path = "inducements.html")]
pub struct InducementsTemplate {
    pub app_routes:            AppRoutes,
    pub space_id:              String,
    pub match_report_id:       String,
    pub team_id:               String,
    pub team_name:             String,
    pub team_initials:         String,
    pub order_label:           String,   // "achète en premier" | "achète en second"
    pub budget:                u32,      // kPo disponibles pour cette équipe
    pub budget_label:          String,   // "Trésorerie" | "Différence TV + dépenses adverses + trésorerie"
    pub inducement_selector_url: String, // URL complète avec tous les params (allowed_uids, roster_id, instance_id)
    pub form_action:           String,   // POST /step2/inducements/{team_id}
    pub pass_url:              String,   // GET url étape suivante
}
```

| Produit par | Consommé par |
|---|---|
| `get_inducements` handler (construit depuis `MatchReportPreMatch` + ports) | Template Askama `inducements.html` · Alpine `x-data` pour le cart (reçoit `budget` via attribut `data-budget`) |

---

## 8. DTOs du widget inducement-selector (BC References)

### Params en entrée (query string)

```rust
#[derive(Deserialize)]
pub struct InducementSelectorParams {
    pub allowed_inducement_uids:  String,          // CSV
    pub allowed_star_player_uids: String,          // CSV
    pub roster_id:                String,
    pub instance_id:              String,
    #[serde(default)]
    pub selected:                 Option<String>,  // "uid:qty,uid:qty"
}
```

| Produit par | Consommé par |
|---|---|
| `get_inducements` handler (construit l'URL `inducement_selector_url`) | `inducement_selector_controller` (References BC) |

### Items rendus — inducements communs / spéciaux

```rust
pub struct InducementSelectorItem {
    pub uid:         String,
    pub name:        String,
    pub description: String,
    pub unit_cost:   u32,
    pub max_qty:     u8,
    pub category:    InducementDisplayCategory,  // Common | Special
    pub initial_qty: u8,                         // 0 ou valeur pré-sélectionnée
}

pub enum InducementDisplayCategory { Common, Special }
```

| Produit par | Consommé par |
|---|---|
| `inducement_selector_controller` (mappe le modèle `Inducement` du References BC) | Template `inducement-selector.html` (onglets Communs / Spéciaux) · Alpine `x-data` (quantities, notify) |

### Items rendus — star players

```rust
pub struct StarPlayerSelectorItem {
    pub uid:                        String,
    pub name:                       String,
    pub rosters_label:              String,  // "Elfes Noirs, Elfes Sylvains…"
    pub cost:                       u32,
    pub ma:                         u8,
    pub st:                         u8,
    pub ag:                         String,  // ex. "2+"
    pub pa:                         String,  // ex. "4+" ou "—"
    pub av:                         String,  // ex. "8+"
    pub skills:                     String,  // liste séparée par virgules
    pub special_ability_name:       String,
    pub special_ability_description: String,
    pub initial_qty:                u8,
}
```

| Produit par | Consommé par |
|---|---|
| `inducement_selector_controller` (mappe le modèle `StarPlayer` du References BC) | Template `inducement-selector.html` (onglet Star Players, cartes dépliables) · Alpine `x-data` (openStars, quantities, notify) |

### Événement DOM émis

```js
htmx.trigger(document.body, 'inducementSelectionChanged', {
  instanceId: String,
  items: [{ uid: String, name: String, qty: Number, unit_cost: Number }],
  total_cost: Number
})
```

| Produit par | Consommé par |
|---|---|
| Alpine dans `inducement-selector.html` (à chaque changement de qty) | Cart widget Alpine (`@inducement-selection-changed.window`) dans `inducements.html` |
