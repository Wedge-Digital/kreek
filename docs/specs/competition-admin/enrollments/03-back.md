# Inscriptions — Phase 3 : Architecture back ✅

## BCs impliqués

- **competitions** : fragment onglet (assemblage), clôture des inscriptions
- **teams** : widgets équipes (pending, enrolled), actions enrollment (approve, reject, dismiss, approve-all)

## Fragment onglet Inscriptions — BC `competitions`

Page d'assemblage : stats inline + conteneurs `hx-get` vers les widgets du BC `teams`.

### Route

```
GET /app/{space_id}/competitions/{competition_id}/{season_id}/admin/enrollments
    → fragment onglet inscriptions
```

### Fichiers

```
src/app/competitions/io/web/admin/
├── enrollments_tab.rs                  ← handler GET fragment
└── templates/admin/
    └── enrollments.html                ← fragment (stats + conteneurs widgets)

assets/static/css/pages/
└── competition-admin-enrollments.css   ← styles spécifiques
```

## Widgets équipes — BC `teams`

Deux widgets GET autonomes, chargés par le fragment onglet via `hx-get`.

### Routes

```
GET /app/{space_id}/team/widgets/pending?competition_id=...&season_id=...
    → widget liste des équipes en attente de validation

GET /app/{space_id}/team/widgets/enrolled?competition_id=...&season_id=...
    → widget liste des équipes inscrites (validées)
```

### Fichiers

```
src/app/teams/io/web/widgets/
├── pending_enrollment_widget.rs        ← handler GET + template
├── enrolled_teams_widget.rs            ← handler GET + template
└── templates/widgets/
    ├── pending-enrollments.html         ← liste pending + boutons Valider/Refuser + Tout valider
    └── enrolled-teams.html              ← liste enrolled + bouton Renvoyer
```

### Données

Chaque widget charge les équipes depuis le repository du BC `teams`, filtrées par `competition_id`, `season_id` et `ParticipationStatus` (PendingEnrollment / Enrolled).

## Actions enrollment — BC `teams`

### Routes

```
POST /app/{space_id}/team/{team_id}/enrollment/approve   → HX-Trigger: enrollmentChanged
POST /app/{space_id}/team/{team_id}/enrollment/reject    → HX-Trigger: enrollmentChanged
POST /app/{space_id}/team/{team_id}/enrollment/dismiss   → HX-Trigger: enrollmentChanged
POST /app/{space_id}/team/widgets/pending/approve-all?competition_id=...&season_id=...
                                                         → HX-Trigger: enrollmentChanged
```

### Fichiers

```
src/app/teams/io/web/
├── enrollment_actions.rs               ← 4 handlers POST

src/app/teams/use_cases/
├── approve_enrollment.rs               ← use case approve
├── reject_enrollment.rs                ← use case reject
├── dismiss_team.rs                     ← use case dismiss
```

### Événements domaine → app events

La validation d'une inscription (`approve`) produit un **domain event** `TeamEnrollmentApproved` dans le BC `teams`. Cet événement est publié sur l'app event bus et capté par un **listener** dans le BC `competitions`, qui l'enregistre dans l'activité récente de la compétition (visible sur le dashboard admin).

```
BC teams (domaine)
│
├── approve_enrollment use case
│   └── team.approve_enrollment()  ──► TeamEnrollmentApproved (domain event)
│                                            │
│                                            ▼
│                                       Event store (teams)
│
└── App event publisher (teams IO)
    └── publie TeamEnrollmentApproved sur l'app event bus
                    │
                    ▼
         BC competitions (IO / listener)
         └── enrollment_approved_listener
             └── enregistre l'activité dans la projection "recent_activity"
```

De même pour `reject` → `TeamEnrollmentRejected` et `dismiss` → `TeamDismissed`.

## Clôture des inscriptions — BC `competitions`

### Route

```
POST /app/{space_id}/competitions/{competition_id}/{season_id}/admin/enrollments/close
    → HX-Trigger: enrollmentChanged
```

### Fichier

```
src/app/competitions/io/web/admin/
└── enrollment_actions.rs               ← handler POST close
```

Change le statut de la saison pour interdire les nouvelles inscriptions.

## Ports nécessaires

Aucun port inter-BC — la composition est faite en front (widgets HTMX). Les événements transitent par l'app event bus.

## Middleware d'autorisation

Les handlers admin (fragment onglet, clôture) réutilisent le même guard que la page hôte (admin espace OU admin compétition).

Les widgets et actions du BC `teams` ne vérifient pas le statut admin de la compétition — ils vérifient que l'utilisateur est membre de l'espace (guard existant).
