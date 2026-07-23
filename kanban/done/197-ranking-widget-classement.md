# BC `ranking` — Widget Classement (handler + template + CSS)

**Priorité : haute**
**Dépend de :** `196-ranking-listener-et-cablage.md`
**Contexte :** `ranking/io/web/widgets/`, `ranking/templates/widgets/`
**Spec :** `docs/specs/ranking/classement/02-front.md`, `04-dtos.md`, `07-integration.md`

## Objectif

Le widget lecture seule affiché en premier onglet de la page détail compétition — 4 états : tableau, "aucune équipe", "aucun match joué", erreur (règles non configurées).

## Conception

### VMs (`ranking/io/web/widgets/classement_widget.rs`)

```rust
pub struct ClassementRowVm { pub rank: u32, pub team_name: String, pub played: u32, pub wins: u32, pub draws: u32, pub losses: u32, pub points: u32 }
pub struct ClassementWidgetVm { pub rules_missing: bool, pub has_enrolled_teams: bool, pub rows: Vec<ClassementRowVm> }
```

`ClassementRowVm` construit dans `builders.rs` (dépend du port `EnrolledTeamInfo`, pas de `from_domain()` sur le VM — règle CLAUDE.md). Tri par `ranking_points` décroissant, `rank` assigné à la construction (jamais stocké).

### Handler

```rust
pub async fn classement_widget(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

401 si pas de session (accès à tout utilisateur connecté, pas réservé admin). Charge en parallèle (`tokio::join!`) `find_ranking_rules`, `find_enrolled_teams`, `find_latest_lines_for_season`. Construit `ClassementWidgetVm`, rend le template.

### Template + CSS

`templates/widgets/classement-widget.html` : `{% if rules_missing %}` erreur `{% else if !has_enrolled_teams %}` vide "aucune équipe" `{% else if rows.is_empty() %}` vide "aucun match joué" `{% else %}` tableau. Racine avec `hx-disinherit="*"`. CSS propre dans `assets/static/css/widgets/classement-widget.css` (dupliqué/adapté depuis `competition-detail.css`, pas de dépendance à ce fichier).

### Route

`GET /app/{space_id}/ranking/{competition_id}/{season_id}/widget` → branché dans `ranking/router.rs`.

## Checklist

- [ ] `ClassementRowVm`, `ClassementWidgetVm`, `builders.rs::build_classement_rows`
- [ ] Handler `classement_widget` (401 si pas de session, `tokio::join!` pour les 3 sources)
- [ ] Template `classement-widget.html` (4 états, `hx-disinherit="*"`)
- [ ] `assets/static/css/widgets/classement-widget.css` (autonome)
- [ ] Route branchée dans `ranking/router.rs` + `routes.rs`
- [ ] Test unitaire `build_classement_rows` (tri, calcul du rang, résolution des noms)
- [ ] `cargo check` + `cargo test` passent
- [ ] Vérification manuelle des 4 états dans le navigateur (via le serveur dev de l'utilisateur, pas démarré par Claude)
