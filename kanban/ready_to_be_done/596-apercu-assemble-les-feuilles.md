# L'aperçu assemble les feuilles et s'imprime

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** 595
**Spec :** `docs/specs/impression-feuille-equipe/team-print/02-front.md` (principe, décisions 2, 4, 7), `04-dtos.md` (`PrintOptionsQuery`, `PrintPreviewVm`)
**Fichiers :** `src/app/teams/io/web/widgets/print_preview_widget.rs`, `templates/widgets/print-preview.html`, `teams-print-page.html` (script), `assets/static/css/pages/team-print.css`, `src/app/teams/routes.rs`

## Objectif

La zone d'aperçu de la page se remplit de feuilles A4, se recharge à chaque
option, et « Imprimer » sort **les feuilles seules**, à taille réelle.

## Conception

- **Route** `…/teams/{team_id}/print/preview`, query `PrintOptionsQuery` (enums
  désérialisés : une valeur inconnue répond 400). Rend `PrintPreviewVm` : la
  classe racine (orientation, densité, encre, style) et les emplacements.
  Cette carte n'en pose qu'un : la fiche de mon équipe, deux fragments vides
  que remplissent la 597 et la 598.
- **Déclenchement** : `hx-trigger="load, change from:#print-options delay:300ms"`,
  `hx-include="#print-options"`.
- **CSS d'impression**, limité à la page :
  - pages nommées `@page a4-portrait` / `@page a4-landscape`, déclarées par
    les feuilles seules — le Ctrl+P des autres pages ne change pas ;
  - `@media print` sur `body.team-print-page` : chrome du layout, options et
    barre d'actions masqués, feuilles à taille réelle, une par page.
- **Script de la page** (`document.currentScript.previousElementSibling`,
  fonctions ≤ 20 lignes) :
  - réduction des feuilles à la largeur de l'aperçu ;
  - suivi du chargement des fragments + `document.fonts.ready` →
    `printPreviewReady { pages }` ;
  - **densité automatique** : une feuille qui déborde passe en compact ;
    déborde encore → avertissement visible dans l'aperçu (rien n'est coupé en
    silence) ;
  - « Imprimer » désactivé jusqu'à `printPreviewReady`, puis `window.print()`.
- États de la maquette : préparation de l'aperçu, erreur avec « Réessayer ».

## Checklist

- [ ] Route, contrôleur, `PrintPreviewVm`, gabarit
- [ ] Tests unitaires : désérialisation des options (valeur inconnue → erreur), classe racine produite
- [ ] CSS : pages nommées, `@media print` limité à la page, aucune règle hors racine (axes 14, 15, 17)
- [ ] Script : réduction, `printPreviewReady`, densité automatique, impression
- [ ] `make check-arch`, `make lint`, `make test`
