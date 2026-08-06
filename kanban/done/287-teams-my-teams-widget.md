# BC `teams` — Widget "Mes équipes" (actives + archivées)

**Priorité : haute**
**Dépend de :** `285-teams-my-teams-repository.md`, `286-team-card-component-initials.md`
**Contexte :** `teams` — widget HTMX

## Objectif

Nouveau widget servant les sections "Mes équipes actives" et "Mes équipes
archivées" de la page hôte `team_creation` (carte 288). Une seule requête,
un seul fragment HTML, regroupement fait en mémoire selon la règle validée :
actives = `PendingEnrollment`/`Enrolled` (tout `game_phase`), archivées =
`Rejected`/`Dismissed`.

**Spec de référence :** `docs/specs/my-teams/mes-equipes/02-front.md`,
`04-dtos.md`, `06-domaine.md`, `07-integration.md`.

---

## Conception

### Route

```rust
// teams/routes.rs
pub const MY_TEAMS_WIDGET: &str = "/app/{space_id}/team/widgets/my-teams";
pub fn my_teams_widget(&self, space_id: &str) -> String {
    path::MY_TEAMS_WIDGET.replace("{space_id}", space_id)
}
```

### Mapping statut + phase → libellé/classe

Reprend **exactement** `team_detail.rs::status_display()` (lignes 242-260) —
pas de nouveau vocabulaire :

| `status` | `game_phase` | Groupe | Libellé | Classe |
|---|---|---|---|---|
| PendingEnrollment | — | active | En attente d'inscription | `pending` |
| Enrolled | ReadyToPlay | active | Prête à jouer | `ready` |
| Enrolled | MatchReporting | active | Rapport en cours | `phase` |
| Enrolled | PlayerImprovement | active | Phase d'amélioration | `phase` |
| Enrolled | Recruitment | active | Phase de recrutement | `phase` |
| Enrolled | Dismissals | active | Phase de renvois | `phase` |
| Enrolled | TemporaryRetirement | active | Retraite temporaire | `phase` |
| Enrolled | OffSeason | active | Off-season | `offseason` |
| Enrolled | *(null)* | active | Inscrite | `ready` |
| Rejected | — | archivée | Inscription refusée | `dismissed` |
| Dismissed | — | archivée | Renvoyée | `dismissed` |

```rust
// teams/io/web/widgets/my_teams_widget.rs
// Duplication assumée de team_detail.rs::status_display (garder synchronisées
// manuellement) : ici sur strings issues de team_proj, pas sur l'agrégat.
fn status_label_and_class(status: &str, game_phase: Option<&str>) -> (String, String) { ... }
```

### VMs

```rust
pub struct TeamCardVm {
    pub team_id: String, pub initials: String, pub team_name: String,
    pub roster_name: String, pub logo: Option<String>,
    pub status_class: String, pub status_label: String, pub link: String,
}
pub struct ArchivedTeamCardVm {
    pub team_id: String, pub initials: String, pub team_name: String,
    pub roster_name: String, pub status_label: String, pub link: String,
}
```

### Handler

```rust
pub async fn my_teams_widget(
    auth_session: AuthSession,
    Path(space_id): Path<String>,
    State(state): State<AppState>,
) -> impl IntoResponse
```
1. `rows = team_repository.find_by_coach_and_space(coach_id, space_id)`
2. Pour chaque row : `status_label_and_class(&row.status, row.game_phase.as_deref())`
3. Regroupe en `active_teams: Vec<TeamCardVm>` / `archived_teams: Vec<ArchivedTeamCardVm>` selon `row.status`
4. Rend `MyTeamsWidgetTemplate`

### Template

`teams/io/web/templates/widgets/my-teams-widget.html` — racine
`hx-disinherit="*"`, CSS embarqué (`assets/static/css/widgets/my-teams-widget.css`) :
- Grille active : réutilise le macro partagé `components/team-card.html`
- Liste archivée : markup propre au widget (compact, muté visuellement — cf. maquette `.archived-card`)
- Section masquée entièrement si son groupe est vide (pas de placeholder "0")

---

## Checklist

- [ ] `path::MY_TEAMS_WIDGET` + `Routes::my_teams_widget()` dans `teams/routes.rs`
- [ ] Wiring `GET` dans `teams/router.rs`
- [ ] `status_label_and_class()` + test unitaire couvrant les 11 lignes du tableau
- [ ] `TeamCardVm` / `ArchivedTeamCardVm`
- [ ] Handler `my_teams_widget`
- [ ] Template `my-teams-widget.html` (2 sections, masquage si vide)
- [ ] CSS `assets/static/css/widgets/my-teams-widget.css`
