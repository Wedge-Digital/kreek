# BC `team_creation` — Widget skill-header (carte D)

**Priorité : haute**
**Dépend de :** `56-tc-player-identity.md`
**Contexte :** BC `team_creation` — widget HTMX

## Objectif

Créer le widget "header joueur" (carte D de la page de finalisation). Affiché dans le panneau droit quand un joueur est sélectionné. Contient : jersey et nom éditables (auto-save), position, roster, compétences acquises en tags, compteur SPP restants.

---

## Conception

### Endpoint

```
GET /app/{space_id}/team/{team_id}/widgets/skill-header?player_id={id}
```

### Handler

Charge l'équipe, trouve le joueur par `player_id`, retourne le fragment HTML.

### Template `widgets/skill-header-widget.html`

- Jersey input (`hx-post` → set_player_identity, `hx-trigger="change"`)
- Nom input (`hx-post` → set_player_identity, `hx-trigger="blur"`)
- Position + roster (lecture seule)
- Tags des compétences acquises (avec bouton ✕ → cancel_spp)
- Compteur SPP restants (pool équipe)

### Intégration dans la page hôte

```html
<div id="skill-header"
     hx-get="{{ team_routes.skill_header_widget(space_id, team_id) }}"
     hx-trigger="playerSelected from:body, skillsUpdated from:body"
     hx-vals='js:{"player_id": event.detail.player_id}'
     hx-target="this"
     hx-swap="innerHTML">
  <div class="skill-panel-empty">Sélectionnez un joueur</div>
</div>
```

### Communication

| Événement reçu | Action |
|---|---|
| `playerSelected from:body` | Recharge avec le nouveau joueur |
| `skillsUpdated from:body` | Recharge (compétences mises à jour) |

---

## Checklist

- [ ] Ajouter `SKILL_HEADER_WIDGET` dans `routes.rs` (path + méthode)
- [ ] Créer le handler dans `widgets/skill_header_widget.rs`
- [ ] Créer le template `widgets/skill-header-widget.html`
- [ ] Enregistrer la route dans le router
- [ ] Inputs auto-save câblés vers `set_player_identity`
- [ ] Tags compétences avec bouton cancel câblés vers `cancel_spp`
- [ ] Compteur SPP restants
