# BC `teams` — Fiche d'équipe câblée avec données réelles

**Priorité : haute**
**Dépend de :** `31-team-created-listener.md`
**Contexte :** `teams` — lecture

## Objectif

Remplacer les données statiques du template `team-detail.html` par de vraies données lues depuis le BC `teams`. Afficher l'identité de l'équipe, son statut et son staff. Le tableau des joueurs est un **widget délégué au BC `players`** chargé via HTMX — le BC `teams` n'interroge pas les données joueurs.

---

## Conception

### Principe de composition

Conformément à la règle de souveraineté des données (CLAUDE.md), le BC `teams` ne fait aucune requête SQL sur les joueurs. La page délègue cette section au BC `players` via un `hx-get` chargé au montage :

```html
<!-- Dans team-detail.html, fourni par BC teams -->
<div id="players-widget"
     hx-get="{{ players_routes.team_roster_widget(space_id, team_id) }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="innerHTML">
  <div class="loading-placeholder">Chargement des joueurs…</div>
</div>
```

Le BC `players` expose l'endpoint `/app/{space_id}/players/by-team/{team_id}/widget` qui rend le fragment HTML du tableau.

### View model (BC `teams` uniquement)

```rust
pub struct TeamDetailVm {
    pub id:                   String,
    pub name:                 String,
    pub initials:             String,
    pub roster_name:          String,
    pub coach_name:           String,
    pub dedicated_fans:       u8,
    pub treasury_kpo:         u32,
    pub participation_status: String,
    pub game_phase:           Option<String>,
    pub status_label:         String,
    pub staff:                StaffVm,
    pub players_widget_url:   String,  // URL vers le widget du BC players
    // record V/N/D : délégué au BC match_report (carte ultérieure)
}
```

### Handler

```rust
pub async fn team_detail(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, AppError> {
    let team = state.teams_ctx.team_repository
        .find_by_id(&TeamId::from(team_id))
        .await?
        .ok_or(AppError::NotFound)?;
    Ok(TeamDetailTemplate { vm: TeamDetailVm::from(team, &space_id), ... })
}
```

---

## Checklist

- [ ] `TeamDetailVm` + `StaffVm` dans `web/` (sans `PlayerVm`)
- [ ] Handler `team_detail` lit uniquement depuis `ITeamRepository`
- [ ] Template `team-detail.html` câblé avec les variables Askama
- [ ] Slot `hx-get` pour le widget joueurs pointant vers le BC `players`
- [ ] Badge statut dynamique selon `participation_status` + `game_phase`
- [ ] Initiales calculées depuis le nom
- [ ] Route enregistrée dans `router.rs`
