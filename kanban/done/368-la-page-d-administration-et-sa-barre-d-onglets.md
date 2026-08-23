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

- [x] Route `SPACE_ADMIN` dans les `Routes` de `spaces` — jamais `AppRoutes`
- [x] Contrôleur, garde `is_admin()`, moins de vingt lignes
- [x] Page enveloppée par `host_layout.wrap_page()`, via `render_page`
- [x] Gabarit : bannière, quatre onglets, zone de contenu, **pas de badge de
      visibilité**
- [x] Bascule d'onglet en Alpine, contenu monté à la **première** activation et
      conservé ensuite — rebasculer ne redéclenche rien et n'efface pas la saisie
- [x] ~~Racine `.admin-container`~~ → **`.space-admin`**. La classe de la
      maquette appartient déjà à la page d'administration de compétition, avec
      une autre largeur ; depuis le bundle unique, deux pages qui partagent une
      classe partagent son style
- [x] `pages/space-admin.css` inscrite dans `FEUILLES_APP`, section pages
- [x] Aucun `<link rel="stylesheet">`, aucun `style="…"`
- [x] Réservation de hauteur posée, en `min-height`, ajustée sous 768 px, avec
      sa composition auditable en commentaire
- [ ] ~~`decalages.py` rend 0 px~~ — **reporté à la carte 369**. Sans chargement
      différé, la mesure rend zéro sans rien prouver : il n'y a encore rien à
      charger. La réservation est posée, sa valeur reste à confirmer
- [x] URL et classe de portée ajoutées au harnais visuel
- [x] **Ajouté hors carte** : l'entrée de menu « Espace », qui existait et était
      inerte, est câblée et n'apparaît qu'aux administrateurs de l'espace ou au
      compte d'exploitation. Deux tests au harnais
- [x] `make lint`, `make check-arch`, `make test` passent — 1124 tests

## Ce qu'on a appris en la faisant

**La maquette réutilisait une classe déjà prise.** `.admin-container` appartient
à la page d'administration de compétition, en 1060 px là où la maquette veut
960. Transcrire les classes littéralement aurait fait exactement ce que la carte
342 documente : le bundle **active** les divergences que les feuilles isolées
cachaient.

**Un token de la maquette n'existe pas.** `--light-1` n'est pas défini dans
`common.css` — remplacé par `--dark-6`, et les neuf autres vérifiés au passage.
Le bundle aurait accepté la règle sans broncher ; elle n'aurait rien fait.

**Une assertion de test trop lâche s'est fait prendre par son propre pendant.**
Vérifier la présence du mot « Espace » passait pour l'administrateur et échouait
pour le membre simple : le nom de l'espace semé est « Espace E2E ». L'assertion
porte désormais sur l'URL d'administration.

**Un contrôle qui n'est exécuté par personne, trouvé en chemin.**
`tests/e2e/visual/debordements.py` — le contrôle C de la carte 342 — n'est lancé
ni par le `Makefile`, ni par la CI, ni par `check-arch`. Il échoue aujourd'hui
sur **sept feuilles et 167 correspondances**, à l'identique avec ou sans cette
page : le débordement est entièrement préexistant. Même famille que la carte
363, qui avait branché le contrôle de portée mais laissé celui-ci orphelin.
