# Classement détaillé — Tests E2E

**Priorité : haute**
**Dépend de :** carte 223 (mise en évidence)
**Contexte :** `tests/e2e/test_detailed_standings.py` (nouveau), `tests/e2e/competition_lifecycle.py`
**Spec :** `docs/specs/ranking/tiebreakers/detailed-standings/07-integration.md`

## Objectif

Vérifier en navigateur que la mise en évidence atterrit sur la bonne cellule du bon
tableau. `tiebreak_outcomes` est couvert unitairement, mais **rien ne garantit que la
classe CSS arrive au bon endroit** — c'est le seul comportement de l'unité qu'aucun test
unitaire ne peut voir.

## Scénarios

| Scénario | Vérifie |
|---|---|
| **Critère décisif mis en évidence** | Deux équipes à égalité de points que seule la différence de TD sépare ⇒ la cellule correspondante porte la classe décisive sur les deux lignes, les autres colonnes ne la portent pas |
| **Ex æquo total** | Deux équipes égales sur tous les critères actifs ⇒ même rang affiché, aucune cellule décisive, toutes marquées égales |
| **Colonnes = critères actifs, dans l'ordre** | Deux critères décochés en phase 2 ⇒ le tableau n'affiche que les restants, numérotés 1..n |

## Deux pièges à ne pas rejouer

### 1. Les bonus sont cochés par défaut

Le formulaire de phase 2 coche les bonus offensif et défensif. Avec eux, un vainqueur 3-0
totalise un point de plus qu'un vainqueur 1-0 : **aucune équipe n'est jamais à égalité de
points**, le classement est décidé par les seuls points, et un test de départage passe au
vert sans rien départager.

C'est exactement ce qui s'est produit à la première écriture de la carte 219. Les
scénarios 1 et 2 exigent `with_default_bonuses=False`, paramètre déjà ajouté à
`create_full_competition` par cette carte-là.

### 2. Décocher des critères sans glisser-déposer

Le scénario 3 a besoin de désactiver des critères en phase 2. Le drag & drop HTML5 est la
partie fragile de `test_competition_rules_tiebreakers.py`, qui doit gérer un repli en
dispatchant les événements à la main. **Décocher suffit** : les critères restants gardent
l'ordre canonique, et l'ordre configuré est déjà couvert unitairement par
`to_tiebreak_order`.

`create_full_competition` ne sait pas décocher de critère — ajouter un paramètre optionnel
sur le modèle de `with_default_bonuses`, en préservant le comportement par défaut des
autres tests.

## Vérification par mutation — obligatoire

Un test E2E vert ne prouve rien tant qu'on ne l'a pas vu échouer. **Avant de commiter**,
neutraliser la mise en évidence côté serveur (renvoyer `Neutral` partout) et vérifier que
le scénario 1 **échoue**, puis révoquer la mutation et vérifier qu'il repasse.

Le serveur dev tourne sous `cargo watch` : attendre que le **processus ait effectivement
redémarré** avant de conclure — comparer l'horodatage de démarrage du processus à celui de
la mutation, la seule comparaison des dates de fichiers ayant déjà induit en erreur sur la
carte 219.

## Prérequis d'exécution

Serveur dev lancé **par l'utilisateur** (`make dev-demo`), `make seed_e2e` préalable.

`make reset_db` **n'est pas nécessaire** : chaque test crée sa propre compétition et sa
propre saison, or le classement est filtré par `season_id` — les lignes antérieures ne
peuvent pas fuiter. Ne jamais réinitialiser la base de l'utilisateur de sa propre
initiative.

## Checklist

- [ ] `tests/e2e/test_detailed_standings.py` créé
- [ ] Les 3 scénarios implémentés
- [ ] Bonus désactivés sur les scénarios exigeant une égalité de points
- [ ] Décochage de critères sans glisser-déposer
- [ ] Scénario 1 **vu échouer** sous mutation, puis vert après révocation
- [ ] La suite e2e complète reste verte
