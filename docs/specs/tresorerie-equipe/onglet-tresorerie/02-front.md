# Onglet Trésorerie · Phase 2 : architecture front

**Maquette** : `assets/rawpages/html/app-team-treasury.html`

## Ce n'est pas une page à widgets, et c'est délibéré

Le `CLAUDE.md` réserve le patron d'assemblage aux pages de **trois sections
interactives et plus**. L'onglet Trésorerie en a **zéro** : c'est une vue en
lecture seule, sans mutation, sans filtre, sans tri.

Découper le bandeau de synthèse et le relevé en deux widgets donnerait deux
requêtes là où une suffit, deux endpoints à garder cohérents, et un instant où
l'écran montre un solde sans les mouvements qui l'expliquent. **Un seul
fragment, une seule requête.**

C'est le même choix que l'onglet Poules de l'administration de compétition fait
à l'inverse : lui compose deux conteneurs parce qu'il a deux sections qui
mutent indépendamment. Ici, rien ne mute.

## Le vrai travail est ailleurs : les onglets n'existent pas

```html
<!-- teams-team-detail.html:145 -->
<div class="tabs team-tabs">
  <div class="tab active">Joueurs &amp; Staff</div>
  <div class="tab">Matchs</div>
  <div class="tab">Trésorerie</div>
</div>
```

**Trois `<div>` inertes.** Aucun `hx-get`, aucun lien, aucune route. « Matchs »
et « Trésorerie » ne mènent nulle part et n'ont jamais rien fait. La fiche
équipe n'a qu'une route, `TEAM_DETAIL`, et tout ce qui suit les onglets — le
widget joueurs et le panneau staff — est le contenu de « Joueurs & Staff » sans
que rien ne le dise.

Livrer la trésorerie impose donc d'abord de **faire exister le mécanisme
d'onglets**, ce que la fonctionnalité n'annonçait pas.

### Le patron maison

L'administration de compétition le pose déjà (`admin-page.html:19`) :

```html
<a class="tab{% if active_tab == "treasury" %} active{% endif %}"
   hx-get="{{ routes.team_treasury(space_id, team_id) }}"
   hx-target="#team-tab-content"
   hx-swap="innerHTML"
   hx-push-url="{{ routes.team_treasury(space_id, team_id) }}">Trésorerie</a>
```

Trois propriétés qu'on ne réinvente pas : le contenu seul est remplacé, l'URL
suit donc l'onglet est partageable et rechargeable, et le serveur aiguille sur
`active_tab` comme `admin_page.rs` le fait déjà.

**`innerHTML` sur `#team-tab-content` est correct ici** : le fragment est le
contenu du conteneur, pas le conteneur lui-même. C'est précisément la forme que
le `CLAUDE.md` exige — l'erreur qu'il proscrit est le fragment qui **répète
l'`id`** de sa cible.

### Ce que ça impose au template existant

Tout ce qui suit les onglets — `#players-widget` et `.staff-panel` — entre dans
un `#team-tab-content`, et devient le fragment de l'onglet « Joueurs & Staff ».
C'est un déplacement de balises, pas une réécriture : **copier-coller**, règle 5
du `CLAUDE.md`.

L'onglet « Matchs » reste inerte. Il n'est pas dans le périmètre, et lui donner
une route vide serait pire que de le laisser tel quel — un onglet qui répond
« rien » se lit comme une panne.

## Le fragment de trésorerie

| Élément | Endpoint | Trigger | Émet | Mode |
|---|---|---|---|---|
| `#team-tab-content` (contenu Trésorerie) | `team_treasury` | clic sur l'onglet, ou chargement direct de l'URL | — | **lecture seule** |

**Aucun événement DOM, ni émis ni écouté.** Rien sur cette page ne réagit à
quoi que ce soit : il n'y a ni mutation, ni sélection, ni second widget à
prévenir.

**Aucun `hx-disinherit`** n'est nécessaire : la règle vise les widgets chargés
dans une page hôte porteuse d'attributs HTMX. Ici le fragment *est* le contenu
de l'onglet, et la fiche équipe ne pose aucun `hx-vals` ni `hx-include` sur son
conteneur. Poser l'attribut par réflexe donnerait une protection contre un
danger qui n'existe pas.

## Ce qui reste front

**Rien.** Pas de tri, pas de filtre, pas de repli de section, pas d'Alpine.

Le sélecteur d'états de la maquette est un outil de démonstration ; il ne
franchit pas la ligne. Les deux états qu'il bascule — équipe en cours de saison
et équipe qui vient d'être créée — sont **deux rendus du même gabarit** selon
qu'il y a ou non des mouvements après la dotation, décidés côté serveur.

C'est le seul conditionnel du gabarit :

```
si le relevé n'a que la ligne de dotation
    → bandeau de synthèse + bloc « Aucun mouvement pour l'instant »
sinon
    → bandeau de synthèse + tableau
```

## Pagination : non, et pourquoi

Mesuré le 2026-08-26 : le plus gros grand livre porte **6 lignes**, la moyenne
2, le 95ᵉ centile 4. Ces chiffres viennent de données de démonstration et
sous-estiment une vraie saison — mais l'ordre de grandeur tient au calcul : une
saison de quinze journées produit au plus une recette, un achat de coups de
pouce et quelques recrutements par journée, soit **quelques dizaines de lignes**.

Une pagination coûterait un curseur, un état d'écran et un second aller-retour
pour un tableau qui tient dans un scroll. **Si un grand livre atteignait
plusieurs centaines de lignes, c'est le tri anti-chronologique qu'il faudrait
d'abord**, pas la pagination — et ce serait un autre écran.

## Les trois libellés que le serveur doit fabriquer

Le gabarit n'interprète rien ; il affiche ce que le view model lui donne. Trois
choses s'y résolvent donc côté serveur, et sont notées ici parce qu'elles
décident du contenu de la phase 3 :

1. **Le libellé du motif** — `PlayerRecruitment` → « Recrutement ». Huit motifs,
   une table de correspondance.
2. **La ligne de détail** — « Gwenn, Passeuse — n° 7 », « Victoire », « Sorcier ».
   Trois sources, dont deux déjà disponibles dans `teams` (phase 1).
3. **Le regroupement par journée** — « Journée 1 — contre les Trolls du Bief »,
   qui vient du port vers `match_report`.

## Responsivité

Desktop-first, breakpoint unique à 768 px, conformément au `CLAUDE.md`.

Sous 768 px, **la colonne « Solde après » disparaît avant les montants** : le
solde courant reste lisible dans le bandeau de synthèse, alors que le montant
d'un mouvement n'a aucun autre endroit où se lire. Le bandeau passe en colonne,
l'équation se replie.

C'est la seule règle responsive propre à cet écran ; le reste — barre de menu,
sidebar, tabbar mobile — est tenu par `app-layout.html`, et cette page n'a pas
à s'en occuper.

## CSS

Une feuille, `pages/team-treasury.css`, portée par `.treasury`, **inscrite dans
`src/web/css_bundle.rs`** parmi les pages — l'axe 14 de `check-arch` refuse
toute feuille absente du bundle.

Elle ne reprend rien d'existant : l'en-tête d'équipe et les onglets vivent déjà
dans `pages/team-page.css`, et le relevé n'a aucun équivalent ailleurs.

## Règles métier

**Aucune à préciser à cette étape** — confirmé le 2026-08-26. La vue est en
lecture seule et n'ajoute aucune règle : elle donne à voir des mouvements que
d'autres fonctionnalités ont produits.

Deux réserves de la phase 1 restent portées jusqu'à la phase 3 :

- **Un joueur renvoyé perd son nom.** `ISquadPort` rend l'effectif courant, pas
  l'historique. La ligne se replie alors sur le poste, qui vient de l'événement.
  C'est un repli assumé, pas une donnée à retrouver.
- **La dotation de départ n'affiche pas son tier.** `TeamCreated` ne le porte
  pas, et le tier appartient à `competitions` — un troisième port pour une seule
  ligne ne se justifie pas.
