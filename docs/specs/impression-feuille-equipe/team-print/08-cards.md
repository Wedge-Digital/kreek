# Page « Imprimer » — Phase 8 : cartes

Épic : `kanban/epics/ready_to_be_done/E16-imprimer-la-feuille-d-equipe.md`.
Cartes dans `kanban/ready_to_be_done/`.

## PR 1 — la page, les fiches d'équipe, le choix des équipes

| # | Carte | BC | Dépend de |
|---|---|---|---|
| 594 | La seconde équipe d'une URL est contrôlée par `space_scope` | infra, web | — |
| 595 | La page « Imprimer » existe | teams | — |
| 596 | L'aperçu assemble les feuilles et s'imprime | teams | 595 |
| 597 | L'en-tête de fiche imprimable | teams | 596 |
| 598 | Le tableau des joueurs imprimable | players | 596 |
| 599 | Imprimer avec le prochain adversaire | competitions, teams | 594, 597, 598 |
| 600 | Imprimer avec une autre équipe de l'espace | teams | 599 |
| 601 | Les tests e2e de la page « Imprimer » (E1 à E8) | — | 594 à 600 |

## PR 2 — les annexes et la feuille de match

| # | Carte | BC | Dépend de |
|---|---|---|---|
| 602 | Le favori et la petite monnaie deviennent des fonctions pures | match_report | — |
| 603 | La disponibilité des star players rejoint le domaine | references | — |
| 604 | `match_reminders_fr.json`, premier fichier optionnel du corpus | references | — |
| 605 | `match_report` lit le catalogue de références | match_report, infra | 603, 604 |
| 606 | L'annexe des compétences | players | 594, 596 |
| 607 | Les coups de pouce et les star players disponibles | match_report | 594, 596, 605 |
| 608 | La feuille de match imprimable | match_report | 594, 596, 602, 605 |
| 609 | Les tests e2e des annexes et de la feuille de match (E9 à E11) | — | 602 à 608 |
