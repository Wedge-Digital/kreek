# L'annexe des compétences

**Priorité : moyenne**
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** 596, 594
**Spec :** `04-dtos.md` (`SkillsAnnexVm`), `05-use-cases.md`
**Fichiers :** `src/app/players/use_cases/skills_annex_service.rs`, `src/app/players/io/web/widgets/print_skills_annex_widget.rs`, `templates/widgets/print-skills-annex.html`, `assets/static/css/widgets/print-skills-annex.css`, `src/app/players/routes.rs`, `src/app/teams/io/web/widgets/print_preview_widget.rs`

## Objectif

L'option « Détail des compétences » : chaque compétence portée par les équipes
imprimées, une seule fois, avec sa catégorie et sa description.

## Conception

- **Routes** `…/players/print/skills/{team_id}` et `…/with/{other_team_id}`,
  query `orientation`, `density`.
- **Service** : joueurs vivants des équipes, compétences de base (par le poste)
  et acquises, résolues par `ISkillCatalogPort::find_skill`.
- Dédoublonnées, triées par nom, **découpées en pages par le serveur** selon
  l'orientation et la densité ; le fragment rend une à n feuilles.
- Pas de ligne « portée par… » : `players` ne connaît pas le nom des équipes
  (`04-dtos.md`).
- L'aperçu ajoute l'emplacement quand l'option est cochée.

## Checklist

- [ ] Service, contrôleur, gabarit, feuille CSS inscrite au bundle
- [ ] Tests unitaires : une compétence partagée par deux équipes n'apparaît qu'une fois · tri · découpage en pages · compétence inconnue du catalogue ignorée sans panique
- [ ] `make check-arch`, `make lint`, `make test`
