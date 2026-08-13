# La valeur d'équipe suit la customisation de prix — et elle seule

**Priorité : haute**
**Dépend de :** `308-players-customisation-endpoints.md`
**Bloque :** `309-players-customisation-e2e.md` (scénario 6)
**Contexte :** `players` → `teams`, par app event

## Le manque

Aucune customisation n'émet d'app event. `PlayersAppEvent` ne connaît que
`InitialRosterCompleted` et `PlayerDismissed`, et `team_value_listener` ne
réagit qu'à ces deux-là.

Conséquence : **une customisation de prix ne déclenche aucun recalcul de valeur
d'équipe.** La TV reste sur l'ancienne valeur jusqu'à ce qu'un événement sans
rapport — validation de phase, renvoi — provoque un recalcul par hasard. La
fiche joueur montre le nouveau prix, la fiche équipe l'ancienne TV, et rien ne
signale la divergence.

Découvert en préparant la carte 309, dont le scénario 6 exige que la TV suive.
Ce scénario aurait été rouge sans que le test soit en cause.

## L'asymétrie est la règle, pas un oubli

**Seul le prix déplace la TV.** Une compétence ou une caractéristique obtenue
par customisation ne la bouge pas — décision explicite de la phase 3.

C'est contre-intuitif : dans la progression normale, une compétence achetée en
SPP **augmente** la valeur du joueur. La customisation est justement ce qui
échappe aux règles du jeu — elle pose une valeur, elle ne la dérive pas d'un
barème.

Cette asymétrie est la raison d'être de la carte. Émettre l'app event sur les
quatre familles de customisation serait plus simple à écrire, et faux.

## Ce qu'il faut faire

L'adapter `squad_adapter` lit déjà `players_proj.value` : un recalcul déclenché
prendrait la bonne valeur. **Il ne manque que le déclencheur.**

- [x] Variante `PlayerValueCustomised { team_id, player_id }` sur
      `PlayersAppEvent`, avec sa constante `event_type`
- [x] Mapping dans le publisher de `players`, **depuis le domain event du même
      nom qui existe déjà** — et depuis lui seul : ni compétence, ni
      caractéristique, ni SPP
- [x] Ajout à `changes_squad` dans `teams::io::listeners::team_value_listener`
- [x] Test unitaire : le publisher convertit `PlayerValueCustomised` et
      **ignore les trois autres customisations**
- [x] Test unitaire côté `teams` : `changes_squad` retient le nouvel événement

## Pourquoi une carte à part, et pas dans la 309

La 309 est une carte de tests. Un commit intitulé « tests e2e » qui contient
une émission d'app event et un abonnement cross-BC est un commit qu'on ne sait
plus révoquer proprement le jour où l'un des deux pose problème.

Et cette propagation mérite ses propres tests unitaires — que la 309 ne prévoit
pas, puisqu'elle ne prévoyait pas ce travail.

## Point de vigilance

`team_value_listener` porte deux fonctions d'abonnement distinctes —
`init(event_bus:)` pour l'intra-BC, `init_from_app_events(app_event_bus:)` pour
le cross-BC. **C'est cette convention de nommage que l'axe 5 de `check-arch`
utilise** pour distinguer les deux, et l'en-tête du fichier prévient de ne pas
la fondre. Le nouvel événement entre par le second.

## Réalisé

L'app event ne porte que `team_id` et `player_id`, sans montant ni delta :
`squad_adapter` relit `players_proj.value`, et un événement qui transporterait
la valeur créerait une seconde source de vérité.

`to_enveloppe` nomme l'**équipe** comme émetteur, pas le joueur — c'est elle que
le listener recalcule. Un test le fixe, parce que rien dans le type ne
l'impose.

L'asymétrie est tenue par le test du publisher, côté `players` : les trois
autres customisations rendent `None`. Côté `teams`, il n'y a donc rien à
filtrer — ce qui est noté dans le test du listener, sans quoi on chercherait
un jour la moitié manquante de la règle au mauvais endroit.
