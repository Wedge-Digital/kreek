# La page d'administration et sa barre d'onglets

**Priorité : haute** — cadre des cartes 369 à 372
**Dépend de :** rien
**Conception :** `docs/specs/space-admin/membres/{02-front.md, 07-integration.md}`
**Maquette :** `assets/rawpages/html/app-space-admin.html`
**Fichiers :** `src/app/spaces/{routes.rs, router.rs}`,
`io/web/controllers/space_admin_controller.rs`, `io/web/templates/space-admin.html`,
`assets/static/css/pages/space-admin.css`, `src/web/css_bundle.rs`

## Objectif

Le cadre : bannière, barre de quatre onglets, zone de contenu. **Aucune
logique** — pas de calcul de VM, pas de JS d'orchestration. C'est le patron
« page d'assemblage à widgets » du `CLAUDE.md`.

Les trois onglets encore vides affichent un état d'attente ; ils seront remplis
par leurs propres cartes.

## La garde

`SpacePermissions::is_admin()`, 403 sinon. Sur la page **et** sur chacun des
endpoints des cartes suivantes : un widget n'hérite d'aucune protection de sa
page hôte, son endpoint étant directement atteignable.

## La réservation de hauteur — le vrai sujet de cette carte

Les onglets sont chargés en différé. Sans réservation, la zone vaut zéro jusqu'à
l'arrivée du fragment et **pousse tout ce qui est en dessous** — le défaut des
cartes 343 et 361.

Le plancher vient d'une règle, pas d'une estimation : **un espace a toujours au
moins un administrateur**, donc la liste a toujours au moins une ligne. La
réservation vaut la barre de statistiques plus une ligne de membre.

`min-height`, jamais `height` — une réservation trop généreuse laisse un blanc
permanent, défaut symétrique et pire, parce qu'il dure. À vérifier sous 768 px
comme au-dessus, la hauteur des deux n'étant pas la même.

## Le badge de visibilité est absent

La maquette porte un badge 🔒 Privé / 🌐 Public. **La visibilité n'existe pas** :
ni colonne, ni écran pour la régler. Elle arrive avec l'onglet Paramètres, qui
ajoutera le badge. Ne pas avancer la colonne pour un ornement.

## Checklist

- [ ] Route `SPACE_ADMIN = "/app/{space_id}/admin"` dans les `Routes` de
      `spaces` — jamais `AppRoutes`, le BC est extractible
- [ ] Contrôleur, garde `is_admin()`, moins de vingt lignes
- [ ] Page enveloppée par `host_layout.wrap_page()`
- [ ] Gabarit : bannière, quatre onglets, zone de contenu. Pas de badge de
      visibilité
- [ ] Bascule d'onglet en Alpine, chargement du widget à la **première**
      activation, sans rechargement ensuite
- [ ] `space-admin.css` inscrite dans `FEUILLES_APP`, section pages — l'axe 14
      de `check-arch` refuse toute feuille orpheline
- [ ] Aucun `<link rel="stylesheet">`, aucun `style="…"` — la maquette en
      contient, ils ne se transcrivent pas
- [ ] Réservation de hauteur posée, vérifiée en desktop **et** sous 768 px
- [ ] `decalages.py` rend **0 px** sur la page, aux deux largeurs
- [ ] URL ajoutée à `tests/e2e/visual/urls.py` et sa classe de portée à
      `CLASSE_ATTENDUE`
- [ ] `make lint`, `make check-arch`, `make test` passent
