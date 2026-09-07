# Phase 5 — Use cases : l'encart du coach connecté

**Entrée** : `04-dtos.md` validé — un DTO d'entrée, un service d'hydratation,
trois VM.

## Aucun use case nouveau — le troisième appelant du même

L'encart appelle **`record_answer_use_case`**, écrit en unité 1, avec
`Repondant::Coach(id du connecté)` (R28).

Les trois unités appellent donc bien le même point d'écriture, comme
`03-back.md` de l'unité 1 l'avait posé. C'est ce qui fait tenir R13, R19 et R21
à un seul endroit — et R28 avec elles, depuis la phase 2.

Ce que cette phase spécifie est donc, comme pour la page publique, **ce que
devient chaque refus** — plus un cas que les deux autres unités ne rencontrent
pas.

## Ce que rend chaque issue

Le fragment remplace sa propre racine. Il doit donc, dans tous les cas,
**contenir une réponse visible** — c'est le point qui demande de l'attention.

| Issue | Réponse |
|---|---|
| `Ok(Enregistree)` | l'encart réaffiché, la ligne à jour |
| `Ok(EnregistreeRencontreARefaire)` | idem, **plus la mention de R30** ci-dessous |
| `SurveyClosedForCoach` (R21) | une carte « cette question n'est plus ouverte », sans boutons |
| `RoundFrozenByReport` (R13) | une carte « la journée a déjà été jouée », sans boutons |
| `TeamNotOwnedByCoach` (R28) | `403`, **aucun swap** |
| `TeamNotInSurvey` (R19) | `403`, aucun swap |
| panne | `500`, journalisée, aucun swap |

### Le clic qui arrive trop tard doit se voir

C'est la course à traiter : le coach ouvre la page, l'organisateur clôt la
campagne, le coach clique. Sans carte explicative, le fragment réaffiché serait
**vide** — la campagne close ne produit plus rien (phase 2) — et l'encart
disparaîtrait sous le curseur. Le coach lirait ce vide comme un succès.

La carte de refus n'est donc pas une politesse : c'est ce qui distingue « c'est
enregistré » de « c'est trop tard ». Elle disparaît au prochain chargement de
page, comme tout ce qui n'est pas persisté.

### Un refus d'autorisation ne rend rien, et c'est correct

`TeamNotOwnedByCoach` et `TeamNotInSurvey` ne peuvent venir que d'un `team_id`
forgé — l'encart n'offre que les équipes du demandeur. htmx ne remplace rien sur
un `4xx`, donc le clic reste sans effet visible.

**Ne rien expliquer est ici le bon comportement** : il n'y a pas d'utilisateur à
renseigner, seulement une requête qui n'aurait pas dû exister. Une carte
« cette équipe n'est pas la vôtre » confirmerait à qui essaie que l'équipe
existe.

## Le cas que les deux autres unités ne rencontrent pas

Un coach peut se décommander **après le tirage**, depuis l'encart. R12 et R16 le
prévoient — c'est même le cas qu'elles décrivent — mais elles le décrivaient vu
de l'organisateur, seul écran qui existait alors.

`record_answer` rend `EnregistreeRencontreARefaire { pairing }`, et
**l'organisateur seul répare** (carte 518). L'encart doit donc dire au coach ce
qui vient de se passer, sans rien promettre qu'il ne contrôle pas.

## Ce que l'encart ne fait pas

**Il ne montre pas l'adversaire.** La maquette écrit « Ton adversaire te sera
communiqué après le tirage » — c'est une promesse sur l'onglet Calendrier, qui
affiche déjà les rencontres, pas une fonctionnalité de l'encart.

L'y ajouter demanderait de lire les appariements de la journée à chaque
affichage de la page de détail, pour une information que l'onglet d'à côté donne
déjà.

**Il ne répare rien.** La réparation est une décision d'organisateur, prise sur
une proposition (carte 518). Un encart qui réapparierait tout seul écrirait au
calendrier sans que personne l'ait validé.

## Règle métier apparue en phase 5

### R30 — Un désistement après tirage le dit, et ne promet rien

Apparue en phase 5, en croisant R12 et R16 avec le fait que l'encart existe.

Quand la réponse d'un coach défait une rencontre déjà tirée, l'encart le lui
dit — « ta rencontre était déjà tirée ; l'organisateur en sera informé » — et
s'arrête là.

**Il n'annonce pas de nouvel adversaire**, parce qu'il n'y en a pas encore : la
réparation est une proposition que l'organisateur valide (R12). Annoncer un
remplacement que personne n'a confirmé produirait deux coachs qui se croient
appariés et un match qui n'existe pas.

**Il ne se tait pas non plus.** Un désistement silencieux laisserait le coach
croire qu'il a simplement changé une case, alors qu'il vient de défaire un match
que son adversaire avait noté. Le dire est ce qui donne à ce clic son poids
réel — et, accessoirement, ce qui décourage de le faire à la légère.

C'était le seul point où l'écran du coach touche une règle écrite pour
l'organisateur.
