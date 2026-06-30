# BC match_report — Widget mercenary-selector (GET)

**Priorité : haute**
**Dépend de :** 125
**Contexte :** `docs/specs/match-report/step2-mercenaires/02-front.md`, `03-back.md`, `04-dtos.md`

## Objectif

Créer le widget de sélection de mercenaires : endpoint GET, template Askama, route, CSS.

## Conception

### 1. Route — src/app/match_report/routes.rs

Dans `pub mod path` :

```rust
pub const MATCH_REPORT_MERCENARY_SELECTOR: &str =
    "/app/{space_id}/match-report/{match_report_id}/step2/{team_id}/mercenaires";
```

Dans `impl Routes` :

```rust
pub fn mercenary_selector(
    &self,
    space_id:        &str,
    match_report_id: &str,
    team_id:         &str,
) -> String {
    path::MATCH_REPORT_MERCENARY_SELECTOR
        .replace("{space_id}", space_id)
        .replace("{match_report_id}", match_report_id)
        .replace("{team_id}", team_id)
}
```

### 2. Router — src/app/match_report/router.rs

```rust
use crate::app::match_report::io::web::widgets::mercenary_selector_widget::get_mercenary_selector;
// ...
.route(path::MATCH_REPORT_MERCENARY_SELECTOR, get(get_mercenary_selector))
```

### 3. Module — src/app/match_report/io/web/widgets/mod.rs

```rust
pub mod mercenary_selector_widget;
```

### 4. Handler — src/app/match_report/io/web/widgets/mercenary_selector_widget.rs

```rust
pub struct PositionCardVm {
    pub uid:           String,
    pub name:          String,
    pub base_cost:     u32,
    pub price_base:    u32,
    pub price_lvl1:    u32,
    pub count_in_team: u8,
    pub max_qty:       u8,
    pub disabled:      bool,
}

#[derive(Template)]
#[template(path = "match_report/widgets/mercenary-selector-widget.html")]
pub struct MercenarySelectorTemplate {
    pub positions: Vec<PositionCardVm>,
}
```

Handler `get_mercenary_selector` :

1. Extraire `space_id`, `match_report_id`, `team_id` depuis le path
2. Appeler `ITeamDataPort::find_roster_positions(team_id)` → positions
3. Appeler `IPlayerDataPort::find_player_counts_by_position(team_id)` → counts
4. `build_position_grid(positions, counts)` → `Vec<PositionCardVm>`
   - Filtrer `is_journalier == true`
   - Pour chaque position : `count_in_team` = lookup dans counts (0 si absent), `disabled` = count >= max_qty
   - `price_base = base_cost + 30`, `price_lvl1 = base_cost + 80`
5. Retourner `MercenarySelectorTemplate { positions }`

### 5. Template — templates/match_report/widgets/mercenary-selector-widget.html

Cf. maquette `assets/rawpages/html/app-match-report-step2-inducements-mercenaires.html`.

Structure :
- `<link rel="stylesheet" href="/static/css/widgets/mercenary-selector.css">`
- Racine avec `hx-disinherit="*"`
- Alpine `x-data="mercenarySelector()"` — compteur, selectedPosition, liste des mercos
- Compteur dots 0/3
- Grille de `PositionCardVm` : nom, prix, count/max, `disabled` si complet
- Hire panel (`x-show="selectedPosition"`) : deux boutons Recruter Base / Niv.1
- Script Alpine inline scoped via `document.currentScript.previousElementSibling`

Événements émis :
- `mercenarySelectionChanged` sur `document.body` à chaque ajout/suppression
- `removeMercenaire { idx }` écouté depuis `body` → appelle `removeMerc(idx)` en Alpine

### 6. CSS — assets/static/css/widgets/mercenary-selector.css

Styles pour : grille positions, carte disabled, hire panel, dots compteur.

## Checklist

- [ ] Constante route + méthode `mercenary_selector` dans `routes.rs`
- [ ] Route GET enregistrée dans `router.rs`
- [ ] `pub mod mercenary_selector_widget` dans `widgets/mod.rs`
- [ ] `PositionCardVm` et `MercenarySelectorTemplate` définis
- [ ] `build_position_grid` implémenté (filtre journaliers, compute disabled, prix)
- [ ] Handler `get_mercenary_selector` implémenté (< 20 lignes)
- [ ] Template `mercenary-selector-widget.html` créé (Alpine, grille, hire panel)
- [ ] `assets/static/css/widgets/mercenary-selector.css` créé
- [ ] `cargo build` passe
- [ ] Vérification manuelle : `GET /app/.../step2/{team_id}/mercenaires` retourne la grille
