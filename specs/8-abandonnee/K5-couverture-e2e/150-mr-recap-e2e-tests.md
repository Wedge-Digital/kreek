# MR-RECAP-07 — Tests E2E

## Objectif

Couvrir la page récap par des tests Playwright : accès par état, publication, dégradation
gracieuse, cohérence de l'affichage.

## Dépendances

149 — handler et template doivent être opérationnels.

## Conception

Voir `docs/specs/match-report/recap/07-integration.md` — section « Plan de tests E2E ».

## Fichiers impactés

- `tests/e2e/test_match_report_recap.py` (nouveau)

## Checklist

- [ ] TC-RECAP-01 — page charge en état `ReadyToPublish`, CTA "Publier" + "← Modifier étape 5" visibles
- [ ] TC-RECAP-02 — Sortie sans badge de blessure, Bilan sanitaire ne liste que les `Blesse{injury}`
- [ ] TC-RECAP-03 — MVP affiché à la fois en sidebar et en fin de chronologie
- [ ] TC-RECAP-04 — publication → redirect vers `/recap`, CTA devient "Retour compétition" + "Voir fiche"
- [ ] TC-RECAP-05 — double publication (POST direct) → 409
- [ ] TC-RECAP-06 — `Draft`/`PreMatch` → 404, `Cancelled` → 410
- [ ] TC-RECAP-07 — dégradation gracieuse si `find_round_context` échoue (pas de bandeau contexte, page fonctionnelle)
- [ ] TC-RECAP-08 — carte "Performances (SPP)" ne casse pas l'affichage (stub)
- [ ] Tests verts (`make e2e`)
