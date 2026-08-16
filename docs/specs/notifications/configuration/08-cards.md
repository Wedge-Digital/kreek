# Phase 8 — Cartes : `configuration/`

Les cartes des **deux** unités ont été produites ensemble, à la fin de `envoi/`.
La liste complète et son ordonnancement vivent dans `../envoi/08-cards.md` ;
seules les quatre cartes de cette unité sont reprises ici.

| # | Carte | Dépend de |
|---|---|---|
| 331 | Réglages : domaine et persistance | — |
| 332 | Widget de réglage et son hôte admin | 331 |
| 333 | Widget de réglage dans le magicien | 331, 332 |
| 334 | Retrait des trois réglages morts | 333 |

## Pourquoi ces quatre-là ne se livrent pas seules

Aucune ne fait partir un email. Livrées sans `envoi/`, elles produiraient un
**troisième interrupteur email mort** — mieux dessiné que les deux qu'il
remplace, et tout aussi inerte. C'est le défaut que cette fonctionnalité corrige ;
le reproduire en le livrant à moitié serait la pire issue possible.

L'avertissement est répété en tête de chacune des quatre cartes.

## Ordre, et pourquoi 332 précède 333

Le mode auto-save n'a besoin d'**aucune** modification du magicien : la carte est
livrable seule, et elle rode le widget sur l'hôte le plus simple avant de
l'installer dans le plus contraint — celui qui enregistre d'un bloc et demande
une réhydratation.

334 est isolée en dernier parce qu'elle frotte avec la spec de retrait des
play-offs : même template. Isolée, elle se replanifie sans bloquer le reste.
