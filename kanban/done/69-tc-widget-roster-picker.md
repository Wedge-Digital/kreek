# BC `team_creation` — Widget Roster Picker autonome

**Priorité : haute**
**Dépend de :** `66-tc-acl-reference-data-port.md`, `67-tc-vm-rapatriement.md`
**Contexte :** BC `team_creation` — widget HTMX

## Objectif

Transformer le sélecteur de roster en widget autonome avec son propre endpoint, sur le modèle du coach-selector. La widget embarque TomSelect, les données de tiers, et émet un événement DOM `rosterSelected` à la sélection. La page hôte ne porte plus de JS pour gérer TomSelect.

---

## Situation actuelle

- Le roster picker est une macro Askama `roster_picker()` dans `widgets/roster-picker-tiers.html` qui rend un `<select>` brut
- L'initialisation TomSelect est faite dans un bloc `<script>` de ~40 lignes dans `build-team.html` (IIFE lignes 282–318)
- Le JS gère : le rendu custom avec badges de tier, le `onChange` qui déclenche `htmx.ajax()` + `reloadLeagueSelector()` + `reloadSpecialRuleSelector()`, et le pré-remplissage si `selected_roster_uid` existe
- Les données de tier (`tier_name`, `tier_index`) sont passées en `data-attributes` sur les `<option>`

---

## Conception

### Nouvel endpoint

```
GET /app/{space_id}/team/{team_id}/widgets/roster-picker
```

### Handler

```rust
pub async fn roster_picker_widget(
    Path((space_id, team_id)): Path<(String, String)>,
    State(state): State<AppState>,
) -> impl IntoResponse {
    let draft = state.team_creation.team_repository
        .find_by_id(&TeamId::try_new(&team_id).unwrap())
        .await;

    let ref_data = state.team_creation.reference_data.as_ref();

    let rosters = build_roster_items_with_tiers(ref_data, draft.creation_rules());

    // Déterminer la sélection courante si un roster est déjà choisi
    let selected = state.team_creation.roster_repository
        .find_by_id(&TeamId::try_new(&team_id).unwrap())
        .await
        .ok()
        .flatten()
        .map(|t| t.roster.id.0.clone());

    RosterPickerWidgetTemplate { rosters, selected }.into_response()
}
```

### Template : `widgets/roster-picker-widget.html`

Pattern identique au coach-selector : Alpine `x-data` avec `init()` / `destroy()` pour le lifecycle TomSelect.

```html
<link rel="stylesheet" href="/static/css/widgets/roster-picker.css">
<div class="roster-picker-widget" hx-disinherit="*"
     x-data="{
    init() {
        this._ts = new TomSelect(this.$refs.select, {
            allowEmptyOption: true,
            maxItems: 1,
            render: {
                option: (d) => {
                    const badge = d.tierName
                        ? '<span class=\'roster-tier-badge tier-' + d.tierIndex + '\'>' + d.tierName + '</span>'
                        : '';
                    return '<div class=\'roster-option\'><span>' + d.text + '</span>' + badge + '</div>';
                },
                item: (d) => {
                    const badge = d.tierName
                        ? '<span class=\'roster-tier-badge tier-' + d.tierIndex + '\'>' + d.tierName + '</span>'
                        : '';
                    return '<div class=\'roster-option\'><span>' + d.text + '</span>' + badge + '</div>';
                },
            },
            onChange: (value) => {
                htmx.trigger(document.body, 'rosterSelected', {
                    uid: value,
                    name: this._ts.options[value]?.text ?? ''
                });
            },
        });
        {% if let Some(uid) = selected %}
        this._ts.setValue('{{ uid }}', true);
        {% endif %}
    },
    destroy() {
        if (this._ts) this._ts.destroy();
    }
}">
    <select x-ref="select">
        <option value="">Choisissez un roster…</option>
        {% for item in rosters %}
        <option value="{{ item.uid }}"
                data-tier-index="{{ item.tier_index }}"
                data-tier-name="{{ item.tier_name }}"
                data-reroll-cost="{{ item.reroll_cost }}">{{ item.name }}</option>
        {% endfor %}
    </select>
</div>
```

### Événement émis

```js
// payload de rosterSelected
{ uid: "LIZARDMEN", name: "Hommes-Lézards" }
```

### Intégration dans la page hôte `build-team.html`

```html
<div class="roster-league-col">
  <div class="league-selector-label">Roster</div>
  <div hx-get="{{ team_routes.roster_picker_widget(space_id, team_id) }}"
       hx-trigger="load"
       hx-target="this"
       hx-swap="innerHTML">
  </div>
</div>
```

### JS supprimé de `build-team.html`

- La variable `CLASSES` et la fonction `badge()`
- Le bloc TomSelect `new TomSelect(el, { ... })` (~40 lignes)
- Le pré-remplissage `ts.setValue()`
- La macro `roster_picker()` dans `widgets/roster-picker-tiers.html` (fichier entier supprimable)

---

## Situation finale

- Le roster picker est une widget autonome avec endpoint dédié
- TomSelect est initialisé/détruit via Alpine lifecycle (pas de `<script>` nu)
- La widget émet `rosterSelected` sur `body` — les autres widgets s'abonnent indépendamment
- La page hôte n'a plus aucun JS relatif au roster picker
- Le fichier `widgets/roster-picker-tiers.html` (macro) est supprimé
- La route est déclarée dans `team_creation/routes.rs`

---

## Checklist

- [ ] Ajouter `ROSTER_PICKER_WIDGET` dans `team_creation/routes.rs` (path + méthode)
- [ ] Créer le handler `roster_picker_widget` dans `team_creation/io/web/widgets/roster_picker_widget.rs`
- [ ] Créer le template `widgets/roster-picker-widget.html`
- [ ] Enregistrer la route dans le router `team_creation`
- [ ] Modifier `build-team.html` : remplacer `{% call roster_cmp::roster_picker(rosters) %}` par `hx-get` + `hx-trigger="load"`
- [ ] Supprimer le JS TomSelect de `build-team.html` (IIFE lignes 275–318)
- [ ] Supprimer le fichier `widgets/roster-picker-tiers.html`
- [ ] Supprimer `rosters` et `selected_roster_uid` de `BuildTeamTemplate`
- [ ] CSS : créer ou adapter `/static/css/widgets/roster-picker.css` si nécessaire
- [ ] Test E2E : vérifier que la sélection de roster émet bien l'événement et déclenche le chargement des joueurs
