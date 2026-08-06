# BC `players` — Widget joueurs : mode édition (front)

**Priorité : haute**
**Dépend de :** `290-players-roster-edit-domain.md`
**Contexte :** `players` — widget HTMX

## Objectif

Étendre le widget joueurs existant pour porter le mode édition (nom,
numéro, ordre) maquetté et validé dans `app-team-detail.html` : bascule
lecture/édition, glisser-déposer, validation de doublon en direct, bandeau
d'erreur. Comble aussi la dette de convention sur ce fichier (fortement
modifié → doit suivre `_widget.rs` sous `widgets/`).

**Spec de référence :** `docs/specs/player-edition/team-detail/02-front.md`,
`03-back.md`, `04-dtos.md`. **Maquette de référence (copier-coller
obligatoire, règle 5 CLAUDE.md) :**
`assets/rawpages/html/app-team-detail.html`.

---

## Conception

### Renommage

`players/io/web/player_table.rs` → `players/io/web/widgets/player_table_widget.rs`
(copié-collé intégral, pas de réécriture). Mettre à jour l'import dans
`players/router.rs`.

### `PlayerTableTemplate` — extension

```rust
pub struct PlayerTableTemplate {
    pub app_routes: AppRoutes,
    pub space_id: String,
    pub team_id: String,            // nouveau
    pub players: Vec<PlayerRowVm>,  // inchangé
    pub save_error: Option<String>, // nouveau
}
```

`PlayerRowVm` : aucun changement — `jersey`/`personal_name` déjà présents.

### Route (`players/routes.rs`)

```rust
pub const PLAYERS_ROSTER_UPDATE: &str = "/app/{space_id}/players/by-team/{team_id}/roster";
pub fn update_roster(&self, space_id: &str, team_id: &str) -> String { ... }
```

Uniquement la constante + le builder d'URL dans cette carte — le handler
POST et son wiring dans `router.rs` sont pour la carte suivante (294). La
constante doit exister pour que le template compile
(`app_routes.players.update_roster(...)`).

### Template (`player-table-fragment.html`)

Repris **tel quel** de la maquette validée (règle 5 CLAUDE.md — copier-coller,
pas de réécriture de mémoire), adapté au binding Askama :
- `<form>` autour de `#roster-tbody`, `hx-post="{{ app_routes.players.update_roster(space_id, team_id) }}"`, `hx-trigger="rosterEditSaveRequested from:body"`, `hx-target="this"`, `hx-swap="outerHTML"`.
- Racine `.player-table` : `{% if save_error.is_some() %}edit-mode{% endif %}` — déjà en mode édition au premier rendu si on revient d'un échec.
- Bandeau d'erreur inline si `save_error.is_some()`.
- Poignée de glisser-déposer, cellules `#`/Nom en `display-value`/`edit-value` (input), colonnes restantes `cell-readonly` — markup et JS (`toggleRosterEdit`, `onJerseyInput`, `enableRosterDragAndDrop`, événements DOM) copiés de la maquette, adaptés pour écouter `rosterEditRequested`/`rosterEditCancelRequested from:body` au lieu des fonctions globales de démo.
- Boucle Askama sur `players` pour générer les lignes (`name="jersey[]"`/`name="personal_name[]"`/`name="player_id[]"` en hidden input par ligne).

---

## Checklist

- [ ] Renommage `player_table.rs` → `widgets/player_table_widget.rs` (import `router.rs` mis à jour)
- [ ] `PlayerTableTemplate` : `team_id` + `save_error`
- [ ] `PLAYERS_ROSTER_UPDATE` + `Routes::update_roster()`
- [ ] Template : formulaire + listeners `hx-trigger` DOM
- [ ] Template : classe `edit-mode` conditionnelle sur `save_error`
- [ ] Template : bandeau d'erreur inline
- [ ] Template : glisser-déposer, inputs nom/numéro (repris maquette)
- [ ] Vérifier au navigateur : mode édition identique à la maquette validée
