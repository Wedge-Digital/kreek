# BC `team_creation` — Assemblage final page build-team

**Priorité : haute (dernière carte de la refacto)**
**Dépend de :** `68-tc-widget-cart.md`, `69-tc-widget-roster-picker.md`, `70-tc-widget-player-table.md`, `71-tc-widget-staff-table.md`, `72-tc-widget-league-specialrule-wiring.md`
**Contexte :** BC `team_creation` — page hôte build-team

## Objectif

Finaliser la refactorisation : `build-team.html` devient une page d'assemblage pure qui compose des widgets indépendantes. `build_team.rs` est nettoyé de tout code handler de mutation (déplacé dans les fichiers widget). Valider `check-arch` et les tests E2E.

---

## Situation attendue après les cartes 66–72

À ce stade, les widgets sont créées mais `build_team.rs` contient encore probablement du code résiduel (anciens handlers, anciens imports, templates struct avec des champs obsolètes).

---

## Conception

### `build-team.html` — état final

```html
{% extends "app-layout.html" %}
{% block content %}
<link rel="stylesheet" href="/static/css/pages/team-build.css">

<div class="home-grid">
  <div class="home-main">
    <div class="create-card create-card--ultrawide">
      <div class="create-header">
        <div class="create-header-icon">🏈</div>
        <div class="create-header-text">
          <div class="title">Construire votre équipe</div>
          <div class="sub">Sélectionnez votre roster, vos joueurs et votre staff</div>
        </div>
      </div>

      <div class="create-team-body">

        <!-- Widget Roster Picker -->
        <div class="section-label">2 — Sélectionnez votre roster et votre ligue</div>
        <div class="roster-league-row">
          <div class="roster-league-col">
            <div class="league-selector-label">Roster</div>
            <div hx-get="{{ team_routes.roster_picker_widget(space_id, team_id) }}"
                 hx-trigger="load" hx-target="this" hx-swap="innerHTML"></div>
          </div>
          <div class="roster-league-col">
            <div id="league-selector-zone"
                 hx-get="..." hx-trigger="rosterSelected from:body" ...></div>
          </div>
          <div class="roster-league-col">
            <div id="special-rule-selector-zone"
                 hx-get="..." hx-trigger="rosterSelected from:body" ...></div>
          </div>
        </div>

        <!-- Widget Player Table -->
        <div class="section-label">3 — Sélectionnez vos joueurs</div>
        <div id="player-table-container"
             hx-get="{{ team_routes.player_table_widget(space_id, team_id) }}"
             hx-trigger="load, rosterSelected from:body" hx-target="this" hx-swap="innerHTML"></div>

        <!-- Widget Staff & Rerolls -->
        <div class="section-label">4 — Sélectionnez votre staff</div>
        <div id="staff-table-container"
             hx-get="{{ team_routes.staff_table_widget(space_id, team_id) }}"
             hx-trigger="load, rosterSelected from:body" hx-target="this" hx-swap="innerHTML"></div>

        <div style="display: flex; justify-content: center;">
          <button class="btn btn-outline" hx-get="{{ team_routes.draft_team(space_id) }}"
                  hx-target="#app-content" hx-select="#app-content" hx-swap="innerHTML" hx-push-url="true">
            ← Retour
          </button>
        </div>
      </div>
    </div>
  </div>

  <div class="home-side">
    <!-- Rules Panel (statique) -->
    <div class="rules-panel">...</div>

    <!-- Widget Cart -->
    <div id="team-cart"
         hx-get="{{ team_routes.cart_widget(space_id, team_id) }}"
         hx-trigger="load, teamMutated from:body" hx-target="this" hx-swap="innerHTML"></div>

    <div class="cart-actions">
      <a class="btn btn-primary" hx-get="{{ team_routes.finalize_team(space_id, team_id) }}"
         hx-target="#app-content" hx-select="#app-content" hx-swap="innerHTML" hx-push-url="true">
        Terminer la construction →
      </a>
    </div>
  </div>
</div>

<!-- ZÉRO <script> -->
{% endblock %}
```

### Points clés

- **Zéro JavaScript** dans la page hôte
- **Zéro macro Askama** importée (`cart_cmp`, `staff_cmp`, `roster_cmp` tous supprimés)
- La page ne contient que du HTML structurel + des `hx-get` / `hx-trigger`
- Chaque widget se charge indépendamment au `load` et réagit aux événements DOM

### `build_team.rs` — état final

Le fichier ne contient plus que :
- `BuildTeamTemplate` avec uniquement les champs nécessaires à la page hôte (routes, space_id, team_id, rules_panel)
- Le handler `build_team()` qui charge les données minimales (rules panel, competition name/season)
- Le handler `submit_team()` (qui reste dans build_team ou est déplacé dans un fichier dédié)

Les handlers de mutation et fragments sont dans :
- `widgets/cart_widget.rs`
- `widgets/roster_picker_widget.rs`
- `widgets/player_table_widget.rs`
- `widgets/staff_table_widget.rs`

### `BuildTeamTemplate` — état final

```rust
pub struct BuildTeamTemplate {
    pub web_routes: WebRoutes,
    pub team_routes: TeamCreationRoutes,
    pub space_id: String,
    pub team_id: String,
    pub rules_panel: RulesPanelVm,
}
```

Champs supprimés : `rosters`, `selected_roster_uid`, `league_selector_url`, `special_rule_selector_url`, `hired_rows`, `staff_rows`, `reroll`, `cart`, `ref_routes`.

### Structure fichiers — état final

```
src/app/team_creation/io/web/
├── mod.rs
├── build_team.rs              ← page hôte (handler build_team + submit_team)
├── post_draft_team.rs         ← inchangé
├── finalize_team.rs           ← inchangé (refacto séparée)
├── view_models.rs             ← tous les VMs (carte 67)
├── builders.rs                ← tous les builders (carte 67)
├── reference_data_adapter.rs  ← implémentation du port ACL (carte 66)
├── widgets/
│   ├── mod.rs
│   ├── cart_widget.rs
│   ├── roster_picker_widget.rs
│   ├── player_table_widget.rs
│   └── staff_table_widget.rs
└── templates/
    ├── build-team.html        ← page hôte (assemblage pur)
    ├── draft-team.html
    ├── finalize-team.html
    └── widgets/
        ├── cart-widget.html
        ├── roster-picker-widget.html
        ├── player-table-widget.html
        ├── player-row-fragment.html
        ├── staff-table-widget.html
        ├── staff-row-fragment.html
        └── reroll-row-fragment.html
```

### Fichiers supprimés

- `team_creation/io/web/templates/cart-fragment.html` (macro → widget)
- `team_creation/io/web/templates/staff-fragment.html` (macro → widget)
- `team_creation/io/web/templates/widgets/roster-picker-tiers.html` (macro → widget)
- `references/io/web/templates/roster-players-fragment.html` (déplacé)
- `references/io/web/templates/player-row-fragment.html` (déplacé)
- `references/io/web/templates/staff-row-fragment.html` (déplacé)
- `references/io/web/templates/reroll-row-fragment.html` (déplacé)

### Flux événementiel — état final

```
┌──────────────┐   rosterSelected    ┌──────────────────┐
│ Roster Picker │ ──────────────────► │ Player Table      │
│   (widget)    │                     │   (widget)        │
└──────────────┘                     └───────┬──────────┘
       │                                      │ teamMutated
       │ rosterSelected                       ▼
       ▼                              ┌──────────────────┐
┌──────────────┐                      │ Cart              │
│ League Sel.  │                      │   (widget)        │
│ (BC ref.)    │                      └──────────────────┘
└──────────────┘                              ▲
       │                                      │ teamMutated
       │ rosterSelected                       │
       ▼                              ┌──────────────────┐
┌──────────────┐                      │ Staff & Rerolls   │
│ Special Rule │                      │   (widget)        │
│ (BC ref.)    │                      └──────────────────┘
└──────────────┘
```

---

## Validation finale

### `check-arch`
- Aucune violation Axe 3 pour le BC `team_creation` (plus d'import `references::*`)
- Les seuls cross-BC autorisés sont les `hx-get` vers les endpoints du BC `references` (league/special-rule selectors) dans le HTML

### Tests E2E — scénario complet
1. Accéder à la page build-team
2. Sélectionner un roster → player table + staff table + league/special-rule se chargent
3. Recruter un joueur → row mise à jour, cart rechargé
4. Recruter un staff → row mise à jour, cart rechargé
5. Acheter une relance → row mise à jour, cart rechargé
6. Sélectionner une ligue → sélecteur rechargé avec sélection
7. Changer de roster → tout se recharge proprement
8. Finaliser → redirection vers finalize-team

### Compilation
- `cargo check` — 0 erreur
- Plus aucun warning relatif aux imports `references::*` dans `team_creation`

---

## Checklist

- [ ] Nettoyer `BuildTeamTemplate` (supprimer les champs obsolètes)
- [ ] Nettoyer le handler `build_team()` (ne charge plus que rules_panel)
- [ ] Déplacer `submit_team()` dans un fichier dédié si pertinent
- [ ] Supprimer les anciens handlers de mutation de `build_team.rs` (hire/fire/buy/remove/roster_players)
- [ ] Supprimer les anciens template structs de `build_team.rs` (fragments)
- [ ] Supprimer les fichiers templates obsolètes (cart-fragment, staff-fragment, roster-picker-tiers)
- [ ] Supprimer les templates du BC references qui ont été déplacés
- [ ] Vérifier que `build-team.html` ne contient aucun `<script>`
- [ ] Vérifier que `build-team.html` n'importe aucune macro
- [ ] `check-arch` — aucune violation
- [ ] Test E2E complet du parcours build-team
- [ ] `cargo check` — 0 erreur, pas de warning d'import inutilisé
