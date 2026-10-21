# Les tests e2e de la page « Imprimer »

**Priorité : moyenne** — dernière carte de la PR 1
**Épic :** E16 — Imprimer la feuille d'équipe · PR 1
**Dépend de :** 594 à 600
**Spec :** `07-integration.md` (« Tests E2E prévus », E1 à E8)
**Fichiers :** `tests/e2e/test_team_print_page.py`, `tests/impact-map.toml`

## Objectif

Vérifier dans un navigateur ce qu'aucun test unitaire ne voit : l'aperçu,
l'impression, la sécurité des URLs.

## Scénarios

| # | Scénario |
|---|---|
| E1 | Bouton d'une équipe prête à jouer → page ; absent en recrutement ; URL directe d'une équipe non prête → explication |
| E1b | Un coach connecté qui ne possède pas l'équipe imprime ; un visiteur non connecté est renvoyé à la connexion |
| E2 | Cocher / décocher une option recharge l'aperçu ; le nombre de pages suit |
| E3 | Prochain adversaire proposé avec sa journée ; sans match à venir, option grisée avec son motif |
| E4 | La cascade compétition → saison → équipe ajoute la fiche de l'autre équipe |
| E5 | Une seconde équipe d'un autre espace dans l'URL d'un fragment → 404 |
| E6 | `page.pdf()` : autant de pages que de feuilles dans l'aperçu, sans le texte des options |
| E7 | « Imprimer » désactivé jusqu'à `printPreviewReady`, puis `window.print()` appelé (doublé) |
| E8 | Une feuille trop pleine passe en compact ; rien n'est coupé |

## Checklist

- [ ] Les scénarios, clics sur contenu injecté par `cliquer_quand_cable`, sans `sleep`
- [ ] Entrée dans `tests/impact-map.toml` (axe 8)
- [ ] `make e2e` vert, `make check-arch`
