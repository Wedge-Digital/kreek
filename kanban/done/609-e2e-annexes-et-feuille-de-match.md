# Les tests e2e des annexes et de la feuille de match

**Priorité : moyenne** — dernière carte de la PR 2
**Épic :** E16 — Imprimer la feuille d'équipe · PR 2
**Dépend de :** 602 à 608
**Spec :** `07-integration.md` (« Tests E2E prévus », E9 à E11)
**Fichiers :** `tests/e2e/test_team_print_annexes.py`, `tests/impact-map.toml`

## Scénarios

| # | Scénario |
|---|---|
| E9 | Annexe des compétences : chaque compétence une seule fois, avec sa description |
| E10 | Coups de pouce : prix par roster, « — » pour un coup de pouce réservé ; star players filtrés par roster |
| E11 | Feuille de match : favori et petite monnaie conformes aux VE ; journaliers marqués ; barème SPP de chaque roster ; rappels et météos du corpus de démonstration |

## Checklist

- [x] Les scénarios, sur le corpus de démonstration
- [x] Entrée dans `tests/impact-map.toml` (axe 8)
- [x] `make e2e` vert, `make check-arch`
