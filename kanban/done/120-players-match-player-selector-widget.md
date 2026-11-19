# BC players — Widget match-player-selector

**Priorité : haute**
**Dépend de :** —
**Contexte :** players — widget pour la sélection de joueur dans les étapes 3 et 4

## Objectif

Créer le widget `match-player-selector` exposé par le BC players et chargé par la page hôte step3/step4 de BC match_report.

## Conception

Cf. `docs/specs/match-report/step3-4-actions/02-front.md`, `03-back.md`

### Route

```
GET /app/{space_id}/players/teams/{team_id}/match-selector
```

Handler : `src/app/players/io/web/widgets/match_player_selector_widget.rs`
Template : `src/app/players/io/web/widgets/match_player_selector_widget.html`

### Comportement

- Charge la liste des joueurs de `team_id` via `IPlayerProjectionRepository::find_by_team_id`
- Rend un sélecteur de joueur unique, sans TomSelect — liste scrollable avec un item par joueur
- Chaque item : `{personal_name} #{jersey}` — clic émet `playerSelected { player_id, player_type: "regular" }` sur `document.body`
- Racine isolée avec `hx-disinherit="*"`
- `<link rel="stylesheet" href="/static/css/widgets/match-player-selector.css">`
- En V1, tous les joueurs sont affichés (disponibilité non gérée)

### View model

```rust
pub struct MatchPlayerSelectorVm {
    pub players: Vec<PlayerRowVm>,
}

pub struct PlayerRowVm {
    pub player_id:     String,
    pub display_name:  String,   // "{personal_name} #{jersey}"
}
```

### Wiring

- Constante `MATCH_PLAYER_SELECTOR` dans `src/app/players/routes.rs`
- Route câblée dans `src/app/players/router.rs`
- `pub mod widgets;` dans `src/app/players/io/web/mod.rs`
- `pub mod match_player_selector_widget;` dans `src/app/players/io/web/widgets/mod.rs`

## Checklist

- [ ] Handler GET + VM `MatchPlayerSelectorVm` + `PlayerRowVm`
- [ ] Template HTML avec `hx-disinherit="*"` + CSS link + événement DOM `playerSelected`
- [ ] Constante de route + méthode builder dans `Routes`
- [ ] Câblage router
- [ ] Fichier CSS `match-player-selector.css` (même si minimal)
