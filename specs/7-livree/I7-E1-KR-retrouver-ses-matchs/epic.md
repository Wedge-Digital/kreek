---
id: I7-E1-KR
title: "Retrouver ses matchs"
status: done
origine: kreek-homerepo:initiatives/5-realisation/I7-retrouver-ses-matchs/I7-E1-retrouver-ses-matchs
---

# I7-E1-KR — Retrouver ses matchs

Epic composant reçue du home repo (`I7-E1`, projection du 2026-11-26). Le besoin,
les décisions et les scénarios S1 à S6 sont dans l'initiative I7 du home repo
(`grilling.md`, `I7-E1-…/epic.md`) ; elle remplace l'épic E25 (K25), remontée au
projet. **Maquette :** `assets/rawpages/html/app-mobile-matches.html`.

## La fonction

Une page « Mes matchs » liste les matchs à jouer des équipes inscrites du coach,
dans l'espace courant. Seul le prochain match de chaque équipe porte un bouton :
« Commencer », « Continuer » ou « Suivre », selon l'état de la saisie, décidé par
`match_report`. « Nouveau match » y démarre un match hors calendrier, masqué si une
compétition de l'espace l'interdit. L'onglet mobile « Matchs » y mène.

## Les tickets

| Carte | Ticket | Intitulé | Dépend de |
|---|---|---|---|
| 675 | I7-E1-KR-T1 | Les matchs à jouer d'un coach | — |
| 676 | I7-E1-KR-T2 | L'action de saisie d'un match | — |
| 677 | I7-E1-KR-T3 | La page « Mes matchs » | 675, 676 |
| 678 | I7-E1-KR-T4 | Les tests e2e de « Mes matchs » | 677 |
| 679 | I7-E1-KR-T5 | Les corrections de « Mes matchs » | 678 |

## Ce qui commande l'ordre

675 (`competitions`) et 676 (`match_report`) ne se touchent pas et se réalisent
en parallèle. 677 les relie — port et adapter de `competitions` vers
`match_report` — et pose la page et l'onglet. 678 rejoue les scénarios.
679 corrige les deux défauts que 678 a trouvés.

## Terminé quand

Les scénarios S1 à S6 de `I7-E1` passent dans la suite e2e.

**Livrée le 2026-11-26** : suite e2e complète verte (575 réussis, 7 ignorés).
