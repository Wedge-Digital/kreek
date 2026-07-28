# Chrome web — spaces ne dépend plus du layout ni de la couche `web` du host

**Priorité : moyenne**
**Dépend de :** 242 (chapeau) ; plus simple après 245 (sous-états)
**Fichiers :** `src/app/spaces/io/web/templates/space-all.html`, `new-space.html`,
`pages/spaces-widget-tester-page.html`, `src/web/app_spaces.rs`,
`src/web/extractors/space_permissions.rs`, `src/web/router.rs`, `askama.toml`

## Problème

### 1. Les pages de spaces étendent le chrome de kreek

```
space-all.html:1  → {% extends "app-layout.html" %}
new-space.html:1  → {% extends "app-layout.html" %}
pages/spaces-widget-tester-page.html:1 → {% extends "widget-tester-layout.html" %}
```

Ces trois layouts vivent dans `src/web/templates/`. `app-layout.html` est le
chrome Kreek : titre, CDN Alpine/TomSelect/Cloudinary, `kreek-select`, sidebar
et menu chargés en HTMX. La résolution ne saute pas aux yeux parce que
`askama.toml` déclare les onze dossiers de templates dans un seul espace de
noms — un template de spaces peut étendre n'importe quoi sans que rien ne le
signale.

`auth` fait déjà bien : il embarque son propre `auth-layout.html` et l'applique
lui-même quand la requête n'est pas une requête HTMX
(`get_login_success.rs` : test de l'en-tête `hx-request`, sinon enveloppe dans
`AuthLayout`). C'est le modèle à suivre.

### 2. Deux morceaux de spaces vivent chez le host

- **`src/web/app_spaces.rs`** — la liste des espaces de la sidebar. Importe
  `spaces::domain::space_repository_port::SpaceSummary` et interroge
  `state.spaces.space_repository`. C'est un widget du BC spaces, rangé dans la
  couche web de l'application.
- **`src/web/extractors/space_permissions.rs`** — l'extracteur `SpacePermissions`
  (403 si l'utilisateur n'est pas membre de l'espace, `is_admin()`,
  `can_report_match()`). Il combine `AuthSession` et
  `spaces.space_repository.find_member_profile()`.

Le second est probablement **la pièce la plus réutilisable de tout le lot** :
un extracteur axum d'autorisation par espace, prêt à l'emploi. Le laisser chez
le host, c'est extraire les deux BCs en oubliant leur meilleur morceau.

## Action

### Décision à prendre : l'enveloppe des pages

Askama résout `extends` statiquement — un BC ne peut pas recevoir son layout en
paramètre. Deux options :

- **A (recommandée, alignée sur auth)** — les templates du BC ne produisent que
  des fragments. Le controller enveloppe lui-même quand la requête n'est pas
  HTMX, via une enveloppe **injectée par le host** (fonction ou trait dans
  `SpacesContext`). Kreek injecte `app-layout.html`, un autre projet injecte
  le sien.
- **B** — spaces embarque un layout minimal à lui, comme `auth-layout.html`.
  Plus simple, mais kreek perd sa sidebar et son menu sur ces deux pages :
  inacceptable en l'état.

Retenir A sauf découverte contraire à l'implémentation.

La page de test des widgets (`spaces-widget-tester-page.html`) est un outil de
dev : la laisser côté host est acceptable, elle ne fait pas partie du paquet
extrait. Le noter plutôt que de la traiter.

### Rapatriements

- `src/web/app_spaces.rs` → `src/app/spaces/io/web/` (widget du BC, avec sa
  route dans `spaces::routes`)
- `src/web/extractors/space_permissions.rs` → `src/app/spaces/io/web/extractors/`

Après la carte 245, l'extracteur se projette sur `SpacesContext` plutôt que sur
`AppState`, ce qui le rend réellement portable. Attention : il est utilisé par
les routes paramétrées `{space_id}` de plusieurs BCs — vérifier tous les
appelants avant de déplacer (règle CLAUDE.md n°4).

## Checklist

- [ ] Option A ou B tranchée et appliquée
- [ ] Aucun template de `src/app/spaces/` n'étend un template de `src/web/templates/`
- [ ] `app_spaces.rs` déplacé dans le BC spaces, route déclarée dans `spaces::routes`
- [ ] `space_permissions.rs` déplacé dans le BC spaces, tous les appelants mis à jour
- [ ] `grep -rn "crate::web::" src/app/auth src/app/spaces` ne remonte rien
- [ ] Vérification manuelle dans le navigateur : sidebar, menu et les deux pages
      d'espaces s'affichent comme avant, en navigation HTMX **et** en F5
- [ ] Test E2E de création et de sélection d'espace au vert
- [ ] `make check-arch` au vert, `make test` au vert
