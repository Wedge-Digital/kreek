# MR-STEP5-05 — Tests E2E step5

## Objectif

Couvrir le step 5 par des tests Playwright : accès, soumission, re-soumission, états invalides.

## Dépendances

142 — handler et template doivent être opérationnels.

## Conception

Voir `docs/specs/match-report/step5-apres-match/07-integration.md` — section « Tests E2E ».

## Fichiers impactés

- `tests/e2e/` — nouveau fichier de tests step5

## Checklist

- [ ] Scénario : accès à step5 depuis step4 — score et sorties affichés, suggestion de gain pré-remplie dans les inputs
- [ ] Scénario : soumission valide (gains + fan mods + résumé) — redirect, rapport en `ReadyToPublish`
- [ ] Scénario : soumission minimale (sans résumé) — soumission acceptée, titre et corps absents tolérés
- [ ] Scénario : re-soumission — formulaire pré-rempli avec valeurs précédemment saisies, modification acceptée
- [ ] Scénario : accès depuis état `Draft` — redirect vers edit match report
- [ ] Tests verts (`make e2e`)
