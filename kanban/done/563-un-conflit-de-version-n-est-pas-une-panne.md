# Un conflit de version n'est pas une panne

**Priorité : moyenne — le jet des erreurs coûteuses répond 500 une fois sur huit**
**Épic :** aucune — une correction, livrable d'un bloc
**Dépend de :** rien
**Fichiers :**
`src/app/teams/use_cases/apply_costly_mistakes_use_case.rs`,
`src/app/teams/use_cases/test_doubles.rs`,
`src/app/teams/io/web/costly_mistakes.rs`

## L'objectif

Un coach qui lance le dé pendant qu'un écouteur écrit encore sur son équipe
obtient son jet, et non une erreur serveur.

## Ce qui l'a fait naître

Un passage de CI a échoué huit fois de suite sur `test_roster_edition`, toutes
au même endroit : `POST …/costly-mistakes/roll` a répondu **500**. Le même
commit rejoué ensuite est passé — c'était donc une course, pas une régression.

Le contrôleur traduit `TeamNotFound` en 404 et un refus du domaine en 409. Un
500 ne peut venir que de la persistance, et la seule qui se produise dans cette
fenêtre est `ConcurrentWrite` : l'ajout emploie le numéro de version lu un
instant plus tôt, et la validation des renvois vient de déclencher des
écouteurs qui écrivent sur la même équipe. Le test poste dès que la phase
bascule ; il tombe parfois entre les deux.

**Le commentaire de la fixture de `test_roster_edition` documentait déjà « un
échec sur huit exécutions »** dans cette même fenêtre, en l'attribuant à un
clic perdu. Le clic avait été corrigé ; la course, elle, restait.

## Ce que la maison fait ailleurs

Un conflit de version n'est traité comme une panne nulle part ailleurs :

| Endroit | Ce qu'il en fait |
|---|---|
| édition d'effectif | « l'effectif a été modifié entre-temps, réessayez » |
| panier de customisation | le panneau re-rendu porte l'état réel, sans message |
| création de joueur | traduit en « déjà traité » |

Le jet est le seul à en faire un 500. C'est la même famille que le défaut de la
carte 561, où un refus d'achat tombait dans la branche générique du contrôleur.

## Le changement

**Le use case réessaie une fois.** L'ajout ayant été rejeté, **rien n'a été
écrit** : relire l'équipe et relancer le dé sur son état frais est sûr, et plus
juste que de rejouer un dé tiré sur une trésorerie périmée. La seconde tentative
est journalisée en `warn` — une course qui se répéterait doit rester visible.

Une seule fois, pas une boucle. Un conflit qui persiste n'est plus une course
mais un problème d'écriture, et l'escamoter ferait exactement ce que le
`CLAUDE.md` proscrit : une étape qui rassure au lieu d'échouer.

**Le contrôleur traduit le conflit restant en 409**, avec le même statut que le
second jet refusé par le domaine : la requête est bien formée, c'est l'état qui
a changé. Le journal passe de `error` à `warn`, puisque ce n'est pas une panne.

Le message de l'écran ne bouge pas. « Le jet a échoué. Rechargez la page pour
connaître votre situation. » est déjà ce qu'il faut dire d'un échec récupérable,
et le script l'affiche sur toute réponse non `ok`.

## Ce que la carte ne touche pas

Les autres écritures de `teams` qui peuvent connaître la même course. Celle-ci
est la seule qu'un test ait prise en flagrant délit, et corriger à l'aveugle des
fenêtres qu'on n'a pas mesurées reviendrait à ajouter des réessais partout.

## Tests

Unitaires, sur le use case :

| | |
|---|---|
| `un_conflit_de_version_est_reessaye_une_fois` | le premier ajout est rejeté, le second passe : le coach obtient son jet |
| `un_conflit_qui_persiste_remonte` | deux rejets de suite : l'erreur sort, aucune boucle |

Le contrôleur, dont la traduction est extraite dans `statut_du_refus` pour
être mesurable : `un_conflit_de_version_est_un_409_pas_un_500`, son
contre-exemple `une_panne_de_depot_reste_un_500` — sans lui, une fonction qui
rendrait 409 pour tout passerait le premier — et
`un_second_jet_reste_un_409_et_une_equipe_inconnue_un_404`, qui épingle les
deux statuts qui ne changent pas.

Le dépôt factice de `teams` gagne un compteur de conflits à simuler : la course
réelle ne se reproduit pas à volonté dans un test séquentiel. Zéro par défaut,
donc aucun test existant ne change de comportement.

## Terminé quand

Un jet lancé pendant qu'un écouteur écrit sur l'équipe aboutit, et le journal
porte un `warn` de réessai plutôt qu'un `error` de panne.
