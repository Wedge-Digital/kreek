# Recrutement — Phase 8 : cartes kanban

**Entrée** : phases 2 à 7 validées.

Les cartes couvrent **les deux pages** : le socle est commun, seules les cartes 262-266
et 267-271 sont propres à une page. Ce document liste l'ensemble ; `renvois/08-cards.md`
n'en reprend que le fil des renvois.

## Socle — préalable aux deux pages

| # | Carte | Dépend de |
|---|---|---|
| 255 | Trésorerie en mouvements — grand livre, tag dérivé, suppression de `refund_kpo` | **251** |
| 256 | `append_batch` — appliquer un lot atomiquement | 255 |
| 257 | Brouillon de phase — table, repository, version optimiste, purge | **251** |
| 258 | Port catalogue de roster — et **limites croisées enfin appliquées** | — |
| 259 | `ISquadPort` — le port de consultation de l'effectif | **250** |
| 260 | Appartenance à l'effectif dans `players` | — |
| 261 | Méthodes domaine de `Team` — achat et renvoi | 255 |

## Recrutement

| # | Carte | Dépend de |
|---|---|---|
| 262 | Agrégat `RecruitmentDraft` — 8 gardes, 17 tests | 257, 258, 259 |
| 263 | Use cases — hydratation, panier, validation en lot | 256, 261, 262 |
| 264 | Page et widgets | 263 |
| 265 | App event `PlayerRecruited` → création du joueur | 261 |
| 266 | Tests e2e — 11 scénarios | 264, 265 |

## Renvois

| # | Carte | Dépend de |
|---|---|---|
| 267 | Agrégat `DismissalsDraft` — plancher des 11, 11 tests | 257, 259 |
| 268 | Use cases | 256, 261, 267 |
| 269 | Page et widgets | 268 |
| 270 | App event `PlayerDismissed` + second recalcul de TV | 251, 260, 261 |
| 271 | Tests e2e — 11 scénarios | 269, 270 |

## Chemin critique

**Rien ne démarre avant la carte 251**, qui crée le bus interne de `teams` et la
publication depuis l'append. Trois cartes du socle en dépendent directement — 255, 257
et 270 — et sans elles ni le grand livre, ni la purge des brouillons, ni le second
recalcul de valeur d'équipe n'ont de support.

Les cartes **258 et 260 sont indépendantes** et peuvent partir immédiatement, en
parallèle de la 251.

## Deux cartes à surveiller

### 258 — la plus incertaine

Elle ne se contente pas d'ajouter un port. Elle doit :

- **unifier deux schémas JSON incompatibles** dans `teams_fr.json` — `{max, in}` pour
  trois rosters, `{limit, limitedPlayerIds}` pour les Élus du Chaos
- remonter le champ jusqu'à `references::TeamDefinition`, puis au port de
  `team_creation`
- supprimer le `cross_limits: vec![]` codé en dur à `roster_service.rs:68`

Effet de bord assumé : **les limites croisées s'appliqueront enfin à la construction
d'équipe**, où elles n'ont jamais fonctionné. Une équipe existante pourrait donc être
en infraction. À vérifier avant de livrer.

### 260 — la plus étendue

Elle modifie **sept chemins de lecture** dans trois BCs, dont deux que les cartes 250
et 253 touchent déjà. C'est le point de collision le plus probable si les deux séries
avancent en parallèle.

Le choix qui la rend sûre : `find_by_team_id` filtre **à la source**, sans variante
`…_including_dismissed`. Aucun appelant n'a de filtre à écrire, donc aucun ne peut
l'oublier.

## Découpage — ce qui a été fusionné

Les méthodes domaine de `Team` pour l'achat **et** le renvoi tiennent dans une seule
carte (261) : même fichier, mêmes corrections d'asymétrie — l'apothicaire achetable, la
relance renvoyable — une seule session.

À l'inverse, les deux app events restent séparés (265 et 270) : ils n'ont pas les mêmes
dépendances et ne débloquent pas la même page.
