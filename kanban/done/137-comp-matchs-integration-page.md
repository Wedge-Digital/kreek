# 137 — Intégration : mise à jour competition_detail + routes

## Objectif

Câbler les deux nouveaux onglets dans la page de détail de compétition : remplacer l'onglet "Matchs" par "Résultats" et "Calendrier", ajouter les routes, supprimer le code obsolète.

## Dépendances

- 135 (handler Résultats)
- 136 (handler Calendrier)

## Conception détaillée

### `io/web/competition_detail.rs`

- Supprimer `get_tab_matches` et `MatchesTabTemplate`
- Ajouter les deux nouveaux handlers comme fallback full-page (navigation directe sans `HX-Request`)
- Le `CompetitionDetailTemplate` n'a plus besoin du champ `journees: Vec<Journee>`

### `templates/competition-tab-matches.html`

- Supprimer ce fichier (remplacé par `competition-tab-resultats.html` et `competition-tab-calendrier.html`)

### `templates/competition-detail.html`

Remplacer l'onglet Matchs par :

```html
<!-- Onglet Résultats -->
<div class="tab {% if active_tab == "resultats" %}active{% endif %}"
     id="tab-btn-resultats"
     hx-get="{{ app_routes.competitions.resultats_tab(space_id, competition_id, season_id) }}"
     hx-target="#resultats-list"
     hx-swap="innerHTML"
     hx-trigger="click once">
  Résultats
</div>

<!-- Onglet Calendrier -->
<div class="tab {% if active_tab == "calendrier" %}active{% endif %}"
     id="tab-btn-calendrier"
     hx-get="{{ app_routes.competitions.calendrier_tab(space_id, competition_id, season_id) }}"
     hx-target="#calendrier-list"
     hx-swap="innerHTML"
     hx-trigger="click once">
  Calendrier
</div>
```

Ajouter les conteneurs dans la zone de contenu des onglets :

```html
<div id="tab-resultats" class="tab-content" style="display:none;">
  <div id="resultats-list"></div>
</div>

<div id="tab-calendrier" class="tab-content" style="display:none;">
  <div id="calendrier-list"></div>
</div>
```

### `router.rs` — 2 nouvelles routes

```rust
.route(
    "/spaces/:space_id/competitions/:competition_id/seasons/:season_id/resultats",
    get(resultats_tab_controller::get_resultats_tab),
)
.route(
    "/spaces/:space_id/competitions/:competition_id/seasons/:season_id/calendrier",
    get(calendrier_tab_controller::get_calendrier_tab),
)
```

### `routes.rs` — AppRoutes

Ajouter les méthodes `resultats_tab()` et `calendrier_tab()` dans la struct de routes competitions.

### `competition-detail.css`

Ajouter les styles `.cal-*` (calendrier) et vérifier que les styles `.match-*` de la maquette sont présents. Se référer au `<style>` de la rawpage `app-competition-detail.html`.

## Checklist

- [ ] `get_tab_matches` et `MatchesTabTemplate` supprimés
- [ ] `competition-tab-matches.html` supprimé
- [ ] 2 nouvelles routes ajoutées dans `router.rs`
- [ ] Méthodes `resultats_tab()` et `calendrier_tab()` dans `routes.rs`
- [ ] `competition-detail.html` mis à jour (5 onglets, 2 conteneurs lazy)
- [ ] CSS `.cal-*` ajouté à `competition-detail.css`
- [ ] `cargo build` passe
