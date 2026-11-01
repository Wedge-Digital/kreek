# Le panier se déplie

**Épic :** E20 — Coups de pouce : le panier et l'annulation · **Ordre :** indépendant, peut partir le premier · **Dépend de :** —
**Conception :** `docs/specs/saisie-mobile/E20-cadrage.md` (R8) · **Maquettes :**
`app-mobile-real-2b-inducements-cart.html`, `app-mobile-real-2b-inducements-cart-open.html`

## Objectif

Le pied de page dit « N coups de pouce sélectionnés » et ne liste que les
mercenaires (`inducements.html:103-122`). Pour retirer un article commun, spécial
ou une vedette, il faut retrouver son onglet. Après la carte, le résumé du
panier est un bouton qui le déplie : **tous** les articles y sont listés, chacun
s'ajuste (− / +) ou se retire (✕) sur place, avec le reste à dépenser.

Même gabarit sur ordinateur, où le panier peut rester ouvert.

## Conception

**Le sélecteur garde les quantités** (`references/…/inducement-selector.html`) ;
le panier ne les touche pas, il **demande** :

- le panier publie `adjustInducement { uid, delta }` sur `body` — `delta`
  vaut `+1`, `-1`, ou `-qty` pour retirer ;
- le sélecteur l'écoute par `@adjust-inducement.window` dans son `x-data`,
  borne par `max_qty`, et republie `inducementSelectionChanged` comme
  aujourd'hui ;
- `inducementSelectionChanged` porte en plus, par article, `max_qty` et
  `category` (`common`, `special`, `star`) : le panier en tire « Commun · 15 kPo
  l'unité » et l'état du bouton +.

C'est le patron de `removeMercenaire` (règle 2 des widgets) ; aucun des deux
widgets ne connaît l'autre, `match_report` n'importe rien de `references`.

**Au passage** : l'écouteur `removeMercenaire` est posé par
`document.body.addEventListener` sans retrait
(`mercenary-selector-widget.html:84`) et survit au swap. Il devient
`@remove-mercenaire.window`, retiré avec le composant.

**Le pied de page** : un résumé repliable (« 4 coups de pouce · Voir le panier
▴ » / « Masquer ▾ »), la liste, « Reste à dépenser ». Le formulaire envoyé ne
change pas — `selection` et `mercenaries`.

**CSS** : `pages/match-report-inducements.css` pour le panier,
`widgets/inducement-selector.css` n'est pas touché. Sous 768 px, le pied de page
reste décalé au-dessus de la barre mobile.

## Données de production

Aucune. Affichage seulement ; ce que le serveur reçoit est identique.

## Checklist

- [x] `adjustInducement` écouté par le sélecteur, `max_qty` et `category` dans
      l'événement
- [x] Le panier dépliable, toutes catégories, ajuster et retirer
- [x] `removeMercenaire` en `@remove-mercenaire.window`
- [x] Les sélecteurs e2e du panier (`.mr-cart-merco-item`, `.mr-cart-merco-remove`,
      `.mr-cart-items`) gardés ou repris dans
      `test_match_report_step2_mercenaires.py` et `test_match_report_inducements.py`
- [x] `scripts/check-css-collisions.sh` ; `make lint`, `make check-arch`, e2e impactés
