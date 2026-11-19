# BC `players` — Routing de la fiche joueur

**Priorité : haute**
**Dépend de :** `166-players-player-detail-page.md`
**Contexte :** `players/routes.rs`, `router.rs`, `player-table-fragment.html`

## Objectif

Exposer la route de la fiche joueur et rediriger le clic sur une ligne du
tableau roster vers cette page, à la place de la page de debug (carte
antérieure, hors spec `player-match-impact`).

---

## Conception

```rust
// routes.rs
pub const PLAYER_DETAIL: &str = "/app/{space_id}/players/{player_id}/detail";

pub fn player_detail(&self, space_id: &str, player_id: &str) -> String {
    path::PLAYER_DETAIL.replace("{space_id}", space_id).replace("{player_id}", player_id)
}
```

```rust
// router.rs
.route(path::PLAYER_DETAIL, get(player_detail_controller))
```

Template `player-table-fragment.html` : remplacer `app_routes.players.player_debug(...)` par `app_routes.players.player_detail(...)` sur le `onclick` de la ligne (`<tr class="player-table-row" onclick="...">`).

La route `player_debug` (page de debug) **n'est pas supprimée** — elle reste disponible comme outil de développement, simplement plus utilisée comme cible par défaut du clic.

---

## Checklist

- [ ] Route `PLAYER_DETAIL` + helper `Routes::player_detail()`
- [ ] Enregistrement dans `router()`
- [ ] `player-table-fragment.html` : `onclick` mis à jour vers `player_detail`
