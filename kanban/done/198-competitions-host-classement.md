# BC `competitions` — Onglet Classement devient un simple host du widget `ranking`

**Priorité : haute**
**Dépend de :** `197-ranking-widget-classement.md`
**Contexte :** `competitions/io/web/competition_detail.rs`, `competitions/templates/competition-tab-standings.html`
**Spec :** `docs/specs/ranking/classement/02-front.md`, `07-integration.md`

## Objectif

`competitions` cède la responsabilité d'affichage du classement à `ranking` — elle garde uniquement la route de deep-link et le shell de page.

## Conception

`competition-tab-standings.html` remplacé par le wrapper `hx-get` :

```html
<div id="ranking-widget"
     hx-get="{{ app_routes.ranking.classement_widget(space_id, competition_id, season_id) }}"
     hx-trigger="load"
     hx-target="this"
     hx-swap="outerHTML">
  <div class="loading-placeholder">Chargement du classement…</div>
</div>
```

`get_tab_standings` (dans `competition_detail.rs`) simplifié : ne calcule plus rien, rend juste ce wrapper (fragment si `hx-request`, sinon via `full_page(...)`).

Suppression exhaustive (vérifier tous les appelants avant, règle CLAUDE.md §4) :
- `fn mock_standings()` 
- `struct StandingRow`

Vérifier que `AppRoutes` expose bien `.ranking.classement_widget(...)` (le module `routes.rs` de `ranking` doit déjà être agrégé dans `AppRoutes` — si ce n'est pas encore le cas, l'ajouter ici).

## Checklist

- [ ] `competition-tab-standings.html` remplacé par le wrapper `hx-get`
- [ ] `get_tab_standings` simplifié (plus aucun calcul de classement)
- [ ] `mock_standings()` et `StandingRow` supprimés — vérification exhaustive qu'ils ne sont plus référencés ailleurs avant suppression
- [ ] `AppRoutes` expose `.ranking.classement_widget(...)`
- [ ] Referme la partie "Classement" de la carte `13-mock-data-competition-detail.md` (mettre à jour cette carte ou la clore si elle ne couvre plus que Résultats/Calendrier déjà traités par ailleurs)
- [ ] `cargo check` passe
- [ ] `make check-arch` propre
- [ ] Vérification manuelle : la page détail compétition charge bien le classement au chargement (onglet actif par défaut)
