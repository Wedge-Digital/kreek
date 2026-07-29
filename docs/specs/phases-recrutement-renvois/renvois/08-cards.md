# Renvois — Phase 8 : cartes kanban

**Entrée** : phases 2 à 7 validées.

Le tableau complet des dix-sept cartes, le chemin critique et les points de vigilance
sont dans `recrutement/08-cards.md`. Ce document ne reprend que le fil des renvois.

## Les cinq cartes propres aux renvois

| # | Carte | Dépend de |
|---|---|---|
| 267 | Agrégat `DismissalsDraft` — plancher des 11 éligibles, 11 tests | 257, 259 |
| 268 | Use cases — marquage, démarquage, validation en lot | 256, 261, 267 |
| 269 | Page et widgets — trois états par ligne, routes `mark`/`unmark` | 268 |
| 270 | App event `PlayerDismissed` + second recalcul de valeur d'équipe | 251, 260, 261 |
| 271 | Tests e2e — 11 scénarios | 269, 270 |

## Ce que les renvois réutilisent du recrutement

- le brouillon (carte 257) — **même table**, discriminée par phase
- `append_batch` (256)
- `remove_draft_line_use_case` (263) — retirer une ligne est la même opération
- le domain service d'hydratation (263)
- `draft-error.html` — fragment d'erreur partagé
- `ISquadPort` (259), élargi une troisième fois pour l'identité et les SPP

Les renvois n'ajoutent **aucune migration** : tout est couvert par le socle.

## Ce qui leur est propre

**Une seule garde** — le plancher des 11 éligibles. Toutes les gardes de composition du
recrutement sont sans objet : retirer ne viole aucune borne haute.

**Trois états par ligne** au lieu de deux : renvoyable, marqué, bloqué. `Marked` n'a
pas d'équivalent au recrutement, parce qu'une ligne s'annule ici depuis la ligne du
joueur **et** depuis le panier.

**Aucun mouvement de trésorerie**, garanti par le compilateur via le `match` exhaustif
de `treasury_movement()`.

**La course TV** (carte 270), qui n'existe que de ce côté : le recalcul part du bus
interne pendant que la sortie d'effectif transite par l'app event bus.

## Ordre conseillé pour la page

267 → 268 → 269, puis 270 et 271. La 270 peut partir en parallèle de la 268 dès que la
261 et la 260 sont faites.

## Le test qui garde la course

Le scénario 8 de la carte 271 — « après validation, la valeur d'équipe exclut les
renvoyés » — **échoue de façon intermittente** si le second recalcul de la carte 270
n'est pas en place.

C'est voulu. Si ce test devient instable, la réponse n'est pas d'y ajouter une attente :
c'est que la 270 est incomplète.
