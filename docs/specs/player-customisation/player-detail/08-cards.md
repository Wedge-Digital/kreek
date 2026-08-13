# Phase 8 — Cartes kanban — player-detail

Huit cartes, ordonnées par dépendance. Toutes dans `kanban/ready_to_be_done/`.

| # | Carte | Dépend de | Résumé |
|---|---|---|---|
| 302 | `players-customisation-domain` | — | Table des directions et bornes descendues sur `StatKind`, value objects, quatre événements, branches `apply()`, méthodes `customise_*` |
| 303 | `players-stat-deltas-projection` | 302 | Colonnes de deltas sur `players_proj`, recalcul depuis l'agrégat dans la transaction, toutes sources confondues |
| 304 | `players-customisation-basket-domain` | 302 | L'agrégat panier : lignes, hydratation, gardes, `validate_all`, `action_for_*` |
| 305 | `players-customisation-basket-persistence` | 304 | Table, port, implémentation Pg, péremption 24 h |
| 306 | `players-customisation-use-cases` | 303, 305 | Cinq mutations, validation, annulation |
| 307 | `players-customisation-widget` | 304 | Widget `GET`, template, autorisation resserrée, bascule de la fiche |
| 308 | `players-customisation-endpoints` | 306, 307 | Les sept `POST` |
| 309 | `players-customisation-e2e` | 307, 308 | Les dix scénarios |

## Deux choix de découpage

**La 303 est isolée volontairement.** Elle touche des comportements
**existants** — augmentations SPP, séquelles, corrections de match — et c'est
la seule de la série qui puisse casser ce qui marche aujourd'hui. Son propre
commit, ses propres tests, un `git revert` propre si elle tourne mal.

Elle ne dépend que des événements de la 302, donc elle peut partir tôt, avant
même que le panier existe.

**La 307 regroupe tout le chemin de lecture** : le widget, l'autorisation et le
choix de l'occupant du slot. Les trois se testent ensemble et n'ont pas de sens
séparément — un widget qu'on ne peut pas atteindre ne prouve rien.

`can_customise` vaut **déjà** ce qu'il faut : `check_admin_rights` exclut le
coach. Une rédaction antérieure de cette spec annonçait un resserrement, par
confusion avec `can_spend_spp`. Il n'y a rien à changer, mais la règle mérite
un test — elle ne tient qu'à la composition d'une fonction que rien n'empêche
d'élargir.

## Ce qui n'est pas découpé

Les quatre familles — compétence, caractéristique, prix, SPP — **ne font pas
quatre cartes**. Elles partagent l'agrégat, le panier, le template et les
endpoints ; les séparer multiplierait les cartes sans rendre aucune livrable
indépendamment.

## Ordre de réalisation conseillé

`302 → 303 → 304 → 305 → 306 → 307 → 308 → 309`.

La 303 et la 304 peuvent avancer en parallèle après la 302, comme la 307 après
la 304. Le chemin critique passe par 302 → 304 → 305 → 306 → 308 → 309.
