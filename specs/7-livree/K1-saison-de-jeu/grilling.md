# K1 — Grilling

> Reconstitué à la migration (2026-11-19), à partir de l'épic `kanban/epics/done/E01-saison-de-jeu.md` (devenue `initiative.md`). Le dépôt ne garde
> pas de trace d'un grilling : les besoins ci-dessous reprennent l'objectif et les cartes, à relire.

## Questions et réponses

Aucune trace dans le dépôt.

## Besoins

- **K1-B1** — Terminé quand (critère de l'épic E01) : Une équipe traverse une journée complète — match joué, recettes encaissées, SPP dépensés, recrutement, renvois, retraite temporaire, erreur coûteuse — et se retrouve « Prête à jouer » pour la journée suivante, sans intervention manuelle en base. **Constaté :** oui. La séquence est en production et exercée par la suite e2e.
- **K1-B2** — BC `teams` — Inscription dans une compétition → "Inscrite / Prête à jouer" (ticket `32-team-enrollment`)
- **K1-B3** — BC `teams` — Listener app event `MatchPlayed` → domain event `PostMatchSequenceStarted` (ticket `35-match-played-fans`)
- **K1-B4** — BC `teams` — Phase d'amélioration des joueurs (dépense de SPP) (ticket `36-player-improvement-phase`)
- **K1-B5** — BC `teams` — Phase de recrutement (achat joueurs et staff) (ticket `37-recruitment-phase`)
- **K1-B6** — BC `teams` — Phase de renvois (ticket `38-dismissals-phase`)
- **K1-B7** — BC `teams` — Table de projection + requêtes de liste (ticket `42-teams-projection`)
- **K1-B8** — BC `teams` — Repos hors-saison (ticket `43-off-season`)
- **K1-B9** — BC `competitions` — Règles d'accession et inscription d'équipe (ticket `49-enrollment-rules-competitions`)

## Hors périmètre

Voir la section « Ce que l'épic ne couvre pas » de `initiative.md`, s'il y en a une.
