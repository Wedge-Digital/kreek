> **Annulée — remplacée par un découpage.** Cette carte datait d'avant le
> passage par le workflow "nouvelle fonctionnalité" complet. Le design a
> depuis évolué (section "archivées" ajoutée, filtres et budget retirés,
> statut "Refusée" pris en compte, bug du roster non affiché découvert et
> corrigé) — voir `docs/specs/my-teams/` pour la spec complète et les
> cartes `285` à `289` qui la remplacent.

# Page "Mes équipes" — deux sections BC `team_creation` + BC `teams`

**Priorité : haute**
**Dépend de :** `31-team-created-listener.md`, `42-teams-projection.md`
**Contexte :** `team_creation` (page hôte) + `teams` (widget section active)

## Objectif

Restructurer la page "Mes équipes" en deux sections distinctes :
1. **En cours de création** — brouillons rendus directement par BC `team_creation`
2. **Mes équipes actives** — widget chargé via HTMX depuis BC `teams`

**Maquette de référence :** `assets/rawpages/html/app-my-teams.html`

---

## Conception

### Architecture de composition

La page est owned par BC `team_creation` (route et handler existants). La section active est un slot HTMX pointant vers BC `teams` :

```html
<!-- Dans my-teams.html (BC team_creation) -->

<!-- Section 1 : brouillons — rendu direct par le handler -->
<div class="section-title">En cours de création <span>{{ drafts.len() }}</span></div>
{% for draft in drafts %}
<div class="draft-card">
  <div class="draft-logo">{{ draft.initials }}</div>
  <div class="draft-info">
    <div class="draft-name">{{ draft.name }}</div>
    <div class="draft-roster">{{ draft.roster_name }}</div>
  </div>
  <span class="draft-status">Brouillon</span>
  <div class="draft-budget">Budget utilisé : <span>{{ draft.budget_used }} / {{ draft.budget_total }} kPo</span></div>
  <a class="btn-draft-continue"
     hx-get="{{ team_routes.team_build(space_id, draft.id) }}"
     hx-target="#app-content" hx-swap="innerHTML" hx-push-url="true">
    Continuer →
  </a>
</div>
{% endfor %}

<!-- Section 2 : équipes actives — widget BC teams -->
<div class="section-title">Mes équipes actives</div>
<div id="active-teams"
     hx-get="{{ teams_routes.my_teams_widget(space_id) }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="innerHTML">
  <div class="loading-placeholder">Chargement…</div>
</div>
```

### Section brouillons — BC `team_creation`

Le handler existant `my_teams` est adapté pour ne produire que les équipes en statut `draft` (non soumises).

```rust
pub struct DraftTeamCardVm {
    pub id:           String,
    pub initials:     String,
    pub name:         String,
    pub roster_name:  String,
    pub budget_used:  u32,   // kPo dépensés
    pub budget_total: u32,   // kPo budget initial du roster
    pub link:         String,
}
```

### Section équipes actives — BC `teams`

Nouvel endpoint dans BC `teams` :

```
GET /app/{space_id}/teams/widget/my-teams
```

Retourne un **fragment HTML** (pas une page complète) contenant :
- Les filtres (statut, compétition)
- La grille de `TeamCardVm`

```rust
pub struct TeamCardVm {
    pub id:           String,
    pub initials:     String,
    pub name:         String,
    pub roster_name:  String,
    pub tv_kpo:       u32,
    pub status:       String,        // "ready" | "pending" | "post_match" | "dismissed"
    pub status_label: String,        // "Prête à jouer" | "En attente d'inscription" | …
    pub game_phase:   Option<String>,
    pub link:         String,
}
```

Les données proviennent de `teams_projection` via `find_by_coach_and_space()`.

### Badges de statut sur les cartes actives

| `participation_status` + `game_phase` | Label | Style |
|---|---|---|
| `pending_enrollment` | En attente d'inscription | orange |
| `enrolled` + `ready_to_play` | Prête à jouer | vert |
| `enrolled` + `player_improvement` | Amélioration joueurs | bleu |
| `enrolled` + `recruitment` | Recrutement | bleu |
| `enrolled` + `dismissals` | Renvois | bleu |
| `enrolled` + `temporary_retirement` | Retraite temporaire | bleu |
| `enrolled` + `off_season` | Repos hors-saison | gris |
| `dismissed` | Renvoyée | rouge |

### Filtres de la section active

- **Statut** — filtré côté serveur via paramètre query `?status=ready`
- **Compétition** — `competition_name` et `season_name` sont dénormalisés dans `teams_projection` depuis le payload de `TeamEnrolled` (à ajouter à la migration de la carte 42)

---

## Impact sur la carte 42

Ajouter dans `teams_projection` :
```sql
competition_name TEXT,   -- NULL si non inscrite, dénormalisé depuis TeamEnrolled
season_name      TEXT,   -- idem
```
Et mettre à jour `update_projection_in_tx()` pour le variant `TeamEnrolled`.

---

## Checklist

- [ ] Handler `my_teams` adapté : ne produit que les brouillons (`DraftTeamCardVm`)
- [ ] `DraftTeamCardVm` avec `budget_used` + `budget_total`
- [ ] Template `my-teams.html` restructuré : section brouillons + slot `hx-get` section active
- [ ] Nouvel endpoint `GET /app/{space_id}/teams/widget/my-teams` dans BC `teams`
- [ ] `TeamCardVm` avec statut + game_phase
- [ ] Fragment template `my-teams-widget.html` dans BC `teams` (filtres + grille)
- [ ] Badge de statut : mapping `participation_status` + `game_phase` → label + style CSS
- [ ] Ajouter `competition_name` + `season_name` dans `teams_projection` (carte 42)
- [ ] Filtre par statut opérationnel (query param)
- [ ] Section brouillons masquée si aucun brouillon
