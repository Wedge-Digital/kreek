# BC `team_creation` — Widget Player Table autonome

**Priorité : haute**
**Dépend de :** `66-tc-acl-reference-data-port.md`, `67-tc-vm-rapatriement.md`, `68-tc-widget-cart.md`
**Contexte :** BC `team_creation` — widget HTMX

## Objectif

Transformer le tableau de recrutement des joueurs en widget autonome. La widget écoute l'événement `rosterSelected` pour se charger, gère ses propres mutations hire/fire, et émet `teamMutated` pour notifier le cart et le staff.

---

## Situation actuelle

- Le tableau est rendu statiquement dans `build-team.html` (lignes 49–107) au premier chargement
- Quand un roster est sélectionné, le JS appelle `htmx.ajax('GET', .../roster/{uid}/players)` qui retourne `roster-players-fragment.html`
- `roster-players-fragment.html` vit dans le BC `references` (violation)
- Les handlers `hire_player` et `fire_player` retournent `player-row-fragment.html` (aussi dans `references`) + OOB cart + OOB staff
- Le fragment `get_roster_players` retourne aussi le staff en OOB (`staff_table_oob`)

---

## Conception

### Endpoints

La widget a besoin de 3 endpoints :

```
GET  /app/{space_id}/team/{team_id}/widgets/player-table              ← chargement initial / rechargement
POST /app/{space_id}/team/{team_id}/widgets/player-table/hire         ← recruter un joueur
POST /app/{space_id}/team/{team_id}/widgets/player-table/fire         ← renvoyer un joueur
```

Note : les routes `hire_player` et `fire_player` actuelles dans `routes.rs` peuvent être conservées ou migrées vers les nouvelles. À décider à l'implémentation.

### Handler GET (chargement)

```rust
pub async fn player_table_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    Query(params): Query<PlayerTableParams>,   // roster_uid optionnel
    State(state): State<AppState>,
) -> impl IntoResponse {
    // Si pas de roster_uid → état vide
    // Si roster_uid → charger les positions + quantités recrutées
    // Utilise IReferenceDataPort (pas state.references)
}
```

### Handler POST hire/fire

Mêmes handlers qu'aujourd'hui, mais :
1. Retournent uniquement le `<tr>` mis à jour (pas d'OOB cart ni staff)
2. Ajoutent `HX-Trigger: teamMutated` dans la réponse

### Templates

Déplacer les templates depuis `references/io/web/templates/` vers `team_creation/io/web/templates/widgets/` :

- `roster-players-fragment.html` → `widgets/player-table-widget.html`
- `player-row-fragment.html` → `widgets/player-row-fragment.html`

Modifications :
- Supprimer les `{% import "cart-fragment.html" %}` et les blocs OOB cart
- Supprimer le bloc OOB `staff_table_oob` de `roster-players-fragment`
- Ajouter `hx-disinherit="*"` à l'élément racine du widget
- Le widget porte son propre `<link rel="stylesheet">`

### Intégration dans la page hôte `build-team.html`

```html
<div class="section-label">3 — Sélectionnez vos joueurs</div>
<div id="player-table-container"
     hx-get="{{ team_routes.player_table_widget(space_id, team_id) }}"
     hx-trigger="load, rosterSelected from:body"
     hx-vals='js:{"roster_uid": (event?.detail?.uid || "")}'
     hx-target="this"
     hx-swap="innerHTML">
</div>
```

**Important** : sur `rosterSelected`, le `roster_uid` est extrait du `detail` de l'événement et envoyé en query param au handler GET.

### Communication événementielle

| Événement reçu | Action |
|---|---|
| `rosterSelected from:body` | Recharge la widget avec le nouveau `roster_uid` |

| Événement émis | Quand |
|---|---|
| `teamMutated` (via `HX-Trigger` header) | Après chaque hire/fire réussi |

### JS supprimé de `build-team.html`

- Le bloc `onChange` du TomSelect qui faisait `htmx.ajax('GET', .../roster/{uid}/players)` — la coordination est maintenant déclarative via `hx-trigger`
- Le placeholder HTML `<tbody id="player-table-body">` vide dans la page — la widget gère son propre état vide

---

## Situation finale

- Le player table est une widget autonome dans `team_creation/io/web/widgets/player_table_widget.rs`
- Les templates vivent dans `team_creation/io/web/templates/widgets/`
- Les templates de `references/io/web/templates/` (`roster-players-fragment.html`, `player-row-fragment.html`) sont supprimés
- La widget écoute `rosterSelected` et émet `teamMutated`
- Aucun OOB swap dans les réponses hire/fire
- La page hôte ne contient plus de JS pour le player table
- Les handlers utilisent `IReferenceDataPort` (pas `state.references`)

---

## Checklist

- [ ] Ajouter les routes widget dans `team_creation/routes.rs`
- [ ] Créer `team_creation/io/web/widgets/player_table_widget.rs` avec les handlers GET/hire/fire
- [ ] Déplacer `roster-players-fragment.html` → `widgets/player-table-widget.html`
- [ ] Déplacer `player-row-fragment.html` → `widgets/player-row-fragment.html`
- [ ] Modifier les templates : supprimer OOB cart, OOB staff, ajouter `hx-disinherit="*"`
- [ ] Modifier les handlers hire/fire : retourner uniquement le `<tr>` + `HX-Trigger: teamMutated`
- [ ] Supprimer les anciens templates de `references/io/web/templates/`
- [ ] Modifier `build-team.html` : remplacer le tableau statique par `hx-get` + `hx-trigger="load, rosterSelected from:body"`
- [ ] Supprimer le JS `htmx.ajax` du `onChange` TomSelect (déjà supprimé par carte 69)
- [ ] Enregistrer les routes dans le router `team_creation`
- [ ] Les handlers n'importent plus de `references::*`
- [ ] Test E2E : sélection roster → tableau affiché → hire → fire → cart mis à jour
