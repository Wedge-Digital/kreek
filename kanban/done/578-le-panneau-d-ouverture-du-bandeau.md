# Le panneau d'ouverture du bandeau

**Priorité : moyenne**
**Épic :** aucune — fonction « phases manuelles », cartes 575 à 579
**Dépend de :** 577
**Maquette :** `assets/rawpages/html/app-team-phase-override.html`
**Spec :** `02-front.md`, `03-back.md`, `04-dtos.md`, `07-integration.md` (point 6)
**Fichiers :** `src/app/teams/routes.rs`, `src/app/teams/router.rs`,
`src/app/teams/io/web/phase_override_controller.rs` *(nouveau)*,
`src/app/teams/io/web/templates/phase-override-error.html` *(nouveau)*,
`src/app/teams/io/web/team_detail.rs`,
`src/app/teams/io/web/templates/teams-team-detail.html`,
`assets/static/css/pages/team-page.css`,
`tests/e2e/test_manual_phase_override.py` *(nouveau)*, `tests/impact-map.toml`

## L'objectif

Un admin de l'espace ou de la compétition ouvre, depuis le bandeau d'une équipe
prête à jouer, l'une des trois phases — telle que la maquette la montre.

## Le changement

- Route `PHASE_OVERRIDE`, dans `routes_ouvertes()` ; contrôleur
  `post_phase_override`, gardé par `require_team_admin`.
- `BannerCtaVm::OpenPhaseOverride { post_url, choices }` quand l'équipe est
  prête à jouer et `is_team_admin` ; `PhaseChoiceVm` construit depuis
  `OverridablePhase::ALL`.
- Le panneau sous le bandeau, `x-data` Alpine (ouverture, choix, libellé du
  bouton) ; le message d'erreur `phase-override-error.html` — le message seul, pas le pied (décision du 2026-10-03).
- Succès `HX-Refresh` ; refus en 200 avec `HX-Retarget` / `HX-Reselect` /
  `HX-Reswap` vers `#phase-override-error` ; phase inconnue 400.
- CSS dans `pages/team-page.css`, sous `.team-page`.

## Tests

Unitaires : le VM du bandeau porte le bouton pour un admin d'une équipe prête,
pas pour un coach ni dans une autre phase ; `PhaseChoiceVm` reprend les trois
phases.

E2E (décision du 2026-10-03 — la première carte visible porte ses tests) :

1. L'admin ouvre chacune des trois phases depuis le bandeau ; l'équipe passe
   dans la phase, avec son bandeau habituel.
2. Un simple membre ne voit pas le bouton ; sa requête forgée reçoit 403.

## Terminé quand

Un admin ouvre une phase depuis la fiche, l'équipe affiche le bandeau de la
phase, les deux scénarios e2e passent, et `make test`, `make lint`,
`make check-arch`, `make e2e` passent.
