# Construction d'équipe — la plus grosse zone non réservée du dépôt

**Priorité : moyenne** — 1 265 px de saut en desktop, 1 841 px en mobile, sur une
page où l'on clique beaucoup
**Dépend de :** carte 343, dont elle reprend l'outil et la méthode
**Fichiers :** `assets/static/css/pages/team-build.css`, `build-team.html`

## Le problème

La carte 343 a soldé les deux causes de son périmètre — la zone de menu et
l'effectif — et le déplacement est tombé à zéro sur les cinq pages qu'elle nomme.
En élargissant la mesure aux autres pages, dix en portent encore. Celle-ci est de
loin la plus lourde :

| Page | Desktop | Mobile |
|---|---|---|
| **Construction d'équipe** | **1 265 px** | **1 841 px** |
| Actions de match | 626 px | — |
| Sélection de match | 596 px | 596 px |
| Finalisation d'équipe | 578 px | 500 px |
| Invitations / nouvelle compétition | ~365 px | ~365 px |
| Brouillon d'équipe | 316 px | 316 px |
| Règles, calendrier, inscriptions, groupes | 124–209 px | 124–342 px |

La construction d'équipe saute en **quatre paliers distincts** — 52, 249, 434 et
530 px en desktop — donc quatre zones différées qui se remplissent
indépendamment. C'est aussi la page d'assemblage à widgets par excellence, celle
que `CLAUDE.md` cite en exemple du patron.

## L'outil existe

`tests/e2e/visual/decalages.py`, écrit pour la 343. Il bloque les requêtes HTMX
pour obtenir l'état exact du premier rendu, puis compare la position des repères
de la page. Il rend les paliers, pas seulement un total.

Ne pas chercher à mesurer le CLS du navigateur : il vaut zéro sur cette
application, le contenu défilant à l'intérieur de `.main-area` et non dans la
fenêtre. Le module l'explique.

## Ce qui décide de la faisabilité, cas par cas

La 343 a montré que deux natures de zone se traitent différemment.

**Hauteur connue d'avance** — la zone de menu vaut deux barres de
`--menubar-height`. La réservation s'écrit avec le token, elle est exacte, et
rien n'est à resynchroniser.

**Hauteur qui dépend d'une donnée** — l'effectif. La page ne peut pas la
connaître, la souveraineté des BCs le lui interdisant. Ce qui a rendu la
réservation sûre, c'est que **onze joueurs est une règle du jeu** : aucune fiche
n'en affiche moins, donc jamais de blanc permanent.

Pour chaque zone de cette page, la question est donc : *existe-t-il un plancher
aussi solide qu'une règle du domaine ?* Si non, réserver au jugé expose au piège
n°1 de la 343 — un blanc permanent, qui est un défaut pire que le saut parce
qu'il dure.

## Checklist

- [ ] Les quatre paliers sont attribués à leurs conteneurs
- [ ] Pour chacun : plancher justifié par une règle du domaine, ou renoncement
      motivé
- [ ] Aucune valeur en dur là où un token existe
- [ ] `min-height`, jamais `height`
- [ ] Mesure : déplacement nul, ou résiduel chiffré et assumé, en desktop et
      sous 768 px
- [ ] Vérifié qu'aucun blanc permanent n'apparaît, sur les effectifs et rosters
      extrêmes
- [ ] `make lint`, `make check-arch`, `make e2e` passent
