# Inscriptions — Phase 7 : Intégration ✅

## Persistance

### Event store — BC `teams`

Les nouveaux événements (`TeamEnrollmentRejected`) sont persistés via le mécanisme existant d'event store du BC `teams`. Pas de modification du mécanisme — seul le nouveau variant est ajouté à la sérialisation.

### Projections — BC `teams`

Ajouter/mettre à jour la table de projection pour lister les équipes par `participation_status` et `season_id`.

Méthode à ajouter sur `ITeamRepository` :

- `find_by_season_and_status(season_id: &str, status: &str) -> Vec<TeamEnrollmentProjection>`

```rust
pub struct TeamEnrollmentProjection {
    pub team_id: String,
    pub team_name: String,
    pub coach_name: String,
    pub roster_name: String,
    pub tier_name: String,
    pub group_name: Option<String>,
}
```

## Handlers — BC `competitions`

### `enrollments_tab.rs`

- Route : `GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin/enrollments`
- Guard admin (même que page hôte)
- Rend `EnrollmentsTabTemplate` (assemblage pur : conteneurs `hx-get` vers les widgets teams)
- Câblage inline dans `admin_page.rs` quand `active_tab == "enrollments"`

## Handlers — BC `teams`

### `pending_enrollment_widget.rs`

- Route : `GET /app/{space_id}/team/widgets/pending?competition_id=...&season_id=...`
- Charge les équipes avec `find_by_season_and_status(season_id, "PendingEnrollment")`
- Rend `PendingEnrollmentWidgetTemplate`

### `enrolled_teams_widget.rs`

- Route : `GET /app/{space_id}/team/widgets/enrolled?competition_id=...&season_id=...`
- Charge les équipes avec `find_by_season_and_status(season_id, "Enrolled")`
- Rend `EnrolledTeamsWidgetTemplate`

### `enrollment_actions.rs`

4 handlers POST :

- `POST /app/{space_id}/team/{team_id}/enrollment/approve`
  1. Charger l'agrégat Team (hydrate)
  2. Appeler `approve_enrollment::execute`
  3. Retourner `HX-Trigger: enrollmentChanged`

- `POST /app/{space_id}/team/{team_id}/enrollment/reject`
  1. Charger l'agrégat Team (hydrate)
  2. Appeler `reject_enrollment::execute`
  3. Retourner `HX-Trigger: enrollmentChanged`

- `POST /app/{space_id}/team/{team_id}/enrollment/dismiss`
  1. Charger l'agrégat Team (hydrate)
  2. Appeler `dismiss_team::execute`
  3. Retourner `HX-Trigger: enrollmentChanged`

- `POST /app/{space_id}/team/widgets/pending/approve-all?competition_id=...&season_id=...`
  1. Charger toutes les équipes pending pour cette saison
  2. Itérer et appeler `approve_enrollment::execute` pour chacune
  3. Retourner `HX-Trigger: enrollmentChanged`

## Templates

### `admin/enrollments.html` — BC `competitions`

Fragment onglet, assemblage pur :
```html
<div id="pending-container"
     hx-get="{{ app_routes.teams.pending_enrollment_widget(space_id) }}?competition_id={{ competition_id }}&season_id={{ season_id }}"
     hx-trigger="load, enrollmentChanged from:body"
     hx-swap="innerHTML">
</div>

<div id="enrolled-container"
     hx-get="{{ app_routes.teams.enrolled_teams_widget(space_id) }}?competition_id={{ competition_id }}&season_id={{ season_id }}"
     hx-trigger="load, enrollmentChanged from:body"
     hx-swap="innerHTML">
</div>
```

### `widgets/pending-enrollments.html` — BC `teams`

- Panel avec header "En attente de validation" + badge compteur
- Bouton "Tout valider" en haut
- Liste de rows : logo, nom, coach, roster, tier + boutons Valider/Refuser
- `hx-disinherit="*"` sur la racine

### `widgets/enrolled-teams.html` — BC `teams`

- Panel avec header "Équipes inscrites" + badge compteur
- Liste de rows : logo, nom, coach, roster, tier, poule + bouton Renvoyer
- `hx-disinherit="*"` sur la racine

## CSS

- `assets/static/css/pages/competition-admin-enrollments.css` — styles spécifiques : enrollment rows, status badges, action buttons

## App events — inter-BC

Les domain events du BC `teams` sont publiés sur l'app event bus et captés par des listeners dans le BC `competitions` :

| Domain event (teams) | App event | Listener (competitions) | Action |
|---|---|---|---|
| `TeamEnrolled` | `TeamEnrolled` | `enrollment_approved_listener` | Enregistre dans la projection activité récente |
| `TeamEnrollmentRejected` | `TeamEnrollmentRejected` | `enrollment_rejected_listener` | Enregistre dans la projection activité récente |
| `TeamDismissed` | `TeamDismissed` | `team_dismissed_listener` | Enregistre dans la projection activité récente |

Ces entrées alimentent le fil d'activité du dashboard admin.

## Tests E2E

Fichier : `tests/e2e/test_competition_admin_enrollments.py`

### Scénarios

1. **Onglet se charge** : naviguer vers l'onglet Inscriptions → les widgets pending et enrolled sont visibles
2. **Valider une inscription** : cliquer "Valider" sur une équipe pending → elle disparaît de pending et apparaît dans enrolled
3. **Refuser une inscription** : cliquer "Refuser" sur une équipe pending → elle disparaît de pending
4. **Renvoyer une équipe** : cliquer "Renvoyer" sur une équipe enrolled → elle disparaît de enrolled

Note : ces tests nécessitent qu'une équipe soit en état `PendingEnrollment` pour la compétition testée. Le test devra créer une compétition + soumettre une équipe avant de tester les actions admin.
