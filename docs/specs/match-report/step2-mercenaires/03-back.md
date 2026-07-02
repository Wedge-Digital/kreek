# Step 2 — Mercenaires — Architecture back

## Vue d'ensemble

```
BC match_report
├── widgets/mercenary_selector_widget.rs   ← NOUVEAU : handler GET du widget
├── templates/widgets/
│   └── mercenary-selector-widget.html     ← NOUVEAU : template Askama
├── inducements_controller.rs              ← MODIFIÉ : parse mercenaries au POST + url dans template GET
├── templates/inducements.html             ← MODIFIÉ : 4 tabs, zone merco, cart étendu
├── routes.rs                              ← MODIFIÉ : route mercenary_selector
├── router.rs                              ← MODIFIÉ : enregistrement GET
├── ports.rs                               ← MODIFIÉ : 2 DTOs + 2 méthodes de trait
├── use_cases/record_inducements_use_case.rs ← MODIFIÉ : parse + valide mercenaires
└── use_cases/init_temp_players_use_case.rs  ← MODIFIÉ : collect_mercs avec position_uid réel

BC références
└── templates/widgets/inducement-selector.html  ← MODIFIÉ : retire tab bar, ajoute event listener

Infrastructure
├── match_report/ref_team_data_adapter.rs  ← MODIFIÉ : impl find_roster_positions
└── match_report/player_data_adapter.rs   ← MODIFIÉ : impl find_player_counts_by_position
```

---

## Nouveau widget — mercenary_selector_widget.rs

**Route** : `GET /app/{space_id}/match-report/{mr_id}/step2/{team_id}/mercenaires`

**Handler** (`mercenary_selector_widget.rs`) :

1. Charge le `MatchReportPreMatch` depuis le repo pour connaître le nombre de mercenaires déjà enregistrés
2. Appelle `ITeamDataPort::find_roster_positions(team_id)` → liste des positions du roster
3. Appelle `IPlayerDataPort::find_player_counts_by_position(team_id)` → counts par position
4. Croise les deux pour construire les `PositionCardVm` (position disponible/disabled)
5. Filtre les journaliers (`is_journalier: true`) — exclus de la grille
6. Retourne le template `mercenary-selector-widget.html`

Pas de domain service : transformation lecture pure, logique de croisement locale au handler (`build_position_grid`).

**Template** (`mercenary-selector-widget.html`) :

- `hx-disinherit="*"` sur la racine
- Alpine `x-data="mercenarySelector()"` (script inline dans le template)
- Compteur dots 0/3
- Grille `PositionCardVm` : nom, prix, count_in_team / max_qty, `disabled` si complet
- Hire panel (`x-show="selectedPosition"`) : deux options Recruter / Niv.1
- CSS embarqué via `<link rel="stylesheet" href="/static/css/widgets/mercenary-selector.css">`

---

## Ports — modifications de match_report/ports.rs

### Nouveaux DTOs

```rust
pub struct RosterPositionDto {
    pub position_uid:  String,
    pub position_name: String,
    pub base_cost:     u32,
    pub max_qty:       u8,
    pub is_journalier: bool,
}

pub struct PositionCountDto {
    pub position_uid: String,
    pub count:        u8,
}
```

### Nouvelles méthodes sur ITeamDataPort

```rust
async fn find_roster_positions(&self, team_id: &str) -> Vec<RosterPositionDto>;
```

### Nouvelles méthodes sur IPlayerDataPort

```rust
async fn find_player_counts_by_position(&self, team_id: &str) -> Vec<PositionCountDto>;
```

---

## Infrastructure — implémentations

### ref_team_data_adapter.rs — find_roster_positions

```
team_repo.find_by_id(team_id) → roster_id
reference_repo.find_team_by_uid(roster_id) → Team.available_players
→ Vec<RosterPositionDto>
```

Les deux repos sont déjà injectés dans `RefTeamDataAdapter`.

### player_data_adapter.rs — find_player_counts_by_position

```
player_projection_repo.find_by_team_id(team_id) → Vec<PlayerProjection>
→ group by roster_line_id (= position_uid)
→ Vec<PositionCountDto>
```

---

## Route et router

### routes.rs (match_report)

```rust
pub fn mercenary_selector(&self, space_id: &str, mr_id: &str, team_id: &str) -> String {
    format!("/app/{space_id}/match-report/{mr_id}/step2/{team_id}/mercenaires")
}
```

### router.rs (match_report)

```rust
.route(
    "/app/:space_id/match-report/:mr_id/step2/:team_id/mercenaires",
    get(mercenary_selector_widget::get_mercenary_selector),
)
```

---

## inducements_controller.rs — modifications

### GET : ajout de mercenary_selector_url dans InducementsTemplate

```rust
pub struct InducementsTemplate {
    // ... champs existants ...
    pub mercenary_selector_url: String,  // NOUVEAU
}
```

Construit dans `build_vm` :
```rust
mercenary_selector_url: routes.match_report.mercenary_selector(space_id, &mr_id.to_string(), &team_id.to_string()),
```

### POST : parsing du champ mercenaires

```rust
#[derive(Deserialize)]
pub struct InducementsForm {
    #[serde(default)]
    pub selection: String,
    #[serde(default)]
    pub mercenaries: String,   // NOUVEAU — JSON "[{position_uid, tier}]"
}
```

Le handler parse `mercenaries` et construit un `Vec<MercenaryPurchaseCmd>` passé au use case.

---

## record_inducements_use_case.rs — modifications

### Nouvelle commande partielle

```rust
pub struct MercenaryPurchaseCmd {
    pub position_id: PositionId,    // SUlid — validé ULID au parsing
    pub level:       MercenaryLevel,
}

pub enum MercenaryLevel { Base, Lvl1 }
```

### RecordInducementsCommand étendue

```rust
pub struct RecordInducementsCommand {
    // ... champs existants ...
    pub mercenary_purchases: Vec<MercenaryPurchaseCmd>,  // NOUVEAU
}
```

### Validation côté use case (avant appel domaine)

- Vérifier que chaque `position_uid` n'est pas un journalier : appel à `ITeamDataPort::find_roster_positions(team_id)`, filtre `is_journalier`
- Cette validation protège contre une soumission malformée — le front ne l'affiche pas mais le back valide

### Calcul du coût par mercenaire

Le use case résout le coût à partir de `find_roster_positions` :
```
price = position.base_cost + if tier == Lvl1 { 80 } else { 30 }
```

### Transformation en InducementPurchase

Les mercenaires sont convertis en `InducementPurchase` avec un UID structuré avant d'être passés à l'agrégat. Format UID : `"MERCO:{position_uid}:{tier}"` — permet à `collect_mercs` de retrouver la position.

Exemple : `"MERCO:blitzeur-elf-noire:base"`, `"MERCO:witch-elf:lvl1"`

L'agrégat voit ces achats comme des inducements normaux (uid + qty:1 + unit_cost).

---

## init_temp_players_use_case.rs — collect_mercs

```rust
fn collect_mercs(pm, team_id) -> Vec<TempPlayer> {
    pm.purchases_for(team_id)
        .iter()
        .filter(|p| p.uid.0.starts_with("MERCO:"))
        .map(|p| {
            // "MERCO:{position_uid}:{tier}"
            let position_uid = p.uid.0.splitn(3, ':').nth(1).unwrap_or("").to_string();
            TempPlayer {
                id:           TempPlayerId(ulid::Ulid::new().to_string()),
                team_id:      team_id.clone(),
                kind:         TempPlayerKind::Mercenary { position_uid },
                display_name: None,
            }
        })
        .collect()
}
```

---

## inducement-selector.html — modification minimale (BC références)

- **Retirer** `<div class="mr-tabs">…</div>` du template
- **Ajouter** dans `window.inducementSelector` :
  ```js
  // dans le return { ... }
  init() {
    document.body.addEventListener('switchInducementTab', (e) => {
      this.activeTab = e.detail.tab;
    });
  }
  ```
  Ou via Alpine : `@switch-inducement-tab.window="activeTab = $event.detail.tab"`

---

## Règles métier identifiées à cette étape

- La validation côté back que `position_uid` n'est pas un journalier se fait dans le use case (avant appel domaine), via `find_roster_positions`
- Le format UID `"MERCO:{position_uid}:{tier}"` est la convention interne d'encodage — jamais exposé à l'utilisateur
- `collect_mercs` filtre sur `uid.starts_with("MERCO:")` pour rester compatible avec les anciennes données éventuelles (`"MERCENARY_PLAYER"` ne matchera plus — à vérifier s'il existe des données existantes en base)
