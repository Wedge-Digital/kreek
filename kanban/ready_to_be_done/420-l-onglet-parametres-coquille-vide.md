# L'onglet Paramètres, coquille vide

**Épic :** E14 · **Ordre :** 2 · **Dépend de :** 419
**Conception :** `docs/specs/modifier-une-competition/onglet-parametres/02-front.md`
et `07-integration.md`

## Objectif

Ouvrir l'onglet et sa place dans l'aiguillage, avec ses cinq conteneurs vides.
Les panneaux viennent ensuite, un par carte — celle-ci les attend sans rien
promettre à l'écran.

## Conception

### La route et l'aiguillage

```rust
COMPETITION_ADMIN_SETTINGS  "/app/{space_id}/competitions/{competition_id}/{season_id}/admin/settings"
```

Montée en `get(settings_tab)`. Branche `"settings"` ajoutée au `match active_tab`
de `admin_page.rs`, entrée d'onglet dans `admin-page.html` — libellé
« ⚙️ Paramètres », dernière position.

### Le contrôleur

```rust
// io/web/admin/settings/settings_tab.rs
pub async fn settings_tab(
    auth_session: AuthSession,
    Path((space_id, competition_id, season_id)): Path<(String, String, String)>,
    State(state): State<AppState>,
) -> Response;
```

**`require_admin_access` en première ligne**, y compris sur ce GET : sans
contrôle sur le chemin htmx, le changement d'onglet contourne l'autorisation
(`admin_page.rs:57`). Le prédicat est admin d'espace, ou admin de compétition
par identifiant **ou** par nom de coach.

### Le template d'assemblage

`templates/admin/settings.html` — cinq conteneurs, aucun calcul, aucun JS :

```html
<div class="competition-admin-settings">
  <div id="settings-general"    hx-get="…/settings/general"    hx-trigger="load"></div>
  <div id="settings-ranking"    hx-get="…/settings/ranking"    hx-trigger="load"></div>
  <div id="settings-pools"      hx-get="…/settings/pools"      hx-trigger="load"></div>
  <div id="settings-tiers"      hx-get="…/settings/tiers"      hx-trigger="load"></div>
  <div id="settings-visibility" hx-get="…/settings/visibility" hx-trigger="load"></div>
</div>
```

Tant que les cinq routes n'existent pas, les conteneurs restent vides — l'onglet
s'ouvre, ne montre rien, et ne casse rien. C'est l'état attendu à la fin de
cette carte.

### La feuille

`assets/static/css/pages/competition-admin-settings.css`, portée par
`.competition-admin-settings`, **inscrite dans `src/web/css_bundle.rs`** parmi
les pages. L'axe 14 la refuse sinon.

Elle porte pour l'instant la mise en page des panneaux — celle de la maquette
`app-competition-admin-modification.html`, section `.settings-panel*`.

**Deux composants ne sont pas réécrits** : les puces de coups de pouce
reprennent `widgets/inducement-grid.css`, les blocs de tier
`pages/new-competition-phase-2.css`.

## Tests

- Unitaire : l'aiguillage rend la branche `settings`.
- E2E : l'onglet apparaît, s'ouvre, et un non-admin reçoit `403` sur son GET.

## Checklist

- [ ] Constante de route, `.route(...)`, branche d'aiguillage, entrée d'onglet
- [ ] `settings_tab.rs` avec `require_admin_access`
- [ ] `templates/admin/settings.html`
- [ ] La feuille + son inscription au bundle
- [ ] `make lint && make test && make check-arch`
