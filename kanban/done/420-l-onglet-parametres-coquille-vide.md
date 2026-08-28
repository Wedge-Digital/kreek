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

## Les conteneurs n'ont pas encore leur `hx-get`

La conception les câblait dès cette carte, vers les cinq routes que livreront les
cartes 421 à 425. Chaque ouverture de l'onglet aurait donc émis **cinq requêtes
rendant `404`** — et `request_log` journalise toutes les requêtes, statut
compris. Cinq lignes `status=404` par ouverture, pour un onglet qui n'a rien à
montrer : exactement le bruit que l'épic E11 s'est employée à bannir. Le journal
n'est utile en production que si ce qu'on y lit signifie quelque chose.

**Le rendu à l'écran est identique dans les deux cas** : sur un `404`, htmx
n'échange rien et le conteneur reste vide. La seule différence entre les deux
options est le bruit.

Chaque carte de panneau apporte donc son `hx-get` en même temps que sa route.
Bénéfice second : elles deviennent autonomes, leur ligne de template arrivant
avec leur endpoint.

## Le test unitaire demandé n'est pas réalisable

La carte demandait « Unitaire : l'aiguillage rend la branche `settings` ».
L'aiguillage vit dans `render_admin_page`, qui prend un `&AppState` — et le dépôt
dit lui-même, dans `recap_controller.rs:190`, qu'`AppState` **n'est pas
constructible en test**. C'est la raison d'être de la carte 311.

L'écrire aurait demandé d'inventer un harnais de niveau handler, très au-delà de
cette carte. **L'e2e couvre l'aiguillage**, et le dit plutôt que de laisser une
case cochée sur un test qui n'existe pas.

## Deux scénarios e2e, et ce qu'ils affirment vraiment

`test_l_onglet_parametres_s_ouvre_sur_ses_conteneurs` porte sur **les cinq
conteneurs**, et non sur la seule présence de l'onglet dans la barre. Sans branche
d'aiguillage, le défaut sert le Résumé — l'onglet s'afficherait, répondrait `200`,
et paraîtrait fonctionner. Retirer la branche fait bien tomber ce test, et lui
seul.

`test_l_onglet_parametres_est_garde` vérifie le `GET` du **fragment**, pas
seulement la page complète : c'est par le fragment qu'on navigue, et l'URL est
devinable.

Il a d'abord échoué pour une raison qui vaut d'être notée : `page.request`
réutilise le cookie de session du navigateur, et `bypass_auth` **ne remplace
jamais une identité déjà connectée** — sa docstring le dit. L'en-tête de membre
simple était ignoré, DevCoach répondait, et le test constatait un `200` sans avoir
rien exercé. Il passe par `requests`, sans cookie.

## Checklist

- [ ] Constante de route, `.route(...)`, branche d'aiguillage, entrée d'onglet
- [ ] `settings_tab.rs` avec `require_admin_access`
- [ ] `templates/admin/settings.html`
- [ ] La feuille + son inscription au bundle
- [ ] `make lint && make test && make check-arch`
