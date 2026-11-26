---
id: 677-la-page-mes-matchs
parent: I7-E1-KR
ticket: I7-E1-KR-T3
status: done
---

# La page « Mes matchs »

**Epic :** I7-E1-KR · **Besoins :** I7-B1 à I7-B5 · **Dépend de :** 675, 676
**Maquette :** `assets/rawpages/html/app-mobile-matches.html`

## Le constat

- L'onglet mobile s'appelle « Match » (📋) et mène à `new_match_report`
  (`src/web/templates/app-menu.html:103-151`) ; il disparaît entier quand le hors
  calendrier est interdit (`AppMenu.hors_calendrier_interdit`, `src/web/app_menu.rs:31`,
  calculé `:145`). L'onglet actif se déduit du segment d'URL (`app_menu.rs:84-87`).
- La règle du hors calendrier vit dans `competitions` :
  `ISeasonRepository::espace_interdit_hors_calendrier(space_id)`.
- Les cartes de match existantes (`match-widget.html`, macro sur `MatchResultatVm`)
  n'ont pas de bouton d'action.

## Ce qui change

- **Port et adapter** `competitions` → `match_report` : l'action de saisie d'une
  liste d'appariements pour un coach (service de 676), avec son URL —
  « Commencer » et « Continuer » vers `from_pairing`, « Suivre » vers `live` —,
  construite dans l'adapter (`infrastructure/competitions/`) par `AppRoutes`.
- **Route et page** dans `competitions` : « Mes matchs », sous l'espace, coach et
  espace lus de la session et du chemin. Page hôte sans logique, qui charge un
  **widget** des matchs (`hx-disinherit="*"`, paramètres dans l'URL).
- **Le widget** : les matchs à jouer (service de 675), une carte par rencontre sur le
  modèle de la maquette (compétition, journée, date limite, les deux équipes, l'état
  en cours). Seul le prochain match de chaque équipe porte son bouton (676) ;
  « Suivre » est un bouton en contour. État vide : « Aucun match à jouer ».
- **« Nouveau match »** au bas de la page, vers `new_match_report`, **masqué** quand
  `espace_interdit_hors_calendrier` est vrai.
- **L'onglet mobile** devient « Matchs » (🏈), toujours visible, vers la page ;
  actif sur la page. La barre latérale sur ordinateur ne change pas.
- CSS : une feuille de page dans le bundle (`src/web/css_bundle.rs`), scopée,
  sans style inline. Libellés au vouvoiement (règle 19) — la maquette tutoie.

## Fichiers touchés

`competitions/ports.rs`, `competitions/routes.rs`, `competitions/router.rs`,
`competitions/io/web/` (contrôleur de page, widget, gabarits),
`infrastructure/competitions/` (adapter neuf), `main.rs` (injection),
`src/web/templates/app-menu.html`, `src/web/app_menu.rs`, une feuille CSS et
`css_bundle.rs`.

## Règles et invariants

- Aucun handler ne combine des requêtes de deux BCs : `competitions` consulte
  `teams` et `match_report` par ports.
- L'admin ne voit que ses équipes.
- Le bouton n'apparaît que sur le prochain match d'une équipe.

## Tests

Unitaires : le choix du bouton et de son URL par action, le masquage de « Nouveau
match », l'onglet actif, l'adapter (doublure du service de 676). Le parcours
navigateur est la 678.

## Conséquences sur les données de production

Aucune : lecture seule. Sur mobile, « Nouveau match » ne se trouve plus dans la barre
mais dans la page.

## Terminé quand

Sur mobile, l'onglet « Matchs » ouvre la page ; chaque équipe inscrite du coach y
montre son prochain match avec le bon bouton ; « Nouveau match » suit la règle du
hors calendrier.
