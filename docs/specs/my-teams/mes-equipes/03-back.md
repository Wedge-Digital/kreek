# Phase 3 — Architecture back — page "Mes équipes"

Règle validée (résout la question ouverte de la phase 2) : **actives** =
`PendingEnrollment` + `Enrolled` (tout `game_phase`) · **archivées** =
`Rejected` + `Dismissed`. Maquette `app-my-teams.html` mise à jour en
conséquence (carte "Refusée" déplacée de la grille active vers la liste
archivée).

## Découverte importante (BC `team_creation`)

Le handler actuel (`my_teams.rs`) a un stub caché : `roster: String::new()`
et `tv: 0` sont codés en dur, jamais remplis. Seul
`team_repository.find_by_coach_and_space` (table `team_drafts`) est appelé —
jamais `roster_repository.find_by_id` (table `team_roster_selections`, qui
porte le roster choisi et le budget). La maquette (roster affiché sur les
brouillons) demande donc un vrai enrichissement du handler, pas seulement un
nettoyage visuel.

## Fichiers — BC `team_creation`

| Fichier | Changement |
|---|---|
| `io/web/my_teams.rs` | Handler réécrit : ne garde que les `DraftTeam` **exclus** de `submitted_ids` (les équipes soumises sortent entièrement de ce BC). Pour chaque brouillon restant, appelle `roster_repository.find_by_id` pour récupérer le nom du roster si déjà choisi (`Option<String>`, absent si encore au stade ruleset). |
| `io/web/templates/my-teams.html` | Restructuré : section brouillons rendue inline (nouveau markup `draft-card`, pas le macro `team-card` partagé — style visuel différent) + un seul slot `hx-get="{{ app_routes.teams.my_teams_widget(space_id) }}"` `hx-trigger="load"` pour la section BC `teams`. |
| `assets/static/css/pages/app-my-teams.css` | Mis à jour : styles `draft-card` (plus de `.draft-budget`), suppression de l'usage de `teams-grid`/`team-card` sur cette page. |

## Fichiers — BC `teams` (nouveau widget)

| Fichier | Changement |
|---|---|
| `routes.rs` | + `path::MY_TEAMS_WIDGET = "/app/{space_id}/team/widgets/my-teams"` + `Routes::my_teams_widget(&self, space_id)`, même convention que `pending_enrollment_widget`/`enrolled_teams_widget`. |
| `router.rs` | Wiring `GET` → nouveau handler. |
| `io/web/widgets/my_teams_widget.rs` *(nouveau)* | `AuthSession` (coach courant) + `Path(space_id)`. Appelle `team_repository.find_by_coach_and_space(coach_id, space_id)`, regroupe en `actives`/`archivées` selon la règle validée, mappe `status`+`game_phase` → badge (fonction pure ≤ 20 lignes, colocalisée). |
| `io/web/templates/widgets/my-teams-widget.html` *(nouveau)* | Réutilise le macro partagé `components/team-card.html` pour la grille active. Nouveau markup pour la liste archivée (compacte, propre au widget). `hx-disinherit="*"` sur la racine, CSS embarqué (`assets/static/css/widgets/my-teams-widget.css`). |
| `ports.rs` | + méthode `find_by_coach_and_space` sur `ITeamRepository`, + struct `MyTeamRow` (DTO de lecture : primitives OK, `team_id`, `team_name`, `roster_name`, `logo_url`, `status`, `game_phase`). |
| `io/repository/team_repository.rs` | Implémentation : même patron que `find_enrolled_for_season`, filtre `WHERE coach_id = $1 AND space_id = $2` (sans restriction de statut — les 4 valeurs remontent, le tri actif/archivé se fait en mémoire dans le handler). |

## Ports / adapters inter-BC

Aucun. Chaque section est servie directement par son BC propriétaire au
navigateur (2 requêtes HTTP distinctes émises par le navigateur, pas d'appel
serveur-à-serveur).

## Domain services

Aucun — pas de DTO de port à transformer. Le mapping `status`+`game_phase` →
badge est de la présentation pure sur des primitives déjà lues depuis la
projection, pas une transformation domaine.
