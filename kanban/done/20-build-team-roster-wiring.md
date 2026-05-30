# Câblage widget roster → table joueurs dans build-team

**Priorité : haute**
**Dépend de :** `19-roster-selection-widget.md`
**Contextes :** `team_creation` (page) + `references` (données)

## Objectif

1. Afficher le widget de sélection de roster (carte 19) dans la page de construction d'équipe
2. Au clic sur un chip roster, remplacer dynamiquement (HTMX) le `<tbody>` de la table joueurs (section 3) avec les postes du roster sélectionné

---

## État de l'existant

| Élément | Fichier | Remarque |
|---|---|---|
| Handler | `team_creation/io/web/build_team.rs` | Pas d'`AppState`, pas de données réelles |
| Template | `team_creation/io/web/templates/build-team.html` | Table joueurs 100 % hardcodée, select hardcodé |
| Modèle position | `references/domain/models.rs` → `PlayerPosition` | Stats en `u8` ; `ag/pa/av` à afficher au format `"N+"` |
| Repo references | `AppState::references.repository` | `find_team_by_uid(uid)` disponible |

---

## Conception

### Nouveau handler `get_roster_players`

Route : `GET /app/{space_id}/team/{team_id}/roster/{roster_uid}/players`

```rust
// team_creation/io/web/build_team.rs (ou fichier dédié)
pub async fn get_roster_players(
    Path((space_id, team_id, roster_uid)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse
```

Logique :
1. `state.references.repository.find_team_by_uid(&roster_uid)` → `404` si absent
2. Construire `Vec<PlayerPositionVm>` depuis `team.available_players`
3. Retourner le fragment `roster-players-fragment.html`

### View model

```rust
// references/io/web/pickers.rs (ou team_creation/io/web/build_team.rs)
pub struct PlayerPositionVm {
    pub name:          String,
    pub cost:          u32,
    pub max_qty_label: String,  // "0-16", "0-4", etc.
    pub ma:            u8,
    pub st:            u8,
    pub ag:            String,  // format: "N+" (ag=3 → "3+")
    pub pa:            String,  // format: "N+" ou "—" si pa=0
    pub av:            String,  // format: "N+"
    pub skills:        String,  // Vec<String> joint par ", "
}

fn to_stat_plus(v: u8) -> String {
    if v == 0 { "—".into() } else { format!("{}+", v) }
}
```

### Fragment template

Nouveau fichier : `references/io/web/templates/roster-players-fragment.html`

```html
{% for pos in positions %}
<tr>
  <td>{{ pos.max_qty_label }}</td>
  <td class="player-name">{{ pos.name }}</td>
  <td>{{ pos.cost }} kPo</td>
  <td>{{ pos.ma }}</td><td>{{ pos.st }}</td>
  <td>{{ pos.ag }}</td><td>{{ pos.pa }}</td><td>{{ pos.av }}</td>
  <td class="player-skills">{{ pos.skills }}</td>
  <td>0</td><td>0 kPo</td>
  <td><button class="tbl-btn" type="button">+</button></td>
  <td><button class="tbl-btn" type="button">−</button></td>
</tr>
{% endfor %}
{% if positions.is_empty() %}
<tr><td colspan="13" style="text-align:center; color: var(--dark-3); font-style: italic;">
  Sélectionnez un roster pour afficher les postes disponibles.
</td></tr>
{% endif %}
```

### Modifications de `build-team.html`

1. **Remplacer le `<select>` hardcodé** par l'inclusion du widget tiers (carte 19) — le widget émet `hx-get` vers la route ci-dessus sur chaque chip :

```html
<span class="roster-chip"
      hx-get="{{ team_routes.roster_players(space_id, team_id, item.uid) }}"
      hx-target="#player-table-body"
      hx-swap="innerHTML"
      hx-push-url="false">{{ item.name }}</span>
```

2. **Ajouter `id="player-table-body"` sur le `<tbody>`** pour le ciblage HTMX :

```html
<tbody id="player-table-body">
  <!-- vide au premier chargement ; remplacé par le fragment au clic -->
  <tr><td colspan="13" ...>Sélectionnez un roster...</td></tr>
</tbody>
```

3. **Injecter `tiers: Vec<RosterTierVm>`** dans `BuildTeamTemplate` (alimenté depuis `AppState::references` + `draft_team.creation_rules()`)

### Modifications de `BuildTeamTemplate`

```rust
pub struct BuildTeamTemplate {
    pub web_routes:  WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub space_id:    String,
    pub team_id:     String,
    pub tiers:       Vec<RosterTierVm>,   // nouveau — depuis carte 19
}
```

Handler `build_team` doit :
1. Recevoir `State(state): State<AppState>`
2. Charger le `DraftTeam` depuis `state.team_creation.team_repository.find_by_id(&team_id)`
3. Appeler `build_roster_tiers(state.references.repository.as_ref(), draft_team.creation_rules())` (carte 19)
4. Passer les tiers au template

### Nouvelle route à déclarer

Dans `team_creation/routes.rs` :
```rust
pub const ROSTER_PLAYERS: &str = "/app/{space_id}/team/{team_id}/roster/{roster_uid}/players";

pub fn roster_players(&self, space_id: &str, team_id: &str, roster_uid: &str) -> String { … }
```

Dans `team_creation/router.rs` :
```rust
.route(path::ROSTER_PLAYERS, get(get_roster_players))
```

---

## Checklist

- [ ] Route `ROSTER_PLAYERS` dans `routes.rs` + `router.rs`
- [ ] Handler `get_roster_players` dans `build_team.rs` (ou fichier dédié)
- [ ] `PlayerPositionVm` + `to_stat_plus()` dans `references/io/web/pickers.rs`
- [ ] Fragment `roster-players-fragment.html` dans `references/io/web/templates/`
- [ ] `BuildTeamTemplate` reçoit `Vec<RosterTierVm>` ; handler charge `DraftTeam` + `AppState`
- [ ] `build-team.html` : `<select>` hardcodé → widget tiers (carte 19) ; `<tbody id="player-table-body">` ; état vide initial
- [ ] Le chip sélectionné reçoit la classe `.active` (JS inline ou `hx-on::after-request`)
- [ ] Réinitialiser les quantités joueurs à 0 lors d'un changement de roster