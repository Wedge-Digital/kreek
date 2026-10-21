# La page « Imprimer » existe

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** rien
**Spec :** `docs/specs/impression-feuille-equipe/team-print/02-front.md`, `05-use-cases.md`, `06-domaine.md` (R1, R16), `07-integration.md`
**Maquette :** `assets/rawpages/html/app-team-print-modal.html` (une modale dans la maquette, une page dans l'application)
**Fichiers :** `src/app/teams/domain/team.rs`, `src/app/teams/routes.rs`, `src/app/teams/router.rs`, `src/app/teams/use_cases/team_print_access_service.rs`, `src/app/teams/io/web/team_print_page.rs`, `src/app/teams/io/web/templates/teams-print-page.html`, `teams-print-unavailable.html`, `src/app/teams/io/web/team_detail.rs`, `src/web/templates/app-layout.html`

## Objectif

Le bouton « Imprimer en PDF » de la fiche appelle `window.print()` sur la page
web. Il mène désormais à une page dédiée, `/app/{space_id}/teams/{team_id}/print`.
Cette carte livre la page et sa structure : colonne d'options, zone d'aperçu
vide, barre d'actions. L'aperçu arrive à la 596.

## Conception

- **Domaine** : `Team::is_printable()` — inscrite et `ReadyToPlay` (R1), la
  condition même de `BannerVm::pour_etat`.
- **Service** : `team_print_access_service::check(team_id)` charge l'agrégat
  et pose la question ; `// arch:no-instrument` motivé.
- **Contrôleur** : `get_team_print_page` — page complète dans `app-layout` ;
  équipe non imprimable → `teams-print-unavailable.html`, explication et lien
  vers la fiche, pas un 404. Tout utilisateur connecté (R16).
- **Layout** : `{% block body_class %}{% endblock %}` sur le `<body>` de
  `app-layout.html`, vide par défaut ; la page y met `team-print-page`.
- **Fiche** : `BannerCtaVm::Print` devient un lien vers la page (via `Routes`
  de teams), même condition d'affichage qu'aujourd'hui.
- Formulaire d'options (`#print-options`) conforme à la maquette, sans le
  choix « autre équipe » (600) ni « prochain adversaire » (599) : seulement
  « mon équipe seule » actif.

## Checklist

- [ ] `Team::is_printable` + tests : prête à jouer · en recrutement · non inscrite
- [ ] Service, contrôleur, routes, gabarits ; aucun `style="…"`
- [ ] Bloc `body_class` dans `app-layout.html` — vérifier qu'aucune page ne casse
- [ ] Le bouton de la fiche mène à la page
- [ ] `pages/team-print.css` créée, inscrite dans `FEUILLES_APP`
- [ ] `make check-arch`, `make lint`, `make test`
