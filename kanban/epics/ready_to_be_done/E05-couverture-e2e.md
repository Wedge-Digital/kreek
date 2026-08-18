# E05 — Couverture e2e du déjà livré

**État :** 4 cartes · 0 faite

## La fonction

Le `CLAUDE.md` pose une règle : toute fonctionnalité livrée est couverte par un
test unitaire **et** un test e2e. Trois fonctionnalités sont en production sans
leur test e2e — la sélection de match, le récap de match, l'onglet Classement.

La justification de la règle vaut d'être rappelée, parce qu'elle est
empirique : le bug du widget coach-search et celui des pickers de tiers
n'auraient été détectés par aucun test unitaire. Seul un navigateur voit qu'un
rendu HTMX/Alpine fonctionne réellement.

L'épic rattrape ces trois dettes et s'attaque au coût qui rend la suite pénible
à exécuter.

## Les cartes

| # | Intitulé | Apport |
|---|---|---|
| 96 | E2E step1 du rapport de match | 6 scénarios : création, pré-remplissage, cascades, reprise, cloisonnement |
| 150 | E2E de la page récap | 8 scénarios : accès par état, publication, double publication, dégradation gracieuse |
| 199 | E2E de l'onglet Classement | 6 scénarios : les 4 états du widget, le cumul sur deux matchs, le tri |
| 312 | Réduire le temps d'exécution de la suite | ~7 min 30 aujourd'hui, dont l'essentiel en fixtures |

## Ce qui commande l'ordre

**Aucune dépendance.** Les trois cartes de couverture dépendaient de code
(92, 93, 149, 198) qui est en `done/` depuis longtemps — elles sont
immédiatement faisables.

**312 gagne à passer en premier** si les trois autres doivent être écrites dans
la foulée : elle allège la fixture que 30 fichiers sur 38 paient déjà, et les
trois nouveaux tests l'hériteront. La piste est identifiée —
`create_full_competition` pilote encore **18 étapes de navigateur** là où les
équipes et les rapports de match sont déjà passés en HTTP direct, avec un gain
mesuré (2,4 s → quelques dizaines de ms par équipe).

Condition non négociable de 312 : **un test doit conserver le parcours réel au
clic.** C'est déjà le cas — `test_competition_full_lifecycle` et
`test_full_competition_creation_flow` l'exercent.

## Ce que l'épic ne couvre pas

- **Les tests unitaires manquants**, s'il y en a. Ces quatre cartes portent sur
  le niveau navigateur.
- **La couverture des fonctionnalités à venir** : chaque carte future porte son
  propre e2e dans sa checklist. L'épic est un rattrapage, pas un processus.
- **`tests/impact-map.toml`**, dont la mise à jour est une exigence de chaque
  carte qui ajoute un test, pas un chantier en soi.

## Terminé quand

Les trois fonctionnalités sont couvertes en navigateur, `make e2e` passe, et la
suite complète s'exécute assez vite pour qu'on la lance sans y penser.
