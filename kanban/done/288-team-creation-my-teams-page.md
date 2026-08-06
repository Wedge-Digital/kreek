# BC `team_creation` — Page "Mes équipes" restructurée (section brouillons)

**Priorité : haute**
**Dépend de :** `287-teams-my-teams-widget.md`
**Contexte :** `team_creation` (page hôte)

## Objectif

Réécrire le handler et le template existants pour ne plus afficher que les
brouillons (équipes non soumises), et déléguer la section active/archivée
au widget BC `teams` (carte 287). Corrige au passage un bug caché : le
handler actuel ne remplit jamais le roster affiché (`roster: String::new()`
codé en dur).

**Maquette de référence :** `assets/rawpages/html/app-my-teams.html`
**Spec de référence :** `docs/specs/my-teams/mes-equipes/` (toutes phases).

---

## Conception

### Handler réécrit

```rust
// team_creation/io/web/my_teams.rs
pub struct DraftTeamCardVm {
    pub id: String, pub initials: String, pub name: String,
    pub logo: Option<String>, pub roster: Option<String>, pub link: String,
}
```

1. `drafts = team_repository.find_by_coach_and_space(coach_id, space_id)` (table `team_drafts`)
2. `submitted_ids = roster_repository.find_submitted_ids_for_space(space_id)`
3. Filtrer `drafts` pour **exclure** les `submitted_ids` (les équipes soumises sortent entièrement de ce BC — elles n'apparaissent plus que côté `teams`)
4. Pour chaque brouillon restant : `roster_repository.find_by_id(team_id)` → `Option<RosterSelectedTeam>` → nom du roster si déjà choisi, `None` sinon (encore au stade ruleset)

### Template restructuré

```html
<!-- Section brouillons — rendue inline, markup draft-card (pas le macro team-card partagé) -->
{% for draft in drafts %}
<div class="draft-card"> ... </div>
{% endfor %}

<!-- Section active/archivée — widget BC teams -->
<div hx-get="{{ app_routes.teams.my_teams_widget(space_id) }}"
     hx-trigger="load" hx-target="this" hx-swap="innerHTML">
  <div class="loading-placeholder">Chargement…</div>
</div>
```

Section brouillons masquée entièrement si vide (pas de placeholder "0").

### CSS

`assets/static/css/pages/app-my-teams.css` — styles `draft-card` (sans
`.draft-budget`, supprimé de la maquette), suppression de l'usage de
`teams-grid`/`team-card` sur cette page.

---

## Checklist

- [ ] `DraftTeamCardVm` avec `roster: Option<String>`
- [ ] Handler `my_teams` réécrit (exclusion des soumis + enrichissement roster)
- [ ] Template `my-teams.html` restructuré (draft-card inline + slot widget)
- [ ] CSS page mise à jour
- [ ] Section brouillons masquée si vide
