# BC `players` — Widget tableau des joueurs

**Priorité : haute**
**Dépend de :** `63-players-bc-projection.md`
**Contexte :** BC `players` — couche IO web

## Objectif

Exposer un fragment HTML HTMX qui affiche le tableau des joueurs d'une équipe.
Ce widget est consommé par la page équipe du BC `teams` via `hx-get + hx-trigger="load"`,
conformément au principe de souveraineté des données (un BC expose ses widgets, les autres
les embarquent).

---

## Route

```
GET /players/widget/team/{team_id}
```

Paramètres query optionnels :
- `space_id` (pour filtrage de sécurité à terme)

---

## Handler

```rust
// src/app/players/io/web/player_table.rs

pub async fn player_table_widget(
    Path(team_id): Path<String>,
    State(state):  State<AppState>,
) -> impl IntoResponse {
    let team_id = TeamId(team_id);
    let players = state.players.projection_repository
        .find_by_team_id(&team_id)
        .await
        .unwrap_or_default();

    PlayerTableTemplate { players }.into_response()
}
```

---

## Template

```
src/app/players/io/web/templates/player-table-fragment.html
```

Colonnes du tableau minimal :

| Colonne | Source |
|---|---|
| # | `jersey` (ou `—` si None) |
| Nom | `personal_name` ou `position_name` si vide |
| Poste | `position_name` |
| Compétences de base | `base_skills` (noms via ref ou stocker labels) |
| Compétences acquises | `acquired_skills` avec badge Chosen/Random |
| SPP | `spp` |
| Valeur | `value_kpo` kPo |

---

## Intégration page équipe (BC `teams`)

Dans le template de la page équipe du BC `teams` :

```html
<!-- Widget joueurs — fourni par BC players -->
<div hx-get="/players/widget/team/{{ team_id }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="outerHTML">
  <div style="color:var(--dark-4);font-style:italic;">Chargement des joueurs…</div>
</div>
```

BC `teams` ignore tout des données joueurs. Il embarque le widget par son URL.

---

## Route à enregistrer

```rust
// src/app/players/router.rs
Router::new()
    .route("/players/widget/team/:team_id", get(player_table_widget))
```

Monté dans le router principal de l'application.

---

## Checklist

- [ ] Route `GET /players/widget/team/:team_id`
- [ ] Handler `player_table_widget` lisant depuis `players_projection`
- [ ] Template `player-table-fragment.html` avec les colonnes définies
- [ ] CSS (peut réutiliser les styles existants de `player-table`)
- [ ] Intégration dans la page équipe du BC `teams`
- [ ] Câblage du router `players` dans le router principal (`main.rs`)
