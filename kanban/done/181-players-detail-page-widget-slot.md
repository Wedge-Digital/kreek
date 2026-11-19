# BC `players` — Slot unique sur la fiche joueur (journal / dépense SPP)

**Priorité : haute**
**Dépend de :** `175-players-domain-improvement.md`
**Contexte :** `players/io/web` — page + widgets

## Objectif

Transformer le bloc actuel "Journal des évolutions" (aujourd'hui inline
dans `player-detail.html`) en un slot unique rempli par l'un de plusieurs
widgets mutuellement exclusifs — journal (par défaut) ou dépense SPP (carte
182) — pour que le futur mode "customisation" s'y branche de la même façon
sans mélanger les responsabilités. Spec complète :
`docs/specs/player-spp-spending/README.md`.

---

## Conception

### Host `player-detail.html`

Remplace les lignes 143-183 (bloc `.pd-right` actuel) par :
```html
<div id="pd-right-panel"
     hx-get="{{ vm.right_panel_widget_url }}"
     hx-trigger="load" hx-target="this" hx-swap="outerHTML">
  <div class="loading-placeholder">Chargement…</div>
</div>
```

### Handler (`player_detail_controller.rs`)

Nouvelle fonction `can_spend_spp(state, user, space_id_vo, team) -> bool` :
`is_coach(team, &user.id) || check_admin_rights(...)` (réutilise la
fonction existante). `is_coach` : `team.coach_id.to_string() == user.id.to_string()`.

`right_panel_widget_url` calculée dans `build_vm`/le handler :
```rust
let widget_url = if team.game_phase == Some(GamePhase::PlayerImprovement) && can_spend_spp {
    app_routes.players.spp_spending_widget(&space_id, &player_id)
} else {
    app_routes.players.evolution_journal_widget(&space_id, &player_id)
};
```

`compute_spp_breakdown` est remplacée par un appel à `player.spp_remaining()`
(carte 175) — supprime la duplication de calcul.

### Widget `evolution_journal_widget` (nouveau, copié à l'identique)

Copier-coller exact des lignes 145-182 actuelles de `player-detail.html`
(spend-panel-header + lock-banner + table spp-summary) dans un nouveau
template `widgets/evolution-journal.html` + un nouveau handler GET
`evolution_journal_widget.rs` — aucune réécriture, seule la source des
données (VM) est adaptée à un handler de widget autonome.

`evolution_log_vm()` (actuellement dans `player_detail_controller.rs`) est
étendue pour inclure aussi les `stat_increases` (aujourd'hui seules les
`acquired_skills` y figurent).

### Routes

```
GET /app/{space_id}/players/{player_id}/evolution-journal-widget
GET /app/{space_id}/players/{player_id}/spp-spending-widget   (câblé carte 182, route ajoutée ici)
```

---

## Checklist

- [ ] `player-detail.html` : bloc `.pd-right` remplacé par le slot `hx-get`
- [ ] `can_spend_spp()` + `is_coach()` dans `player_detail_controller.rs`
- [ ] `right_panel_widget_url` calculée et injectée dans le VM
- [ ] `compute_spp_breakdown` remplacé par `player.spp_remaining()`
- [ ] `evolution_journal_widget.rs` + `widgets/evolution-journal.html` (copiés, pas réécrits)
- [ ] `evolution_log_vm()` étendue aux augmentations de caractéristiques
- [ ] 2 routes GET ajoutées (widget dépense SPP câblé en carte 182)
- [ ] Test : équipe hors phase `PlayerImprovement` → widget journal ; équipe en phase + coach/admin → widget dépense ; équipe en phase mais utilisateur non autorisé → widget journal
