# Phase 7 — Intégration — page "Mes équipes"

## CSS — deux systèmes de badge parallèles existent déjà

`team_detail.rs` utilise `.team-status-badge--{{pending|ready|phase|offseason|dismissed}}`
(`assets/static/css/pages/app-team-detail.css:34-38`). Le composant partagé
`team-card.html` utilise un scope différent, `.team-card-status--{{status}}`
(`assets/static/css/components/team-card.css`), qui ne définit que
`--draft` et `--active` aujourd'hui. Comme la grille active réutilise ce
composant, il faut ajouter les nouvelles variantes sous son propre préfixe,
avec les mêmes couleurs que la page détail :

```css
/* assets/static/css/components/team-card.css — ajouts */
.team-card-status--pending   { background: rgba(255,107,53,0.12);  color: var(--orange); }
.team-card-status--ready     { background: rgba(98,149,132,0.15); color: var(--green); }
.team-card-status--phase     { background: rgba(0,48,73,0.10);    color: var(--main-blue); }
.team-card-status--offseason { background: var(--dark-6);         color: var(--dark-3); }
```

Le badge des cartes archivées (markup inline, pas le composant partagé)
reste dans le CSS propre du widget — un seul style suffit ("Refusée" et
"Renvoyée" sont les deux seuls libellés possibles côté archivé, tous deux
`dismissed`).

## Persistance

- `teams` : nouvelle méthode
  `ITeamRepository::find_by_coach_and_space(coach_id, space_id) -> Vec<MyTeamRow>`,
  implémentée sur `team_proj` (`WHERE coach_id = $1 AND space_id = $2`,
  aucune restriction de statut). Aucune migration — toutes les colonnes
  existent déjà.
- `team_creation` : aucune nouvelle méthode. `my_teams.rs` utilise trois
  appels déjà existants : `team_repository.find_by_coach_and_space`
  (drafts), `roster_repository.find_submitted_ids_for_space` (exclusion),
  `roster_repository.find_by_id` par brouillon restant (nom du roster —
  nouvel usage, méthode déjà là).

## Événements

Aucun (page 100% lecture, confirmé phase 6).

## Handlers

```rust
// team_creation/io/web/my_teams.rs
pub async fn my_teams(auth_session: AuthSession, Path(space_id): Path<String>, State(state): State<AppState>) -> impl IntoResponse
```
```rust
// teams/io/web/widgets/my_teams_widget.rs (nouveau)
pub async fn my_teams_widget(auth_session: AuthSession, Path(space_id): Path<String>, State(state): State<AppState>) -> impl IntoResponse
```

## Templates

- `team_creation/io/web/templates/my-teams.html` — restructuré (section
  brouillons inline + slot `hx-get` widget).
- `teams/io/web/templates/widgets/my-teams-widget.html` *(nouveau)* —
  grille active (macro `team-card`) + liste archivée (markup propre),
  racine `hx-disinherit="*"`.
- `src/web/templates/components/team-card.html` + `competition-teams.html`
  — mis à jour (paramètre `initials`).

## États (comble un trou de la Phase 1 — non maquettés à l'époque)

- Section vide (brouillons / actives / archivées) → section masquée
  entièrement (titre + compteur + contenu), pas de placeholder "0". Chaque
  section gère son propre vide indépendamment — pas d'état combiné inter-BC.
- Erreur repository → `StatusCode::INTERNAL_SERVER_ERROR` + log, même
  pattern que `enrolled_teams_widget.rs` (pas de fragment d'erreur dédié).
- Chargement du widget → placeholder texte simple pendant le `hx-get`,
  comme l'existant.

## Tests E2E prévus

1. Coach avec brouillons (roster choisi + non choisi), équipes actives
   (plusieurs `game_phase`), une refusée, une renvoyée → 3 sections
   correctement peuplées, bons libellés.
2. Clic "Continuer" sur un brouillon → navigue vers la page de build.
3. Clic sur une carte active/archivée → navigue vers le détail d'équipe.
4. Coach sans aucune équipe → aucune des 3 sections ne s'affiche (pas
   d'état combiné inter-BC prévu pour l'instant).
